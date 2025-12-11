use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    hash::{DefaultHasher, Hash, Hasher},
};

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
        // self
        let self_parts: Vec<&str> = self.start_time.split(' ').collect();
        let self_date_parts: Vec<u32> = self_parts[0]
            .split('/')
            .map(|s| s.parse().unwrap())
            .collect();
        let self_time_parts: Vec<u32> = self_parts[1]
            .split(':')
            .map(|s| s.parse().unwrap())
            .collect();

        // other
        let other_parts: Vec<&str> = other.start_time.split(' ').collect();
        let other_date_parts: Vec<u32> = other_parts[0]
            .split('/')
            .map(|s| s.parse().unwrap())
            .collect();
        let other_time_parts: Vec<u32> = other_parts[1]
            .split(':')
            .map(|s| s.parse().unwrap())
            .collect();

        self_date_parts[2]
            .cmp(&other_date_parts[2])
            .then(self_date_parts[1].cmp(&other_date_parts[1]))
            .then(self_date_parts[0].cmp(&other_date_parts[0]))
            .then(self_time_parts[0].cmp(&other_time_parts[0]))
            .then(self_time_parts[1].cmp(&other_time_parts[1]))
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
