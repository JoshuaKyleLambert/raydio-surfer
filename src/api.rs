use radiobrowser::{ApiStation, StationOrder, blocking::RadioBrowserAPI};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};
use std::{fs, path::Path};

const CACHE_FILENAME: &str = "stations_cache.json";
const CACHE_MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24); // 24 hours
const MAX_STATIONS_LIMIT: &str = "5000";

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct CachedStation {
    pub stationuuid: String,
    pub name: String,
    pub url: String,
    pub tags: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub countrycode: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub language: String,
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
            country: s.country.clone(),
            countrycode: s.countrycode.clone(),
            state: s.state.clone(),
            language: s.language.clone(),
        })
        .collect();

    if let Ok(json_data) = serde_json::to_string(&serializable_stations) {
        let _ = fs::write(CACHE_FILENAME, json_data);
    }
}

pub fn get_stations_with_cache(api: &mut RadioBrowserAPI) -> Vec<CachedStation> {
    // 1. Try loading from cache first
    if let Some(stations) = load_stations_from_cache() {
        return stations;
    }

    // 2. Fetch from network if cache is missing or expired
    let api_stations = api
        .get_stations()
        .limit(MAX_STATIONS_LIMIT)
        .hidebroken(true)
        .order(StationOrder::Clickcount)
        .reverse(true)
        .send()
        .ok();

    if api_stations.is_none() {
        return vec![CachedStation {
            stationuuid: String::new(),
            name: "Failed To Load Stations".to_string(),
            url: String::new(),
            tags: String::new(),
            country: String::new(),
            countrycode: String::new(),
            state: String::new(),
            language: String::new(),
        }];
    }
    // 3. Save to disk
    save_stations_to_cache(api_stations.as_ref().unwrap());

    // 4. Return mapped local representation

    api_stations
        .unwrap()
        .into_iter()
        .map(|s| CachedStation {
            stationuuid: s.stationuuid,
            name: s.name,
            url: s.url,
            tags: s.tags,
            country: s.country,
            countrycode: s.countrycode,
            state: s.state,
            language: s.language,
        })
        .collect()
}

pub fn filter_stations(search: &str, stations: &[CachedStation]) -> Vec<CachedStation> {
    let search_lower = search.trim().to_lowercase();
    if search_lower.is_empty() {
        return stations.to_vec();
    }
    stations
        .iter()
        .filter(|station| {
            station.name.to_lowercase().contains(&search_lower)
                || station.tags.to_lowercase().contains(&search_lower)
                || station.country.to_lowercase().contains(&search_lower)
                || station.countrycode.to_lowercase().contains(&search_lower)
                || station.state.to_lowercase().contains(&search_lower)
                || station.language.to_lowercase().contains(&search_lower)
                || station.url.to_lowercase().contains(&search_lower)
        })
        .cloned()
        .collect()
}

#[expect(dead_code)]
pub fn search_stations(api: &mut RadioBrowserAPI, search: &str) -> Vec<CachedStation> {
    let stations = get_stations_with_cache(api);
    filter_stations(search, &stations)
}
fn truncate(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}
#[expect(dead_code)]
pub fn stations_to_string(stations: &[CachedStation]) -> String {
    stations
        .iter()
        .map(|s| truncate(&s.name, 24))
        .collect::<Vec<String>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_stations() {
        let stations = vec![
            CachedStation {
                stationuuid: "1".into(),
                name: "Vaporwave FM".into(),
                url: "http://stream1".into(),
                tags: "electronic,synth,vaporwave".into(),
                country: "Japan".into(),
                countrycode: "JP".into(),
                state: "Tokyo".into(),
                language: "japanese".into(),
            },
            CachedStation {
                stationuuid: "2".into(),
                name: "Jazz Lounge".into(),
                url: "http://stream2".into(),
                tags: "jazz,relax".into(),
                country: "United States".into(),
                countrycode: "US".into(),
                state: "California".into(),
                language: "english".into(),
            },
            CachedStation {
                stationuuid: "3".into(),
                name: "Paris Chanson".into(),
                url: "http://stream3".into(),
                tags: "chanson,french,retro".into(),
                country: "France".into(),
                countrycode: "FR".into(),
                state: "Ile-de-France".into(),
                language: "french".into(),
            },
        ];

        // Search by name
        let filtered_name = filter_stations("Vaporwave", &stations);
        assert_eq!(filtered_name.len(), 1);
        assert_eq!(filtered_name[0].name, "Vaporwave FM");

        // Search by tag / genre
        let filtered_tag = filter_stations("relax", &stations);
        assert_eq!(filtered_tag.len(), 1);
        assert_eq!(filtered_tag[0].name, "Jazz Lounge");

        let filtered_genre = filter_stations("synth", &stations);
        assert_eq!(filtered_genre.len(), 1);
        assert_eq!(filtered_genre[0].name, "Vaporwave FM");

        // Search by country
        let filtered_country = filter_stations("France", &stations);
        assert_eq!(filtered_country.len(), 1);
        assert_eq!(filtered_country[0].name, "Paris Chanson");

        // Search by countrycode
        let filtered_countrycode = filter_stations("US", &stations);
        assert_eq!(filtered_countrycode.len(), 1);
        assert_eq!(filtered_countrycode[0].name, "Jazz Lounge");

        // Search by location / state
        let filtered_state = filter_stations("Tokyo", &stations);
        assert_eq!(filtered_state.len(), 1);
        assert_eq!(filtered_state[0].name, "Vaporwave FM");

        let filtered_california = filter_stations("california", &stations);
        assert_eq!(filtered_california.len(), 1);
        assert_eq!(filtered_california[0].name, "Jazz Lounge");

        // Non-matching search
        let filtered_none = filter_stations("nonexistent-query-xyz", &stations);
        assert!(filtered_none.is_empty());
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("Short", 10), "Short");
        assert_eq!(truncate("A very long station title", 10), "A very lon");
    }
}
