use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::collections::HashMap;
use std::ffi::CString;

use super::shared_booking::LocationBookings;
use crate::settings::Settings;

const SCRAPER_PY: &str = include_str!("scraper.py");

#[derive(Debug)]
pub struct ScrapeError(String);

impl std::fmt::Display for ScrapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ScrapeError: {}", self.0)
    }
}

impl std::error::Error for ScrapeError {}

impl From<PyErr> for ScrapeError {
    fn from(err: PyErr) -> Self {
        ScrapeError(err.to_string())
    }
}

impl From<tokio::task::JoinError> for ScrapeError {
    fn from(err: tokio::task::JoinError) -> Self {
        ScrapeError(err.to_string())
    }
}

fn scrape_single_group(
    locations: Vec<String>,
    headless: bool,
    username: String,
    password: String,
    have_booking: bool,
    timeout_ms: u64,
    polling_ms: u64,
    proxies: Vec<String>,
    parallel_browsers: usize,
) -> Result<HashMap<String, LocationBookings>, ScrapeError> {
    Python::with_gil(|py| {
        let _ = pyo3_log::try_init();

        let code = CString::new(SCRAPER_PY).expect("Failed to create CString from scraper code");
        let scraper_module = PyModule::from_code(py, &code, c"scraper.py", c"scraper")?;

        let scrape_fn = scraper_module.getattr("scrape_rta_timeslots_parallel")?;

        let result = scrape_fn.call1((
            locations,
            headless,
            username,
            password,
            have_booking,
            timeout_ms,
            polling_ms,
            proxies,
            parallel_browsers,
        ))?;

        let bookings: HashMap<String, LocationBookings> = result.extract()?;

        Ok(bookings)
    })
}

pub async fn scrape_rta_timeslots(
    locations: Vec<String>,
    settings: &Settings,
) -> Result<HashMap<String, LocationBookings>, Box<dyn std::error::Error + Send + Sync>> {
    let proxies = settings.proxies.clone();
    let parallel_browsers = settings.parallel_browsers;
    
    log::info!(
        "Starting scrape with {} parallel browsers for {} locations",
        parallel_browsers,
        locations.len()
    );
    
    let headless = settings.headless;
    let username = settings.username.clone();
    let password = settings.password.clone();
    let have_booking = settings.have_booking;
    let timeout_ms = settings.selenium_element_timout;
    let polling_ms = settings.selenium_element_polling;
    
    let result = tokio::task::spawn_blocking(move || {
        scrape_single_group(
            locations,
            headless,
            username,
            password,
            have_booking,
            timeout_ms,
            polling_ms,
            proxies,
            parallel_browsers,
        )
    })
    .await??;
    
    log::info!("Scraping complete: {} locations scraped.", result.len());

    Ok(result)
}
