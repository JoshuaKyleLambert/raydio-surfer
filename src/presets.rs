use crate::api::CachedStation;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const PRESETS_FILENAME: &str = "presets.json";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Presets {
    pub slots: [Option<CachedStation>; 6],
}

impl Presets {
    pub fn load() -> Self {
        let path = Path::new(PRESETS_FILENAME);
        if path.exists()
            && let Ok(content) = fs::read_to_string(path)
            && let Ok(loaded) = serde_json::from_str::<Presets>(&content)
        {
            return loaded;
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(PRESETS_FILENAME, json);
        }
    }

    pub fn set_preset(&mut self, slot_idx: usize, station: CachedStation) {
        if slot_idx < 6 {
            self.slots[slot_idx] = Some(station);
            self.save();
        }
    }

    pub fn get_preset(&self, slot_idx: usize) -> Option<&CachedStation> {
        if slot_idx < 6 {
            self.slots[slot_idx].as_ref()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presets_in_memory() {
        let mut presets = Presets::default();
        assert!(presets.get_preset(0).is_none());

        let station = CachedStation {
            stationuuid: "123".into(),
            name: "Ambient Sleep".into(),
            url: "http://sleep".into(),
            tags: "ambient".into(),
            ..Default::default()
        };

        presets.set_preset(2, station.clone());
        assert_eq!(presets.get_preset(2).unwrap().name, "Ambient Sleep");
    }
}
