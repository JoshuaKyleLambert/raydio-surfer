use crate::bands::GenreBand;
use radiobrowser::{ApiStation, StationOrder, blocking::RadioBrowserAPI};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
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
pub fn save_stations_to_cache(stations: &[CachedStation]) {
    if let Ok(json_data) = serde_json::to_string(stations) {
        let _ = fs::write(CACHE_FILENAME, json_data);
    }
}

pub fn map_api_stations(stations: Vec<ApiStation>) -> Vec<CachedStation> {
    stations
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

    let mapped = map_api_stations(api_stations.unwrap());
    save_stations_to_cache(&mapped);
    mapped
}

pub fn fetch_remote_stations(
    api: &mut RadioBrowserAPI,
    band: GenreBand,
    query: &str,
) -> Vec<CachedStation> {
    let query_trimmed = query.trim();

    if query_trimmed.is_empty() {
        if matches!(band, GenreBand::All) {
            if let Ok(res) = api
                .get_stations()
                .limit(MAX_STATIONS_LIMIT)
                .hidebroken(true)
                .order(StationOrder::Clickcount)
                .reverse(true)
                .send()
            {
                return map_api_stations(res);
            }
        } else {
            let keywords = band.keywords().to_vec();
            if !keywords.is_empty()
                && let Ok(res) = api
                    .get_stations()
                    .tag_list(keywords)
                    .limit(MAX_STATIONS_LIMIT)
                    .hidebroken(true)
                    .order(StationOrder::Clickcount)
                    .reverse(true)
                    .send()
            {
                let mapped = map_api_stations(res);
                let filtered: Vec<CachedStation> = mapped
                    .into_iter()
                    .filter(|st| band.matches(st))
                    .collect();
                if !filtered.is_empty() {
                    return filtered;
                }
            }

            // Fallback: general query filtered by band
            if let Ok(res) = api
                .get_stations()
                .limit(MAX_STATIONS_LIMIT)
                .hidebroken(true)
                .order(StationOrder::Clickcount)
                .reverse(true)
                .send()
            {
                let mapped = map_api_stations(res);
                return mapped.into_iter().filter(|st| band.matches(st)).collect();
            }
        }
        return Vec::new();
    }

    // Non-empty query: search across name, tag, country, state, language on live API
    let mut collected: Vec<CachedStation> = Vec::new();
    let mut seen_uuids = std::collections::HashSet::new();

    let mut add_results = |api_stations: Vec<ApiStation>| {
        for s in api_stations {
            if seen_uuids.insert(s.stationuuid.clone()) {
                let st = CachedStation {
                    stationuuid: s.stationuuid,
                    name: s.name,
                    url: s.url,
                    tags: s.tags,
                    country: s.country,
                    countrycode: s.countrycode,
                    state: s.state,
                    language: s.language,
                };
                if band.matches(&st) {
                    collected.push(st);
                }
            }
        }
    };

    if let Ok(res) = api
        .get_stations()
        .name(query_trimmed)
        .limit(MAX_STATIONS_LIMIT)
        .hidebroken(true)
        .order(StationOrder::Clickcount)
        .reverse(true)
        .send()
    {
        add_results(res);
    }

    if let Ok(res) = api
        .get_stations()
        .tag(query_trimmed)
        .limit(MAX_STATIONS_LIMIT)
        .hidebroken(true)
        .order(StationOrder::Clickcount)
        .reverse(true)
        .send()
    {
        add_results(res);
    }

    if let Ok(res) = api
        .get_stations()
        .country(query_trimmed)
        .limit(MAX_STATIONS_LIMIT)
        .hidebroken(true)
        .order(StationOrder::Clickcount)
        .reverse(true)
        .send()
    {
        add_results(res);
    }

    if let Ok(res) = api
        .get_stations()
        .state(query_trimmed)
        .limit(MAX_STATIONS_LIMIT)
        .hidebroken(true)
        .order(StationOrder::Clickcount)
        .reverse(true)
        .send()
    {
        add_results(res);
    }

    if let Ok(res) = api
        .get_stations()
        .language(query_trimmed)
        .limit(MAX_STATIONS_LIMIT)
        .hidebroken(true)
        .order(StationOrder::Clickcount)
        .reverse(true)
        .send()
    {
        add_results(res);
    }

    if collected.len() > 5000 {
        collected.truncate(5000);
    }

    collected
}

