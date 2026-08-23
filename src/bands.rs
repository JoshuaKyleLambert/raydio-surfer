use crate::api::CachedStation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenreBand {
    All,
    Rock,
    Jazz,
    Electro,
    Pop,
    Classic,
    Ambient,
    News,
    Retro,
}

impl GenreBand {
    pub const ALL_BANDS: [GenreBand; 9] = [
        GenreBand::All,
        GenreBand::Rock,
        GenreBand::Jazz,
        GenreBand::Electro,
        GenreBand::Pop,
        GenreBand::Classic,
        GenreBand::Ambient,
        GenreBand::News,
        GenreBand::Retro,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            GenreBand::All => "ALL",
            GenreBand::Rock => "ROCK",
            GenreBand::Jazz => "JAZZ",
            GenreBand::Electro => "ELECTRO",
            GenreBand::Pop => "POP",
            GenreBand::Classic => "CLASSIC",
            GenreBand::Ambient => "AMBIENT",
            GenreBand::News => "NEWS",
            GenreBand::Retro => "80s/90s",
        }
    }

    pub fn matches(&self, station: &CachedStation) -> bool {
        if matches!(self, GenreBand::All) {
            return true;
        }

        let tags = station.tags.to_lowercase();
        let name = station.name.to_lowercase();

        let match_any = |keywords: &[&str]| {
            keywords
                .iter()
                .any(|&kw| tags.contains(kw) || name.contains(kw))
        };

        match self {
            GenreBand::All => true,
            GenreBand::Rock => match_any(&["rock", "metal", "punk", "indie", "guitar", "alt"]),
            GenreBand::Jazz => match_any(&["jazz", "blues", "soul", "funk", "smooth"]),
            GenreBand::Electro => match_any(&[
                "electro",
                "synth",
                "dance",
                "edm",
                "techno",
                "house",
                "trance",
                "vaporwave",
                "chillwave",
                "club",
            ]),
            GenreBand::Pop => match_any(&["pop", "top40", "hits", "chart"]),
            GenreBand::Classic => {
                match_any(&["classic", "orchestra", "symphon", "opera", "baroque"])
            }
            GenreBand::Ambient => match_any(&[
                "ambient",
                "chill",
                "relax",
                "meditat",
                "lounge",
                "downtempo",
            ]),
            GenreBand::News => match_any(&["news", "talk", "npr", "bbc", "politics", "sport"]),
            GenreBand::Retro => {
                match_any(&["80s", "90s", "70s", "retro", "disco", "oldies", "vintage"])
            }
        }
    }
}

pub fn filter_by_band_and_search(
    stations: &[CachedStation],
    band: GenreBand,
    search: &str,
) -> Vec<CachedStation> {
    let search_lower = search.trim().to_lowercase();

    stations
        .iter()
        .filter(|station| {
            if !band.matches(station) {
                return false;
            }
            if search_lower.is_empty() {
                return true;
            }
            station.name.to_lowercase().contains(&search_lower)
                || station.tags.to_lowercase().contains(&search_lower)
                || station.url.to_lowercase().contains(&search_lower)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genre_band_matching() {
        let rock_station = CachedStation {
            stationuuid: "1".into(),
            name: "Classic Rock 101".into(),
            url: "http://stream1".into(),
            tags: "classic rock,70s,guitar".into(),
        };

        let jazz_station = CachedStation {
            stationuuid: "2".into(),
            name: "Blue Note FM".into(),
            url: "http://stream2".into(),
            tags: "smooth jazz,relax".into(),
        };

        assert!(GenreBand::Rock.matches(&rock_station));
        assert!(!GenreBand::Rock.matches(&jazz_station));
        assert!(GenreBand::Jazz.matches(&jazz_station));
        assert!(GenreBand::All.matches(&rock_station));
        assert!(GenreBand::All.matches(&jazz_station));
    }
}
