use crate::api::CachedStation;
use crate::bands::{BandSlot, Bands};
use crate::presets::Presets;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const SETTINGS_FILENAME: &str = "settings.json";
pub const DEFAULT_VOLUME: f32 = 0.75;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Settings {
    #[serde(default = "default_volume")]
    pub volume: f32,
    #[serde(default)]
    pub bands: Bands,
    #[serde(default)]
    pub presets: Presets,
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
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = Path::new(SETTINGS_FILENAME);
        let mut settings = if path.exists()
            && let Ok(content) = fs::read_to_string(path)
            && let Ok(loaded) = serde_json::from_str::<Settings>(&content)
        {
            loaded
        } else {
            Self::default()
        };

        // Migration: If legacy presets.json or bands.json exist on disk, migrate them into settings
        let presets_path = Path::new("presets.json");
        if presets_path.exists()
            && settings.presets.slots.iter().all(|s| s.is_none())
            && let Ok(content) = fs::read_to_string(presets_path)
            && let Ok(loaded_presets) = serde_json::from_str::<Presets>(&content)
        {
            settings.presets = loaded_presets;
        }

        let bands_path = Path::new("bands.json");
        if bands_path.exists()
            && settings.bands == Bands::default()
            && let Ok(content) = fs::read_to_string(bands_path)
            && let Ok(loaded_bands) = serde_json::from_str::<Bands>(&content)
        {
            settings.bands = loaded_bands;
        }

        settings.save();
        settings
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(SETTINGS_FILENAME, json);
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

        let json = serde_json::to_string(&settings).expect("Must serialize");
        let deserialized: Settings = serde_json::from_str(&json).expect("Must deserialize");
        assert_eq!(settings, deserialized);
        assert_eq!(deserialized.volume, 0.85);
        assert_eq!(deserialized.get_band(1).unwrap().label, "SYNTH");
        assert_eq!(deserialized.get_preset(0).unwrap().name, "Synth Station");
    }
}
