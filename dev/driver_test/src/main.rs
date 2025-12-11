use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use thirtyfour::components::SelectElement;
use thirtyfour::{By, DesiredCapabilities, WebDriver};
use thirtyfour::prelude::*;
use rand::Rng;

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub headless: bool,
    pub username: String,
    pub password: String,
    pub have_booking: bool,
    pub selenium_driver_url: String,
    pub selenium_element_timout: u64,
    pub selenium_element_polling: u64,
}

impl Settings {
    pub fn from_yaml<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        dotenv::from_path("../../.env").ok();
        
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        let mut settings: Settings = serde_yaml::from_str(&contents)?;
        
        settings.username = parse_env_var(&settings.username)?;
        settings.password = parse_env_var(&settings.password)?;
        
        Ok(settings)
    }
}

fn parse_env_var(value: &str) -> Result<String, Box<dyn std::error::Error>> {
    if value.starts_with("${") && value.ends_with("}") {
        let env_name = &value[2..value.len() - 1];
        match env::var(env_name) {
            Ok(val) => Ok(val),
            Err(_) => Err(format!("Environment variable '{}' not found", env_name).into()),
        }
    } else {
        Ok(value.to_string())
    }
}

use std::{cmp::Ordering, hash::{DefaultHasher, Hash, Hasher}};

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct TimeSlot {
    pub availability: bool,
    pub slot_number: Option<u32>,
    #[serde(rename = "startTime")]
    pub start_time: String,
}

impl PartialEq for TimeSlot {
    fn eq(&self, other: &Self) -> bool {
        self.start_time == other.start_time
    }
}

impl Eq for TimeSlot {}

