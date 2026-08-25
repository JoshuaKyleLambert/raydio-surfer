use crate::api::CachedStation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    pub fn keywords(&self) -> &'static [&'static str] {
        match self {
            GenreBand::All => &[],
            GenreBand::Rock => &["rock", "metal", "punk", "indie", "guitar", "alt"],
            GenreBand::Jazz => &["jazz", "blues", "soul", "funk", "smooth"],
            GenreBand::Electro => &[
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
            ],
            GenreBand::Pop => &["pop", "top40", "hits", "chart"],
            GenreBand::Classic => &["classic", "classical", "orchestra", "symphon", "opera", "baroque"],
            GenreBand::Ambient => &[
                "ambient",
                "chill",
                "relax",
                "meditat",
                "lounge",
                "downtempo",
            ],
            GenreBand::News => &["news", "talk", "npr", "bbc", "politics", "sport"],
            GenreBand::Retro => &["80s", "90s", "70s", "retro", "disco", "oldies", "vintage"],
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
            GenreBand::Rock => match_any(self.keywords()),
            GenreBand::Jazz => match_any(self.keywords()),
            GenreBand::Electro => match_any(self.keywords()),
            GenreBand::Pop => match_any(self.keywords()),
            GenreBand::Classic => match_any(self.keywords()),
            GenreBand::Ambient => match_any(self.keywords()),
            GenreBand::News => match_any(self.keywords()),
            GenreBand::Retro => match_any(self.keywords()),
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
                || station.country.to_lowercase().contains(&search_lower)
                || station.countrycode.to_lowercase().contains(&search_lower)
                || station.state.to_lowercase().contains(&search_lower)
                || station.language.to_lowercase().contains(&search_lower)
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
            country: "United States".into(),
            countrycode: "US".into(),
            state: "California".into(),
            language: "english".into(),
        };

        let jazz_station = CachedStation {
            stationuuid: "2".into(),
            name: "Blue Note FM".into(),
            url: "http://stream2".into(),
            tags: "smooth jazz,relax".into(),
            country: "United Kingdom".into(),
            countrycode: "GB".into(),
            state: "London".into(),
            language: "english".into(),
        };

        assert!(GenreBand::Rock.matches(&rock_station));
        assert!(!GenreBand::Rock.matches(&jazz_station));
        assert!(GenreBand::Jazz.matches(&jazz_station));
        assert!(GenreBand::All.matches(&rock_station));
        assert!(GenreBand::All.matches(&jazz_station));
    }

    #[test]
    fn test_filter_by_band_and_search_criteria() {
        let stations = vec![
            CachedStation {
                stationuuid: "1".into(),
                name: "K-Rock FM".into(),
                url: "http://krock".into(),
                tags: "rock,alternative,grunge".into(),
                country: "United States".into(),
                countrycode: "US".into(),
                state: "California".into(),
                language: "english".into(),
            },
            CachedStation {
                stationuuid: "2".into(),
                name: "Berlin Chillout".into(),
                url: "http://berlinchill".into(),
                tags: "ambient,downtempo,chill".into(),
                country: "Germany".into(),
                countrycode: "DE".into(),
                state: "Berlin".into(),
                language: "german".into(),
            },
            CachedStation {
                stationuuid: "3".into(),
                name: "Tokyo Jazz Cafe".into(),
                url: "http://tokyojazz".into(),
                tags: "jazz,bebop".into(),
                country: "Japan".into(),
                countrycode: "JP".into(),
                state: "Kanto".into(),
                language: "japanese".into(),
            },
        ];

        // Search by name
        let by_name = filter_by_band_and_search(&stations, GenreBand::All, "K-Rock");
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].name, "K-Rock FM");

        // Search by tag / genre
        let by_tag = filter_by_band_and_search(&stations, GenreBand::All, "grunge");
        assert_eq!(by_tag.len(), 1);
        assert_eq!(by_tag[0].name, "K-Rock FM");

        let by_genre = filter_by_band_and_search(&stations, GenreBand::All, "ambient");
        assert_eq!(by_genre.len(), 1);
        assert_eq!(by_genre[0].name, "Berlin Chillout");

        // Search by country
        let by_country = filter_by_band_and_search(&stations, GenreBand::All, "Germany");
        assert_eq!(by_country.len(), 1);
        assert_eq!(by_country[0].name, "Berlin Chillout");

        // Search by location / state / countrycode
        let by_state = filter_by_band_and_search(&stations, GenreBand::All, "Berlin");
        assert_eq!(by_state.len(), 1);
        assert_eq!(by_state[0].name, "Berlin Chillout");

        let by_location = filter_by_band_and_search(&stations, GenreBand::All, "California");
        assert_eq!(by_location.len(), 1);
        assert_eq!(by_location[0].name, "K-Rock FM");

        let by_code = filter_by_band_and_search(&stations, GenreBand::All, "JP");
        assert_eq!(by_code.len(), 1);
        assert_eq!(by_code[0].name, "Tokyo Jazz Cafe");

        // Search combined with band filter
        let by_band_and_search = filter_by_band_and_search(&stations, GenreBand::Rock, "California");
        assert_eq!(by_band_and_search.len(), 1);
        assert_eq!(by_band_and_search[0].name, "K-Rock FM");

        let non_matching_band = filter_by_band_and_search(&stations, GenreBand::Jazz, "California");
        assert!(non_matching_band.is_empty());
    }
}
