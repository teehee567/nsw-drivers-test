use std::{cmp::Ordering, hash::{DefaultHasher, Hash, Hasher}};
use serde::{Deserialize, Serialize};

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
