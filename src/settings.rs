use crate::api::CachedStation;
use crate::bands::{BandSlot, Bands};
use crate::paths::{self, SETTINGS_FILENAME};
use crate::presets::Presets;
use serde::{Deserialize, Serialize};
use std::fs;
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
        let path = paths::settings_path();
        let mut settings = if path.exists()
            && let Ok(content) = fs::read_to_string(&path)
            && let Ok(loaded) = serde_json::from_str::<Settings>(&content)
        {
            loaded
        } else if let Ok(content) = fs::read_to_string(SETTINGS_FILENAME)
            && let Ok(loaded) = serde_json::from_str::<Settings>(&content)
        {
            loaded
        } else {
            Self::default()
        };

        // Migration: If legacy presets.json or bands.json exist on disk, migrate them into settings
        let presets_local = Path::new("presets.json");
        let presets_config = paths::config_dir().map(|d| d.join("presets.json"));
        let presets_content = presets_config
            .as_deref()
            .and_then(|p| fs::read_to_string(p).ok())
            .or_else(|| fs::read_to_string(presets_local).ok());

        if let Some(content) = presets_content
            && settings.presets.slots.iter().all(|s| s.is_none())
            && let Ok(loaded_presets) = serde_json::from_str::<Presets>(&content)
        {
            settings.presets = loaded_presets;
        }

        let bands_local = Path::new("bands.json");
        let bands_config = paths::config_dir().map(|d| d.join("bands.json"));
        let bands_content = bands_config
            .as_deref()
            .and_then(|p| fs::read_to_string(p).ok())
            .or_else(|| fs::read_to_string(bands_local).ok());

        if let Some(content) = bands_content
            && settings.bands == Bands::default()
            && let Ok(loaded_bands) = serde_json::from_str::<Bands>(&content)
        {
            settings.bands = loaded_bands;
        }

        settings.save();
        settings
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let path = paths::settings_path();
            paths::ensure_parent_dir_exists(&path);
            let _ = fs::write(path, json);
        }
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
        let mut settings = Settings::default();
        settings.volume = 0.5;
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
        let mut settings = Settings::default();
        settings.volume = 0.85;
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
}
