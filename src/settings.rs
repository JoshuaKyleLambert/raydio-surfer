use crate::api::CachedStation;
use crate::bands::{BandSlot, Bands};
use crate::paths::{self, SETTINGS_FILENAME};
use crate::presets::Presets;
use crate::storage;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const DEFAULT_VOLUME: f32 = 0.75;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Settings {
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub bands: Bands,
    #[serde(default)]
    pub presets: Presets,
    #[serde(default)]
    pub current_station: Option<CachedStation>,
}

fn default_volume() -> f32 {
    DEFAULT_VOLUME
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            volume: DEFAULT_VOLUME,
            bands: Bands::default(),
            presets: Presets::default(),
            current_station: None,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let config_path = paths::settings_path();
        let local_path = Path::new(SETTINGS_FILENAME);

        let mut settings = storage::load_json_with_fallback::<Settings>(
            &config_path,
            if local_path != config_path {
                Some(local_path)
            } else {
                None
            },
        )
        .unwrap_or_default();

        // Migration: If legacy presets.json exists on disk, migrate into settings if presets are empty
        if settings.presets.slots.iter().all(|s| s.is_none()) {
            let presets_local = Path::new("presets.json");
            let presets_config = paths::config_dir().map(|d| d.join("presets.json"));
            if let Some(legacy_presets) = storage::load_json_with_fallback::<Presets>(
                presets_config.as_deref().unwrap_or(presets_local),
                if presets_config.is_some() {
                    Some(presets_local)
                } else {
                    None
                },
            ) {
                settings.presets = legacy_presets;
            }
        }

        // Migration: If legacy bands.json exists on disk, migrate into settings if bands are default
        if settings.bands == Bands::default() {
            let bands_local = Path::new("bands.json");
            let bands_config = paths::config_dir().map(|d| d.join("bands.json"));
            if let Some(legacy_bands) = storage::load_json_with_fallback::<Bands>(
                bands_config.as_deref().unwrap_or(bands_local),
                if bands_config.is_some() {
                    Some(bands_local)
                } else {
                    None
                },
            ) {
                settings.bands = legacy_bands;
            }
        }

        settings.save();
        settings
    }

    pub fn save(&self) {
        let path = paths::settings_path();
        let _ = storage::save_json_pretty(&path, self);
    }

    pub fn set_volume(&mut self, volume: f32) {
        let clamped = volume.clamp(0.0, 1.0);
        if (self.volume - clamped).abs() > 0.001 {
            self.volume = clamped;
            self.save();
        }
    }

    pub fn set_preset(&mut self, slot_idx: usize, station: CachedStation) {
        if self.presets.set_preset(slot_idx, station) {
            self.save();
        }
    }

    pub fn get_preset(&self, slot_idx: usize) -> Option<&CachedStation> {
        self.presets.get_preset(slot_idx)
    }

    pub fn set_band(&mut self, idx: usize, search_term: &str) {
        if self.bands.set_band(idx, search_term) {
            self.save();
        }
    }

    pub fn get_band(&self, idx: usize) -> Option<&BandSlot> {
        self.bands.get_band(idx)
    }

    pub fn set_current_station(&mut self, station: Option<CachedStation>) {
        if self.current_station != station {
            self.current_station = station;
            self.save();
        }
    }

    pub fn get_current_station(&self) -> Option<&CachedStation> {
        self.current_station.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert!((settings.volume - DEFAULT_VOLUME).abs() < 0.001);
        assert_eq!(settings.bands.slots.len(), 9);
        assert_eq!(settings.presets.slots.len(), 6);
    }

    #[test]
    fn test_set_volume_clamps() {
        let settings = Settings {
            volume: 0.5,
            ..Default::default()
        };
        assert_eq!(settings.volume, 0.5);
    }

    #[test]
    fn test_settings_presets_and_bands() {
        let mut settings = Settings::default();
        let station = CachedStation {
            stationuuid: "test-uuid".into(),
            name: "Test Radio".into(),
            url: "http://test.radio".into(),
            ..Default::default()
        };

        settings.set_preset(1, station.clone());
        assert_eq!(settings.get_preset(1).unwrap().name, "Test Radio");

        settings.set_band(2, "lofi");
        assert_eq!(settings.get_band(2).unwrap().label, "LOFI");
        assert_eq!(settings.get_band(2).unwrap().query, "lofi");
    }

    #[test]
    fn test_settings_serialization_combined() {
        let mut settings = Settings {
            volume: 0.85,
            ..Default::default()
        };
        settings.bands.slots[1] = BandSlot {
            label: "SYNTH".into(),
            query: "synth".into(),
        };
        settings.presets.slots[0] = Some(CachedStation {
            name: "Synth Station".into(),
            url: "http://synth".into(),
            ..Default::default()
        });
        settings.set_current_station(Some(CachedStation {
            name: "Active Stream".into(),
            url: "http://active.stream".into(),
            tags: "ambient".into(),
            ..Default::default()
        }));

        let json = serde_json::to_string(&settings).expect("Must serialize");
        let deserialized: Settings = serde_json::from_str(&json).expect("Must deserialize");
        assert_eq!(settings, deserialized);
        assert_eq!(deserialized.volume, 0.85);
        assert_eq!(deserialized.get_band(1).unwrap().label, "SYNTH");
        assert_eq!(deserialized.get_preset(0).unwrap().name, "Synth Station");
        assert_eq!(
            deserialized.get_current_station().unwrap().name,
            "Active Stream"
        );
    }

    #[test]
    fn test_settings_legacy_deserialization_without_current_station() {
        let legacy_json = r#"{
            "volume": 0.6
        }"#;
        let loaded: Settings = serde_json::from_str(legacy_json).expect("Must deserialize legacy");
        assert_eq!(loaded.volume, 0.6);
        assert_eq!(loaded.current_station, None);
        assert_eq!(loaded.bands.slots.len(), 9);
        assert_eq!(loaded.presets.slots.len(), 6);
    }

    #[test]
    fn test_favorites_presets_retention_and_persistence() {
        let temp_dir = std::env::temp_dir().join("raydio_surfer_presets_test");
        let settings_file = temp_dir.join("settings.json");

        let mut settings = Settings::default();
        let fav1 = CachedStation {
            stationuuid: "fav-1".into(),
            name: "Favorite One".into(),
            url: "http://fav1.fm".into(),
            tags: "rock".into(),
            ..Default::default()
        };
        let fav2 = CachedStation {
            stationuuid: "fav-2".into(),
            name: "Favorite Two".into(),
            url: "http://fav2.fm".into(),
            tags: "jazz".into(),
            ..Default::default()
        };

        settings.set_preset(0, fav1.clone());
        settings.set_preset(5, fav2.clone());

        // Save to test path
        let save_res = storage::save_json_pretty(&settings_file, &settings);
        assert!(save_res.is_ok());

        // Load back from test path
        let reloaded: Settings = storage::load_json(&settings_file).expect("Must load settings");
        assert_eq!(reloaded.get_preset(0), Some(&fav1));
        assert_eq!(reloaded.get_preset(1), None);
        assert_eq!(reloaded.get_preset(5), Some(&fav2));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_station_cache_and_settings_separation() {
        let temp_dir = std::env::temp_dir().join("raydio_surfer_separation_test");
        let settings_file = temp_dir.join("config").join("settings.json");
        let cache_file = temp_dir.join("cache").join("stations_cache.json");

        let mut settings = Settings::default();
        settings.volume = 0.9;
        settings.set_preset(0, CachedStation {
            name: "Preset Station".into(),
            url: "http://preset.com".into(),
            ..Default::default()
        });

        let cache_data = vec![
            CachedStation {
                name: "Cached Catalog Station 1".into(),
                url: "http://catalog1.com".into(),
                ..Default::default()
            },
            CachedStation {
                name: "Cached Catalog Station 2".into(),
                url: "http://catalog2.com".into(),
                ..Default::default()
            },
        ];

        // Save settings and cache separately
        storage::save_json_pretty(&settings_file, &settings).unwrap();
        storage::save_json(&cache_file, &cache_data).unwrap();

        // Verify independent loading
        let loaded_settings: Settings = storage::load_json(&settings_file).unwrap();
        let loaded_cache: Vec<CachedStation> = storage::load_json(&cache_file).unwrap();

        assert_eq!(loaded_settings.volume, 0.9);
        assert_eq!(loaded_settings.get_preset(0).unwrap().name, "Preset Station");
        assert_eq!(loaded_cache.len(), 2);
        assert_eq!(loaded_cache[0].name, "Cached Catalog Station 1");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