#[derive(Debug, Clone)]
pub struct FetchRequest {
    pub id: u64,
    pub band: GenreBand,
    pub query: String,
}

#[derive(Debug, Clone)]
pub struct FetchResponse {
    pub id: u64,
    pub band: GenreBand,
    pub query: String,
    pub stations: Vec<CachedStation>,
}

pub struct StationLoader {
    request_tx: Sender<FetchRequest>,
    response_rx: Receiver<FetchResponse>,
    current_request_id: u64,
    is_loading: bool,
    band_cache: HashMap<GenreBand, Vec<CachedStation>>,
    search_cache: HashMap<(GenreBand, String), Vec<CachedStation>>,
    pending_request: Option<(GenreBand, String)>,
    debounce_timer: f32,
    all_stations_snapshot: Vec<CachedStation>,
}

impl StationLoader {
    pub fn new() -> Self {
        let (request_tx, request_rx) = channel::<FetchRequest>();
        let (response_tx, response_rx) = channel::<FetchResponse>();

        thread::spawn(move || {
            let mut api = RadioBrowserAPI::new().ok();
            while let Ok(req) = request_rx.recv() {
                if api.is_none() {
                    api = RadioBrowserAPI::new().ok();
                }
                let stations = if let Some(ref mut client) = api {
                    fetch_remote_stations(client, req.band, &req.query)
                } else {
                    Vec::new()
                };
                let _ = response_tx.send(FetchResponse {
                    id: req.id,
                    band: req.band,
                    query: req.query,
                    stations,
                });
            }
        });

        let mut band_cache = HashMap::new();
        let initial_snapshot = load_stations_from_cache().unwrap_or_default();
        if !initial_snapshot.is_empty() {
            band_cache.insert(GenreBand::All, initial_snapshot.clone());
        }

        let mut loader = Self {
            request_tx,
            response_rx,
            current_request_id: 0,
            is_loading: false,
            band_cache,
            search_cache: HashMap::new(),
            pending_request: None,
            debounce_timer: 0.0,
            all_stations_snapshot: initial_snapshot,
        };

        // If no disk cache is present, initiate immediate background fetch for All stations
        if loader.all_stations_snapshot.is_empty() {
            loader.fetch_remote(GenreBand::All, "");
        }

        loader
    }

    pub fn initial_stations(&self) -> Vec<CachedStation> {
        self.band_cache
            .get(&GenreBand::All)
            .cloned()
            .unwrap_or_default()
    }

    pub fn total_cached_count(&self) -> usize {
        self.all_stations_snapshot.len()
    }

    pub fn is_loading(&self) -> bool {
        self.is_loading
    }

    fn fetch_remote(&mut self, band: GenreBand, query: &str) {
        self.current_request_id += 1;
        self.is_loading = true;
        let req = FetchRequest {
            id: self.current_request_id,
            band,
            query: query.to_string(),
        };
        let _ = self.request_tx.send(req);
    }

