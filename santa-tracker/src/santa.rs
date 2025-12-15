use chrono::{DateTime, Datelike, Local, TimeZone, Utc};
use rand::Rng;

#[derive(Debug, Clone)]
pub struct Location {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone_offset: i32, // Hours from UTC
}

pub struct SantaTracker {
    pub current_location: Location,
    pub next_location: Location,
    pub progress: f64,
    pub presents_delivered: u64,
    pub speed: f64, // km/h
    locations: Vec<Location>,
    current_index: usize,
}

impl SantaTracker {
    pub fn new() -> Self {
        let locations = Self::get_delivery_locations();
        let current = locations[0].clone();
        let next = locations[1].clone();

        Self {
            current_location: current,
            next_location: next,
            progress: 0.0,
            presents_delivered: 0,
            speed: 0.0,
            locations,
            current_index: 0,
        }
    }

    pub fn update(&mut self) {
        let now: DateTime<Utc> = Utc::now();
        let local = now.with_timezone(&Local);

        // Check if it's Christmas time
        let is_christmas_eve = local.month() == 12 && local.day() == 24;
        let is_christmas = local.month() == 12 && local.day() == 25;

        if !is_christmas_eve && !is_christmas {
            // Before Christmas: Santa is at North Pole preparing
            self.current_location = self.locations[0].clone();
            self.next_location = self.locations[1].clone();
            self.progress = 0.0;
            self.speed = 0.0;
            self.presents_delivered = 0;
            return;
        }

        // Calculate Santa's progression through the route
        let seconds_since_start = if is_christmas_eve {
            let christmas_eve_start = Local.with_ymd_and_hms(local.year(), 12, 24, 18, 0, 0).unwrap();
            (local - christmas_eve_start).num_seconds().max(0)
        } else {
            // Christmas Day
            let christmas_eve_start = Local.with_ymd_and_hms(local.year(), 12, 24, 18, 0, 0).unwrap();
            let christmas_end = Local.with_ymd_and_hms(local.year(), 12, 25, 23, 59, 59).unwrap();
            (local - christmas_eve_start).num_seconds().min((christmas_end - christmas_eve_start).num_seconds())
        };

        // Total delivery time: ~30 hours following timezones
        let total_seconds = 30.0 * 3600.0;
        let overall_progress = (seconds_since_start as f64 / total_seconds).min(1.0);

        // Calculate location index
        let location_index = (overall_progress * (self.locations.len() - 1) as f64).floor() as usize;
        let location_progress = (overall_progress * (self.locations.len() - 1) as f64).fract();

        if location_index < self.locations.len() - 1 {
            self.current_index = location_index;
            self.current_location = self.locations[location_index].clone();
            self.next_location = self.locations[location_index + 1].clone();
            self.progress = location_progress;
            
            // Calculate speed and presents
            let distance = Self::calculate_distance(
                self.current_location.latitude,
                self.current_location.longitude,
                self.next_location.latitude,
                self.next_location.longitude,
            );
            self.speed = distance * 10.0; // Fictional speed
            self.presents_delivered = (overall_progress * 7_800_000_000.0) as u64;
        } else {
            // Finished delivering
            self.current_location = self.locations.last().unwrap().clone();
            self.next_location = self.locations.last().unwrap().clone();
            self.progress = 1.0;
            self.speed = 0.0;
            self.presents_delivered = 7_800_000_000;
        }
    }

    fn calculate_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
        let r = 6371.0; // Earth radius in km
        let d_lat = (lat2 - lat1).to_radians();
        let d_lon = (lon2 - lon1).to_radians();
        let a = (d_lat / 2.0).sin().powi(2)
            + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        r * c
    }

    fn get_delivery_locations() -> Vec<Location> {
        vec![
            Location {
                name: "North Pole".to_string(),
                latitude: 90.0,
                longitude: 0.0,
                timezone_offset: 0,
            },
            Location {
                name: "Tokyo, Japan".to_string(),
                latitude: 35.6762,
                longitude: 139.6503,
                timezone_offset: 9,
            },
            Location {
                name: "Sydney, Australia".to_string(),
                latitude: -33.8688,
                longitude: 151.2093,
                timezone_offset: 11,
            },
            Location {
                name: "Beijing, China".to_string(),
                latitude: 39.9042,
                longitude: 116.4074,
                timezone_offset: 8,
            },
            Location {
                name: "Mumbai, India".to_string(),
                latitude: 19.0760,
                longitude: 72.8777,
                timezone_offset: 5,
            },
            Location {
                name: "Dubai, UAE".to_string(),
                latitude: 25.2048,
                longitude: 55.2708,
                timezone_offset: 4,
            },
            Location {
                name: "Moscow, Russia".to_string(),
                latitude: 55.7558,
                longitude: 37.6173,
                timezone_offset: 3,
            },
            Location {
                name: "Berlin, Germany".to_string(),
                latitude: 52.5200,
                longitude: 13.4050,
                timezone_offset: 1,
            },
            Location {
                name: "London, UK".to_string(),
                latitude: 51.5074,
                longitude: -0.1278,
                timezone_offset: 0,
            },
            Location {
                name: "New York, USA".to_string(),
                latitude: 40.7128,
                longitude: -74.0060,
                timezone_offset: -5,
            },
            Location {
                name: "Los Angeles, USA".to_string(),
                latitude: 34.0522,
                longitude: -118.2437,
                timezone_offset: -8,
            },
            Location {
                name: "Honolulu, Hawaii".to_string(),
                latitude: 21.3099,
                longitude: -157.8581,
                timezone_offset: -10,
            },
        ]
    }

    pub fn get_status_message(&self) -> String {
        let now = Local::now();
        if now.month() == 12 && (now.day() == 24 || now.day() == 25) {
            if self.progress >= 1.0 {
                "🎅 Santa has completed his journey! Merry Christmas! 🎄".to_string()
            } else {
                format!("🎅 Santa is traveling from {} to {}!", 
                    self.current_location.name, self.next_location.name)
            }
        } else {
            "🎅 Santa is preparing at the North Pole for Christmas Eve! 🎄".to_string()
        }
    }
}
