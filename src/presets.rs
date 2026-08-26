use crate::api::CachedStation;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Presets {
    pub slots: [Option<CachedStation>; 6],
}

impl Presets {
    pub fn set_preset(&mut self, slot_idx: usize, station: CachedStation) -> bool {
        if slot_idx < 6 {
            self.slots[slot_idx] = Some(station);
            true
        } else {
            false
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

        let ok = presets.set_preset(2, station.clone());
        assert!(ok);
        assert_eq!(presets.get_preset(2).unwrap().name, "Ambient Sleep");
    }
}
