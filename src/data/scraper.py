
import logging
import random
import time
from typing import Optional

import undetected_chromedriver as uc
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait, Select
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.chrome.service import Service as ChromeService
from webdriver_manager.chrome import ChromeDriverManager
from webdriver_manager.core.os_manager import ChromeType

logger = logging.getLogger("rta_scraper")


def random_sleep(min_ms: int, max_ms: int):
    duration = random.randint(min_ms, max_ms) / 1000.0
    time.sleep(duration)


def type_like_human(element, text: str, min_delay_ms: int = 30, max_delay_ms: int = 90):
    for char in text:
        element.send_keys(char)
        random_sleep(min_delay_ms, max_delay_ms)


def scrape_rta_timeslots(
    locations: list,
    headless: bool,
    username: str,
    password: str,
    have_booking: bool,
    timeout_ms: int,
    polling_ms: int,
    proxy: str,
) -> dict:
    logger.info(f"Starting browser with proxy {proxy} for {len(locations)} locations")
    location_bookings = {}
    timeout_sec = timeout_ms / 1000.0
    
    options = uc.ChromeOptions()
    if headless:
        options.add_argument("--headless=new")
    options.add_argument(f"--proxy-server={proxy}")
    options.add_argument("--no-sandbox")
    options.add_argument("--disable-dev-shm-usage")
    options.add_argument("--disable-gpu")
    options.add_argument("--window-size=1920,1080")
    
    import os
    import platform
    from webdriver_manager.chrome import ChromeDriverManager
    
    chrome_install = ChromeDriverManager().install()
    
    # Check for CHROME_PATH environment variable first (used in Docker)
    chrome_binary = os.environ.get("CHROME_PATH")
    
    if not chrome_binary or not os.path.exists(chrome_binary):
        # Fall back to searching webdriver-manager's Chrome installation
        chrome_dir = os.path.dirname(os.path.dirname(chrome_install))
        chrome_binary = None
        
        # Determine the Chrome binary name based on platform
        if platform.system() == "Windows":
            chrome_exe_name = "chrome.exe"
        else:
            chrome_exe_name = "chrome"
        
        for root, dirs, files in os.walk(chrome_dir):
            if chrome_exe_name in files:
                chrome_binary = os.path.join(root, chrome_exe_name)
                break
    
    driver = uc.Chrome(
        options=options,
        browser_executable_path=chrome_binary,
        driver_executable_path=chrome_install,
    )
    
    try:
        wait = WebDriverWait(driver, timeout_sec, poll_frequency=polling_ms / 1000.0)
        
        driver.get("https://www.myrta.com/wps/portal/extvp/myrta/login/")
        random_sleep(500, 1000)

        username_input = wait.until(EC.presence_of_element_located((By.ID, "widget_cardNumber")))
        random_sleep(100, 250)
        type_like_human(username_input, username)
        random_sleep(150, 350)

        password_input = wait.until(EC.presence_of_element_located((By.ID, "widget_password")))
        random_sleep(100, 250)
        type_like_human(password_input, password)
        random_sleep(200, 400)

        next_button = wait.until(EC.element_to_be_clickable((By.ID, "nextButton")))
        random_sleep(125, 300)
        next_button.click()
        random_sleep(1000, 2000)
        
        if have_booking:
            manage_booking = wait.until(
                EC.element_to_be_clickable((By.XPATH, "//*[text()=\"Manage booking\"]"))
            )
            random_sleep(100, 250)
            manage_booking.click()
            random_sleep(750, 1250)
            
            change_location = wait.until(EC.element_to_be_clickable((By.ID, "changeLocationButton")))
            random_sleep(100, 250)
            change_location.click()
            random_sleep(500, 1000)
        else:
            book_test = wait.until(EC.element_to_be_clickable((By.XPATH, "//*[text()='Book test']")))
            random_sleep(100, 250)
            book_test.click()
            random_sleep(750, 1250)
            
            car_option = wait.until(EC.element_to_be_clickable((By.ID, "CAR")))
            random_sleep(100, 250)
            car_option.click()
            random_sleep(250, 500)
            
            test_item = wait.until(EC.element_to_be_clickable(
                (By.XPATH, "//fieldset[@id='DC']/span[contains(@class, 'rms_testItemResult')]")
            ))
            random_sleep(100, 250)
            test_item.click()
            random_sleep(250, 500)
            
            next_btn = wait.until(EC.element_to_be_clickable((By.ID, "nextButton")))
            random_sleep(100, 250)
            next_btn.click()
            random_sleep(750, 1250)
            
            check_terms = wait.until(EC.element_to_be_clickable((By.ID, "checkTerms")))
            random_sleep(50, 150)
            check_terms.click()
            random_sleep(250, 500)
            
            next_btn_terms = wait.until(EC.element_to_be_clickable((By.ID, "nextButton")))
            random_sleep(100, 250)
            next_btn_terms.click()
            random_sleep(500, 1000)

        for location in locations:
            try:
                random_sleep(500, 1000)

                location_dropdown = wait.until(EC.element_to_be_clickable((By.ID, "rms_batLocLocSel")))
                random_sleep(100, 200)
                location_dropdown.click()
                random_sleep(250, 500)

                select_element = wait.until(EC.presence_of_element_located((By.ID, "rms_batLocationSelect2")))
                select = Select(select_element)
                select.select_by_value(location)
                
                random_sleep(1250, 2000)

                next_btn_loc = wait.until(EC.element_to_be_clickable((By.ID, "nextButton")))
                random_sleep(100, 250)
                next_btn_loc.click()
                random_sleep(500, 1000)

                try:
                    earliest_btn = driver.find_element(By.ID, "getEarliestTime")
                    if earliest_btn.is_displayed() and earliest_btn.is_enabled():
                        random_sleep(100, 200)
                        earliest_btn.click()
                        random_sleep(1250, 2250)
                except:
                    random_sleep(250, 500)
                
                random_sleep(500, 1250)

                timeslots = driver.execute_script("return timeslots")
                
                next_available_date = None
                slots = []
                
                if timeslots and "ajaxresult" in timeslots:
                    ajax = timeslots["ajaxresult"]
                    if "slots" in ajax:
                        slots_data = ajax["slots"]
                        next_available_date = slots_data.get("nextAvailableDate")
                        list_timeslots = slots_data.get("listTimeSlot", [])
                        
                        for slot in list_timeslots:
                            slots.append({
                                "availability": slot.get("availability", False),
                                "slot_number": slot.get("slotNumber"),
                                "startTime": slot.get("startTime", ""),
                            })
                
                logger.info(f"Parsed {len(slots)} slots for {location}. Next available: {next_available_date}")
                
                location_bookings[location] = {
                    "location": location,
                    "slots": slots,
                    "next_available_date": next_available_date,
                }
                
                random_sleep(400, 750)

                another_location = wait.until(EC.element_to_be_clickable((By.ID, "anotherLocationLink")))
                random_sleep(100, 250)
                another_location.click()
                
            except Exception as e:
                logger.error(f"Failed processing location {location}: {e}")

                try:
                    another_link = driver.find_element(By.ID, "anotherLocationLink")
                    if another_link.is_displayed():
                        another_link.click()
                        logger.info("Recovery click succeeded.")
                except:
                    logger.warning("Recovery failed.")
                random_sleep(1000, 1500)
                continue
            
            random_sleep(750, 1500)
        
        logger.info(f"Finished scraping {len(location_bookings)} locations with proxy {proxy}.")
        
    finally:
        driver.quit()
    
    return location_bookings
