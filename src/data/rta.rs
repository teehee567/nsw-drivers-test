use pyo3::prelude::*;
use pyo3::types::PyModule;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::sync::Arc;

use super::shared_booking::LocationBookings;
use crate::settings::Settings;

const SCRAPER_PY: &CStr = c"scraper.py";

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

fn split_locations_into_groups(locations: Vec<String>, num_groups: usize) -> Vec<Vec<String>> {
    let mut shuffled = locations;
    {
        let mut rng = rand::rng();
        shuffled.shuffle(&mut rng);
    }

    let mut groups: Vec<Vec<String>> = vec![Vec::new(); num_groups];
    for (i, location) in shuffled.into_iter().enumerate() {
        groups[i % num_groups].push(location);
    }

    groups
}

fn scrape_single_group(
    locations: Vec<String>,
    headless: bool,
    username: String,
    password: String,
    have_booking: bool,
    timeout_ms: u64,
    polling_ms: u64,
    proxy: String,
) -> Result<HashMap<String, LocationBookings>, ScrapeError> {
    Python::with_gil(|py| {
        let _ = pyo3_log::try_init();

        let scraper_module = PyModule::from_code(py, SCRAPER_PY, c"scraper.py", c"scraper")?;

        let scrape_fn = scraper_module.getattr("scrape_rta_timeslots")?;

        let result = scrape_fn.call1((
            locations,
            headless,
            username,
            password,
            have_booking,
            timeout_ms,
            polling_ms,
            proxy,
        ))?;

        let bookings: HashMap<String, LocationBookings> = result.extract()?;

        Ok(bookings)
    })
}

pub async fn scrape_rta_timeslots(
    locations: Vec<String>,
    settings: &Settings,
) -> Result<HashMap<String, LocationBookings>, Box<dyn std::error::Error + Send + Sync>> {
    let parallel_browsers = settings.parallel_browsers;
    let proxies = settings.proxies.clone();
    let num_rotations = (proxies.len() + parallel_browsers - 1) / parallel_browsers;

    // rotation sets
    let proxy_sets: Vec<Vec<String>> = proxies
        .chunks(parallel_browsers)
        .map(|chunk| chunk.to_vec())
        .collect();

    // randomize location groups
    let location_groups = split_locations_into_groups(locations.clone(), parallel_browsers);

    let all_bookings = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    for (rotation_idx, proxy_set) in proxy_sets.iter().enumerate() {
        log::info!(
            "Starting rotation {}/{} with {} proxies",
            rotation_idx + 1,
            num_rotations,
            proxy_set.len()
        );

        let mut handles = Vec::new();

        for (group_idx, (location_group, proxy)) in
            location_groups.iter().zip(proxy_set.iter()).enumerate()
        {
            let already_scraped = all_bookings.lock().await;
            let remaining_locations: Vec<String> = location_group
                .iter()
                .filter(|loc| !already_scraped.contains_key(*loc))
                .cloned()
                .collect();
            drop(already_scraped);

            if remaining_locations.is_empty() {
                continue;
            }

            let headless = settings.headless;
            let username = settings.username.clone();
            let password = settings.password.clone();
            let have_booking = settings.have_booking;
            let timeout_ms = settings.selenium_element_timout;
            let polling_ms = settings.selenium_element_polling;
            let proxy = proxy.clone();
            let all_bookings = Arc::clone(&all_bookings);

            log::info!(
                "Group {} starting with proxy {} for {} locations",
                group_idx,
                proxy,
                remaining_locations.len()
            );

            // start
            let handle = tokio::task::spawn_blocking(move || {
                let result = scrape_single_group(
                    remaining_locations,
                    headless,
                    username,
                    password,
                    have_booking,
                    timeout_ms,
                    polling_ms,
                    proxy.clone(),
                );
                (group_idx, proxy, result)
            });

            handles.push(handle);
        }

        // wait for finish
        for handle in handles {
            match handle.await {
                Ok((group_idx, proxy, result)) => match result {
                    Ok(bookings) => {
                        let count = bookings.len();
                        all_bookings.lock().await.extend(bookings);
                        log::info!(
                            "Group {} with proxy {} completed. Got {} locations.",
                            group_idx,
                            proxy,
                            count
                        );
                    }
                    Err(e) => {
                        log::error!("Group {} with proxy {} failed: {}", group_idx, proxy, e);
                    }
                },
                Err(e) => {
                    log::error!("Task join error: {}", e);
                }
            }
        }

        let scraped_count = all_bookings.lock().await.len();
        let total_count = locations.len();
        log::info!(
            "After rotation {}: {}/{} locations scraped.",
            rotation_idx + 1,
            scraped_count,
            total_count
        );

        if scraped_count >= total_count {
            break;
        }
    }

    let final_bookings = Arc::try_unwrap(all_bookings)
        .map_err(|_| "Failed to unwrap Arc")?
        .into_inner();

    Ok(final_bookings)
}