    pub fn request_stations(
        &mut self,
        band: GenreBand,
        query: &str,
        immediate: bool,
    ) -> Option<Vec<CachedStation>> {
        let query_trimmed = query.trim();

        if query_trimmed.is_empty() {
            self.pending_request = None;
            self.debounce_timer = 0.0;

            if let Some(cached) = self.band_cache.get(&band) {
                self.is_loading = false;
                return Some(cached.clone());
            }

            self.fetch_remote(band, "");
            return None;
        }

        let key = (band, query_trimmed.to_string());
        if let Some(cached) = self.search_cache.get(&key) {
            self.pending_request = None;
            self.debounce_timer = 0.0;
            self.is_loading = false;
            return Some(cached.clone());
        }

        if immediate {
            self.pending_request = None;
            self.debounce_timer = 0.0;
            self.fetch_remote(band, query_trimmed);
        } else {
            self.pending_request = Some((band, query_trimmed.to_string()));
            self.debounce_timer = 0.35; // 350ms debounce
            self.is_loading = true;
        }

        None
    }

    pub fn update(&mut self, dt: f32) {
        if self.debounce_timer > 0.0 {
            self.debounce_timer -= dt;
            if self.debounce_timer <= 0.0
                && let Some((band, query)) = self.pending_request.take()
            {
                self.fetch_remote(band, &query);
            }
        }
    }

    pub fn handle_response(&mut self, resp: FetchResponse) {
        let trimmed_query = resp.query.trim().to_string();
        if trimmed_query.is_empty() {
            if matches!(resp.band, GenreBand::All) && !resp.stations.is_empty() {
                self.all_stations_snapshot = resp.stations.clone();
                save_stations_to_cache(&resp.stations);
            }
            self.band_cache.insert(resp.band, resp.stations);
        } else {
            self.search_cache
                .insert((resp.band, trimmed_query), resp.stations);
        }
        self.is_loading = false;
    }

    pub fn poll_response(&mut self) -> Option<FetchResponse> {
        let mut latest_matching_resp = None;
        while let Ok(resp) = self.response_rx.try_recv() {
            if resp.id == self.current_request_id {
                self.handle_response(resp.clone());
                latest_matching_resp = Some(resp);
            }
        }
        latest_matching_resp
    }

    #[allow(dead_code)]
    pub fn insert_band_cache(&mut self, band: GenreBand, stations: Vec<CachedStation>) {
        self.band_cache.insert(band, stations);
    }

    #[allow(dead_code)]
    pub fn insert_search_cache(&mut self, band: GenreBand, query: &str, stations: Vec<CachedStation>) {
        self.search_cache
            .insert((band, query.trim().to_string()), stations);
    }
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
    fn test_station_loader_in_memory_session_cache() {
        let mut loader = StationLoader::new();
        let sample_rock = vec![CachedStation {
            stationuuid: "rock-1".into(),
            name: "Pure Rock".into(),
            url: "http://purerock.fm".into(),
            tags: "rock,guitar".into(),
            ..Default::default()
        }];

        loader.insert_band_cache(GenreBand::Rock, sample_rock.clone());

        // Requesting cached band returns instantly from in-memory cache without network delay
        let result = loader.request_stations(GenreBand::Rock, "", false);
        assert_eq!(result, Some(sample_rock));
        assert!(!loader.is_loading());
    }

    #[test]
    fn test_station_loader_search_cache() {
        let mut loader = StationLoader::new();
        let sample_stations = vec![CachedStation {
            stationuuid: "jazz-1".into(),
            name: "Tokyo Jazz Radio".into(),
            url: "http://tokyojazz.com".into(),
            tags: "jazz,bebop".into(),
            country: "Japan".into(),
            countrycode: "JP".into(),
            state: "Tokyo".into(),
            language: "japanese".into(),
        }];

        // Simulate receiving a response from the worker
        let resp = FetchResponse {
            id: loader.current_request_id,
            band: GenreBand::All,
            query: "tokyo".to_string(),
            stations: sample_stations.clone(),
        };

        loader.handle_response(resp);

        // Subsequent query for "tokyo" retrieves from memory cache immediately
        let cached = loader.request_stations(GenreBand::All, "tokyo", false);
        assert_eq!(cached, Some(sample_stations));
        assert!(!loader.is_loading());
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("Short", 10), "Short");
        assert_eq!(truncate("A very long station title", 10), "A very lon");
    }
}
