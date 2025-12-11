use std::{
    collections::HashMap,
    sync::{Arc, OnceLock, RwLock},
};

use serde::{Deserialize, Serialize};

static LOCATION_STORE: OnceLock<Arc<RwLock<LocationStore>>> = OnceLock::new();

fn get_location_store() -> &'static Arc<RwLock<LocationStore>> {
    LOCATION_STORE.get_or_init(|| Arc::new(RwLock::new(LocationStore::new())))
}

fn initialize_location_store() {
    fn parse_locations() -> Vec<Location> {
        let json_data = include_str!("../../data/centres.json");
        serde_json::from_str(json_data).unwrap_or_else(|e| {
            log::error!("Failed to parse locations: {}", e);
            Vec::new()
        })
    }

    let store = get_location_store();
    if let Ok(mut store) = store.try_write() {
        if store.get_all_locations().is_empty() {
            store.load_locations(parse_locations());
        }
    }
}

struct LocationStore {
    locations: Vec<Location>,
    location_by_id: HashMap<u32, usize>,
}

impl LocationStore {
    fn new() -> Self {
        Self {
            locations: Vec::new(),
            location_by_id: HashMap::new(),
        }
    }

    fn load_locations(&mut self, locations: Vec<Location>) {
        self.location_by_id.clear();
        self.location_by_id.reserve(locations.len());

        for (idx, location) in locations.iter().enumerate() {
            self.location_by_id.insert(location.id, idx);
        }

        self.locations = locations;
    }

    #[inline]
    fn get_all_locations(&self) -> &[Location] {
        &self.locations
    }

    #[inline]
    fn get_by_id(&self, id: u32) -> Option<&Location> {
        self.location_by_id
            .get(&id)
            .map(|&idx| &self.locations[idx])
    }

    fn get_locations_by_distance(&self, latitude: f64, longitude: f64) -> Vec<(Location, f64)> {
        let mut locations_with_distance = Vec::with_capacity(self.locations.len());

        for loc in &self.locations {
            let distance = loc.distance_from(latitude, longitude);
            locations_with_distance.push((loc.clone(), distance));
        }

        locations_with_distance
            .sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        locations_with_distance
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Location {
    pub id: u32,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub passes: i32,
    pub failures: i32,
    pub pass_rate: f64,
}

impl Location {
    pub fn distance_from(&self, lat: f64, lng: f64) -> f64 {
        const EARTH_RADIUS: f64 = 6371.0;

        let lat1_rad = self.latitude.to_radians();
        let lat2_rad = lat.to_radians();
        let delta_lat = (lat - self.latitude).to_radians();
        let delta_lng = (lng - self.longitude).to_radians();

        if delta_lat.abs() < 0.001 && delta_lng.abs() < 0.001 {
            let x = delta_lng * lat1_rad.cos();
            let y = delta_lat;
            return EARTH_RADIUS * (x * x + y * y).sqrt();
        }

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lng / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        EARTH_RADIUS * c
    }
}

#[derive(Clone)]
pub struct LocationManager;

impl LocationManager {
    pub fn new() -> Self {
        initialize_location_store();
        Self
    }

    pub fn get_by_distance(&self, lat: f64, lng: f64) -> Vec<(Location, f64)> {
        match get_location_store().read() {
            Ok(store) => store.get_locations_by_distance(lat, lng),
            Err(_) => Vec::new(),
        }
    }

    pub fn get_all(&self) -> Vec<Location> {
        match get_location_store().read() {
            Ok(store) => store.get_all_locations().to_vec(),
            Err(_) => Vec::new(),
        }
    }

    pub fn get_by_id(&self, id: u32) -> Option<Location> {
        get_location_store().read().ok()?.get_by_id(id).cloned()
    }
}
