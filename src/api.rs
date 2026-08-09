use radiobrowser::{ApiStation, blocking::RadioBrowserAPI};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use std::{fs, path::Path};

const CACHE_FILENAME: &str = "stations_cache.json";
const CACHE_MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24); // 24 hours

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CachedStation {
    pub stationuuid: String,
    pub name: String,
    pub url: String,
    pub tags: String,
    // Add any other fields you need for your UI here
}

// Load cached stations from disk
pub fn load_stations_from_cache() -> Option<Vec<CachedStation>> {
    let path = Path::new(CACHE_FILENAME);
    if !path.exists() {
        return None;
    }

    // Check file age
    if let Ok(metadata) = fs::metadata(path)
        && let Ok(modified) = metadata.modified()
        && let Ok(age) = SystemTime::now().duration_since(modified)
        && age > CACHE_MAX_AGE
    {
        return None; // Expired
    }

    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

// Map ApiStation instances to CachedStation and save them
fn save_stations_to_cache(stations: &[ApiStation]) {
    let serializable_stations: Vec<CachedStation> = stations
        .iter()
        .map(|s| CachedStation {
            stationuuid: s.stationuuid.clone(),
            name: s.name.clone(),
            url: s.url.clone(),
            tags: s.tags.clone(),
        })
        .collect();

    if let Ok(json_data) = serde_json::to_string(&serializable_stations) {
        let _ = fs::write(CACHE_FILENAME, json_data);
    }
}

pub fn get_stations_with_cache(api: &mut RadioBrowserAPI) -> Option<Vec<CachedStation>> {
    // 1. Try loading from cache first
    if let Some(stations) = load_stations_from_cache() {
        return Some(stations);
    }

    // 2. Fetch from network if cache is missing or expired
    let api_stations = api.get_stations().send().ok()?;

    // 3. Save to disk
    save_stations_to_cache(&api_stations);

    // 4. Return mapped local representation
    Some(
        api_stations
            .into_iter()
            .map(|s| CachedStation {
                stationuuid: s.stationuuid,
                name: s.name,
                url: s.url,
                tags: s.tags,
            })
            .collect(),
    )
}

pub fn filter_stations(search: &str, stations: &[CachedStation]) -> Vec<CachedStation> {
    let search_lower = search.to_lowercase();
    stations
        .iter()
        .filter(|station| {
            station.name.to_lowercase().contains(&search_lower)
                || station.tags.to_lowercase().contains(&search_lower)
            || station.url.to_lowercase().contains(&search_lower)
        })
        .cloned()
        .collect()
}

#[expect(dead_code)]
pub fn search_stations(api: &mut RadioBrowserAPI, search: &str) -> Vec<CachedStation> {
    let stations = get_stations_with_cache(api).unwrap_or(vec![CachedStation {
        stationuuid: String::new(),
        name: "No stations found".to_string(),
        url: String::new(),
        tags: String::new(),
    }]);
    filter_stations(search, &stations)
}

pub fn stations_to_string(stations: &[CachedStation]) -> String {
    stations
        .iter()
        .map(|s| format!("{} - {}", s.name, s.url))
        .collect::<Vec<String>>()
        .join("\n")
}