impl PartialOrd for TimeSlot {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TimeSlot {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start_time.cmp(&other.start_time)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct LocationBookings {
    pub location: String,
    pub slots: Vec<TimeSlot>,
    pub next_available_date: Option<String>,
}

impl LocationBookings {
    pub fn calculate_hash(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish().to_string()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Hash)]
pub struct BookingData {
    pub results: Vec<LocationBookings>,
    pub last_updated: Option<String>,
}

impl BookingData {
    pub fn calculate_hash(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish().to_string()
    }
}


async fn random_sleep(min_millis: u64, max_millis: u64) {
    if min_millis >= max_millis {
        tokio::time::sleep(Duration::from_millis(min_millis)).await;
        return;
    }
    let duration = rand::thread_rng().gen_range(min_millis..max_millis);
    tokio::time::sleep(Duration::from_millis(duration)).await;
}

async fn type_like_human(element: &WebElement, text: &str, min_delay_ms: u64, max_delay_ms: u64) -> WebDriverResult<()> {
    for char in text.chars() {
        element.send_keys(char.to_string()).await?;
        random_sleep(min_delay_ms, max_delay_ms).await;
    }
    Ok(())
}

pub async fn scrape_rta_timeslots(
    locations: Vec<String>,
    settings: &Settings
) -> WebDriverResult<HashMap<String, LocationBookings>> {

    let mut location_bookings: HashMap<String, LocationBookings> = HashMap::new();

    let mut caps = DesiredCapabilities::chrome();
    if settings.headless {
        caps.add_arg("--headless=new")?;
    }

    caps.add_arg("--no-sandbox")?;
    caps.add_arg("--disable-dev-shm-usage")?;
    caps.add_arg("--disable-gpu")?;
    caps.add_arg("--window-size=1920,1080")?;
    caps.add_arg("--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36");

    let driver = WebDriver::new(settings.selenium_driver_url.clone(), caps).await?;

    driver.execute(r#"
        Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
        // Minimal spoofing of window.chrome, might need adjustment
        window.chrome = window.chrome || {};
        window.chrome.runtime = window.chrome.runtime || {};
        // Attempt to remove cdc_ properties (might not exist)
        try {
            let key = Object.keys(window).find(key => key.startsWith('cdc_'));
            if (key) { delete window[key]; }
            let docKey = Object.keys(document).find(key => key.startsWith('cdc_'));
            if (docKey) { delete document[docKey]; }
        } catch (e) { console.debug('Error removing cdc keys:', e); }
    "#, Vec::new()).await?;


    let timeout = Duration::from_millis(settings.selenium_element_timout);
    let polling = Duration::from_millis(settings.selenium_element_polling);

    driver.goto("https://www.myrta.com/wps/portal/extvp/myrta/login/").await?;
    random_sleep(1000, 2000).await;

    let username_input = driver.query(By::Id("widget_cardNumber")).first().await?;
    username_input.wait_until().wait(timeout, polling).displayed().await?;
    random_sleep(200, 500).await;
    type_like_human(&username_input, &settings.username, 60, 180).await?;
    random_sleep(300, 700).await;

    let password_input = driver.query(By::Id("widget_password")).first().await?;
    password_input.wait_until().wait(timeout, polling).displayed().await?;
    random_sleep(200, 500).await;
    type_like_human(&password_input, &settings.password, 60, 180).await?;
    random_sleep(400, 800).await;

    let next_button = driver.query(By::Id("nextButton")).first().await?;
    next_button.wait_until().wait(timeout, polling).displayed().await?;
    // next_button.wait_until().wait(timeout, polling).has_attribute("aria-disabled", "false").await?; // Alternative if clickable() doesn't work
    random_sleep(250, 600).await;
    next_button.click().await?;

    random_sleep(2000, 4000).await;

    if settings.have_booking {
        let manage_booking = driver.query(By::XPath("//*[text()=\"Manage booking\"]")).first().await?;
        manage_booking.wait_until().wait(timeout, polling).displayed().await?;
        random_sleep(200, 500).await;
        manage_booking.click().await?;
        random_sleep(1500, 2500).await;

        let change_location = driver.query(By::Id("changeLocationButton")).first().await?;
        change_location.wait_until().wait(timeout, polling).displayed().await?;
        random_sleep(200, 500).await;
        change_location.click().await?;
        random_sleep(1000, 2000).await;

    } else {
         let book_test = driver.query(By::XPath("//*[text()=\"Book test\"]")).first().await?;
         book_test.wait_until().wait(timeout, polling).displayed().await?;
         random_sleep(200, 500).await;
         book_test.click().await?;
         random_sleep(1500, 2500).await;

         let car_option = driver.query(By::Id("CAR")).first().await?;
         car_option.wait_until().wait(timeout, polling).displayed().await?;
         random_sleep(200, 500).await;
         car_option.click().await?;
         random_sleep(500, 1000).await;

         let test_item = driver.query(By::XPath("//fieldset[@id='DC']/span[contains(@class, 'rms_testItemResult')]")).first().await?;
         test_item.wait_until().wait(timeout, polling).displayed().await?;
         random_sleep(200, 500).await;
         test_item.click().await?;
         random_sleep(500, 1000).await;

         let next_button = driver.query(By::Id("nextButton")).first().await?;
         next_button.wait_until().wait(timeout, polling).displayed().await?;
         random_sleep(200, 500).await;
         next_button.click().await?;
         random_sleep(1500, 2500).await;

         let check_terms = driver.query(By::Id("checkTerms")).first().await?;
         check_terms.wait_until().wait(timeout, polling).displayed().await?;
         random_sleep(100, 300).await;
         check_terms.click().await?;
         random_sleep(500, 1000).await;

         let next_button_terms = driver.query(By::Id("nextButton")).first().await?;
         next_button_terms.wait_until().wait(timeout, polling).displayed().await?;
         random_sleep(200, 500).await;
         next_button_terms.click().await?;
         random_sleep(1000, 2000).await;
    }

    for location in locations {
        // println!("INFO: Processing location: {}", location);
        let process_result: WebDriverResult<LocationBookings> = async {

            random_sleep(1000, 2000).await;

            let location_select_dropdown = driver.query(By::Id("rms_batLocLocSel")).first().await?;
            location_select_dropdown.wait_until().wait(timeout, polling).displayed().await?;
            random_sleep(200, 400).await;
            location_select_dropdown.click().await?;
            random_sleep(500, 1000).await;

            let select_element_query = driver.query(By::Id("rms_batLocationSelect2"));
            let select_element = select_element_query.wait(timeout, polling).first().await?;
            select_element.wait_until().wait(timeout, polling).displayed().await?;
            let select_box = SelectElement::new(&select_element).await?;

            if let Err(e) = select_box.select_by_value(&location).await {
                 eprintln!("ERROR: Failed to select location '{}' in dropdown: {}. Ensure the value is correct.", location, e);
                 return Err(e);
            }

            // println!("INFO: Selected location: {}", location);
            random_sleep(2500, 4000).await;

            let next_button_loc = driver.query(By::Id("nextButton")).first().await?;
            next_button_loc.wait_until().wait(timeout, polling).displayed().await?;
            random_sleep(200, 500).await;
            next_button_loc.click().await?;

            random_sleep(1000, 2000).await;

            match driver.query(By::Id("getEarliestTime")).first().await {
                Ok(element) => {
                     if element.is_clickable().await.unwrap_or(false) {
                         random_sleep(200, 400).await;
                         if let Err(e) = element.click().await {
                            eprintln!("WARN: Failed to click 'Get Earliest Time' button for {}: {}. Proceeding anyway.", location, e);
                         } else {
                             random_sleep(2500, 4500).await;
                         }
                     } else {
                         println!("INFO: 'Get Earliest Time' button found but not clickable (visible/enabled).");
                         random_sleep(500, 1000).await;
                     }
                },
                Err(_) => {
                    println!("INFO: 'Get Earliest Time' button not found for {}. Proceeding.", location);
                    random_sleep(500, 1000).await;
                },
            }

            random_sleep(1000, 2500).await;

            let timeslots = driver.execute("return timeslots", vec![]).await?;

            let next_available_date = timeslots.json()
                .get("ajaxresult")
                .and_then(|ajax| ajax.get("slots"))
                .and_then(|slots| slots.get("nextAvailableDate"))
                .and_then(|date| date.as_str())
                .map(|s| s.to_string());
                
            let slots: Vec<TimeSlot> = timeslots.json()
                .get("ajaxresult")
                .and_then(|ajax| ajax.get("slots"))
                .and_then(|slots| slots.get("listTimeSlot"))
                .and_then(|list| serde_json::from_value(list.clone()).ok())
                .unwrap_or_else(Vec::new);


            println!("INFO: Parsed {} slots for {}. Next available: {:?}", slots.len(), location, next_available_date);

            let location_result = LocationBookings {
                location: location.to_string(),
                slots,
                next_available_date,
            };

            random_sleep(800, 1500).await;

            let another_location_link = driver.query(By::Id("anotherLocationLink")).first().await?;
            another_location_link.wait_until().wait(timeout, polling).displayed().await?;
            random_sleep(200, 500).await;
            another_location_link.click().await?;

            Ok(location_result)

        }.await;

        match process_result {
            Ok(booking_data) => {
                location_bookings.insert(location.clone(), booking_data);
            }
            Err(e) => {
                 eprintln!("ERROR: Failed processing location {}: {}", location, e);
                 match driver.query(By::Id("anotherLocationLink")).first().await {
                     Ok(link) => {
                          if link.is_displayed().await.unwrap_or(false) {
                              eprintln!("INFO: Attempting recovery click on 'Another Location'.");
                              if let Err(click_err) = link.click().await {
                                  eprintln!("WARN: Recovery click failed: {}", click_err);
                              } else {
                                  println!("INFO: Recovery click succeeded.");
                              }
                          } else {
                              eprintln!("WARN: Recovery link found but not displayed.");
                          }
                     }
                     Err(_) => {
                         eprintln!("WARN: Recovery link ('anotherLocationLink') not found. State unclear.");
                     }
                 }
                 random_sleep(2000, 3000).await;
                 continue;
            }
        }
         random_sleep(1500, 3000).await;
    }

    println!("INFO: Finished scraping all locations. Quitting driver.");
    driver.quit().await?;

    Ok(location_bookings)
}


#[tokio::main]
async fn main() {
    let mut settings = Settings::from_yaml("../../settings.yaml").unwrap();
    settings.headless = false;
    let env_content = include_str!("../../../.env");

    let (username, password) = env_content
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim(), value.trim()))
        .fold((None, None), |(mut u, mut p), (k, v)| {
            if k == "USERNAME" { u = Some(v.to_string()); }
            if k == "PASSWORD" { p = Some(v.to_string()); }
            (u, p)
        });

    settings.username = username.unwrap();
    settings.password = password.unwrap();
    
    let locations = vec!["141", "Yass", "Finley", "Hornsby", "Armidale", "Auburn", "Ballina"];
    let locations = locations.into_iter().map(|a| a.to_string()).collect();
    
    dbg!(scrape_rta_timeslots(locations, &settings).await);

}
