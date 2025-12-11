use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

#[derive(Debug, Serialize, Deserialize)]
struct NominatimResponse {
    lat: String,
    lon: String,
    display_name: String,
}

#[derive(Clone)]
pub struct GeocodingResult {
    pub latitude: f64,
    pub longitude: f64,
    pub display_name: String,
}

static GEOCODING_CACHE: OnceLock<Mutex<HashMap<String, GeocodingResult>>> = OnceLock::new();

fn get_geocoding_cache() -> &'static Mutex<HashMap<String, GeocodingResult>> {
    GEOCODING_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub async fn geocode_address(address: &str) -> Result<GeocodingResult, String> {
    {
        let cache = get_geocoding_cache().lock().unwrap();
        if let Some(result) = cache.get(address) {
            return Ok(result.clone());
        }
    }

    let encoded_address = urlencoding::encode(address);
    let url = format!(
        "https://nominatim.openstreetmap.org/search?q={}&format=json&limit=1&addressdetails=1&countrycodes=au",
        encoded_address
    );

    let response = Request::get(&url)
        .header(
            "User-Agent",
            "NSW Drivers Test Nearest Date - teegee567/1.0",
        )
        .send()
        .await
        .map_err(|e| format!("Request error: {}", e))?;

    let results: Vec<NominatimResponse> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let result = results
        .first()
        .ok_or_else(|| "No results found".to_string())?;

    let geocoding_result = GeocodingResult {
        latitude: result.lat.parse().unwrap_or(0.0),
        longitude: result.lon.parse().unwrap_or(0.0),
        display_name: result.display_name.clone(),
    };

    {
        let mut cache = get_geocoding_cache().lock().unwrap();
        cache.insert(address.to_string(), geocoding_result.clone());
    }

    Ok(geocoding_result)
}
