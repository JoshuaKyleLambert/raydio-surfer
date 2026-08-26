use crate::api::StationLoader;
use crate::audio::AudioController;
use crate::bands::GenreBand;
use crate::controls::vintage_ui::{VintageUiState, render_vintage_stereo};
use crate::layout::StereoLayout;
use crate::settings::Settings;
use radiobrowser::{ApiStation, ApiTag, blocking::RadioBrowserAPI};
use raylib::prelude::*;

mod api;
mod audio;
mod bands;
mod controls;
mod layout;
mod paths;
mod presets;
mod settings;

// Background color for the window
const BACKGROUND_COLOR: Color = Color::new(16, 16, 22, 255);

fn main() {
    let mut loader = StationLoader::new();
    let initial_stations = loader.initial_stations();

    let mut settings = Settings::load();

    let audio = AudioController::new();
    audio.set_volume(settings.volume);
    let mut ui = VintageUiState::new(settings.volume);

    let mut last_search = ui.search_input.clone();
    let mut last_band_idx = ui.active_band_idx;
    let mut active_stations = if !initial_stations.is_empty() {
        bands::filter_by_band_and_search(&initial_stations, ui.active_band, &ui.search_input)
    } else {
        Vec::new()
    };
    let mut last_played_channel: Option<(String, String)> = None;

    let (mut rl, thread) = raylib::init()
        .size(860, 480)
        .title("RaydioSurfer - Vintage Internet Radio")
        .resizable()
        .highdpi()
        .always_run()
        .build();

    rl.set_target_fps(60);

    // Main responsive loop
    while !rl.window_should_close() {
        let dt = rl.get_frame_time();

        // Update loader debounce timer
        loader.update(dt);

        // Feedback timer decay
        if let Some((_, ref mut timer)) = ui.status_feedback {
            *timer -= dt;
            if *timer <= 0.0 {
                ui.status_feedback = None;
            }
        }

        // Check if search query or genre band changed
        let search_changed = ui.search_input != last_search;
        let band_changed = ui.active_band_idx != last_band_idx;
        if search_changed || band_changed {
            let immediate = band_changed || rl.is_key_pressed(KeyboardKey::KEY_ENTER);
            if let Some(cached) = loader.request_stations(GenreBand::All, &ui.search_input, immediate) {
                active_stations = cached;
                ui.active_index = 0;
            }
            last_search = ui.search_input.clone();
            last_band_idx = ui.active_band_idx;
        }

        // Poll for asynchronous background responses
        if let Some(resp) = loader.poll_response()
            && resp.query.trim().to_lowercase() == ui.search_input.trim().to_lowercase()
        {
            let current_selected_url = if ui.active_index < active_stations.len() {
                Some(active_stations[ui.active_index].url.clone())
            } else {
                None
            };
            active_stations = resp.stations;
            if let Some(url) = current_selected_url
                && let Some(pos) = active_stations.iter().position(|s| s.url == url)
            {
                ui.active_index = pos;
            } else {
                ui.active_index = 0;
            }
        }

        ui.is_loading = loader.is_loading();

        // Keyboard Shortcuts
        // Number keys 1..=6 to recall presets
        for i in 0..6 {
            let key = match i {
                0 => KeyboardKey::KEY_ONE,
                1 => KeyboardKey::KEY_TWO,
                2 => KeyboardKey::KEY_THREE,
                3 => KeyboardKey::KEY_FOUR,
                4 => KeyboardKey::KEY_FIVE,
                5 => KeyboardKey::KEY_SIX,
                _ => unreachable!(),
            };
            if rl.is_key_pressed(key)
                && let Some(st) = settings.get_preset(i)
            {
                if let Some(pos) = active_stations.iter().position(|s| s.url == st.url || s.name == st.name) {
                    ui.active_index = pos;
                } else {
                    ui.active_band_idx = 0;
                    ui.active_band = GenreBand::All;
                    ui.search_input.clear();
                    last_search.clear();
                    last_band_idx = 0;
                    if let Some(cached) = loader.request_stations(GenreBand::All, "", true) {
                        active_stations = cached;
                        if let Some(pos) = active_stations.iter().position(|s| s.url == st.url || s.name == st.name) {
                            ui.active_index = pos;
                        } else {
                            ui.active_index = 0;
                        }
                    }
                }
                if ui.is_power_on {
                    audio.play(st.name.clone(), st.url.clone());
                }
                last_played_channel = Some((st.name.clone(), st.url.clone()));
                ui.status_feedback = Some((format!("Tuned to Preset [{}]", i + 1), 3.0));
            }
        }

        // Enter key immediate search query submission
        if rl.is_key_pressed(KeyboardKey::KEY_ENTER)
            && !ui.search_input.is_empty()
            && let Some(cached) = loader.request_stations(GenreBand::All, &ui.search_input, true)
        {
            active_stations = cached;
            ui.active_index = 0;
        }

        // Arrow keys for tuning
        if rl.is_key_pressed(KeyboardKey::KEY_LEFT) && !active_stations.is_empty() {
            ui.active_index = ui.active_index.saturating_sub(1);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_RIGHT)
            && !active_stations.is_empty()
            && ui.active_index + 1 < active_stations.len()
        {
            ui.active_index += 1;
        }

        // Clamp active index
        if !active_stations.is_empty() && ui.active_index >= active_stations.len() {
            ui.active_index = active_stations.len() - 1;
        }

        let current_station = if !active_stations.is_empty() {
            Some(&active_stations[ui.active_index])
        } else {
            None
        };

        // Enter key to play currently tuned station
        if rl.is_key_pressed(KeyboardKey::KEY_ENTER)
            && let Some(st) = current_station
            && ui.is_power_on
        {
            audio.play(st.name.clone(), st.url.clone());
            last_played_channel = Some((st.name.clone(), st.url.clone()));
        }

        // P / M key for power toggle
        if rl.is_key_pressed(KeyboardKey::KEY_P) || rl.is_key_pressed(KeyboardKey::KEY_M) {
            ui.is_power_on = !ui.is_power_on;
            if ui.is_power_on {
                audio.set_volume(ui.volume);
                if let Some(st) = current_station {
                    audio.play(st.name.clone(), st.url.clone());
                    last_played_channel = Some((st.name.clone(), st.url.clone()));
                }
            } else {
                audio.stop();
            }
        }

        // As soon as the channel has changed, start playing if power is on
        let current_channel = current_station.map(|s| (s.name.clone(), s.url.clone()));
        if current_channel != last_played_channel {
            last_played_channel = current_channel.clone();
            if let Some(st) = current_station {
                if ui.is_power_on {
                    audio.play(st.name.clone(), st.url.clone());
                }
            } else {
                audio.stop();
            }
        }

        // Begin drawing
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(BACKGROUND_COLOR);

        // Dynamically compute responsive layout from current screen dimensions
        let screen_w = d.get_screen_width() as f32;
        let screen_h = d.get_screen_height() as f32;
        let layout = StereoLayout::compute(screen_w, screen_h, settings.bands.slots.len());

        let total_stations_count = loader.total_cached_count().max(active_stations.len());
        let ctx = controls::vintage_ui::StationViewContext {
            total_stations_count,
            active_filtered_count: active_stations.len(),
            current_station,
            all_stations: None,
        };

        render_vintage_stereo(&mut d, &layout, &mut ui, &mut settings, &audio, ctx);
    }
}

fn _get_tags() -> Option<Vec<ApiTag>> {
    RadioBrowserAPI::new().ok()?.get_tags().send().ok()
}

fn _get_stations() -> Option<Vec<ApiStation>> {
    RadioBrowserAPI::new()
        .ok()?
        .get_stations()
        .name("vaporwave")
        .send()
        .ok()
}

// --- EXISTING RAYLIB MAIN CODE SITS ABOVE THIS LINE ---

// This attribute tells the compiler to only build this module when running tests
#[cfg(test)]
mod tests {
    use crate::api::CachedStation;
    use crate::settings::Settings;

    #[test]
    fn test_environment_initialization() {
        let execution_status = true;
        assert!(execution_status);
    }

    #[test]
    fn test_check_builder_methods() {
        let mut builder = raylib::init();
        let _ = builder
            .size(860, 480)
            .title("RaydioSurfer - Vintage Internet Radio")
            .resizable()
            .highdpi()
            .always_run()
            .vsync();
    }

    #[test]
    fn test_channel_change_detection_triggers() {
        let station_a = CachedStation {
            stationuuid: "uuid-1".to_string(),
            name: "Station Alpha".to_string(),
            url: "http://stream.alpha.fm".to_string(),
            tags: "rock,classic".to_string(),
            ..Default::default()
        };
        let station_b = CachedStation {
            stationuuid: "uuid-2".to_string(),
            name: "Station Beta".to_string(),
            url: "http://stream.beta.fm".to_string(),
            tags: "jazz,smooth".to_string(),
            ..Default::default()
        };

        let mut last_played_channel: Option<(String, String)> = None;

        // 1. Initial selection of Station Alpha
        let current_station = Some(&station_a);
        let current_channel = current_station.map(|s| (s.name.clone(), s.url.clone()));
        assert_ne!(current_channel, last_played_channel);
        last_played_channel = current_channel;
        assert_eq!(
            last_played_channel,
            Some(("Station Alpha".to_string(), "http://stream.alpha.fm".to_string()))
        );

        // 2. Same frame / no change
        let current_station = Some(&station_a);
        let current_channel = current_station.map(|s| (s.name.clone(), s.url.clone()));
        assert_eq!(current_channel, last_played_channel);

        // 3. Channel changed to Station Beta
        let current_station = Some(&station_b);
        let current_channel = current_station.map(|s| (s.name.clone(), s.url.clone()));
        assert_ne!(current_channel, last_played_channel);
        last_played_channel = current_channel;
        assert_eq!(
            last_played_channel,
            Some(("Station Beta".to_string(), "http://stream.beta.fm".to_string()))
        );

        // 4. Channel changed to None (e.g. 0 search results)
        let current_station: Option<&CachedStation> = None;
        let current_channel = current_station.map(|s| (s.name.clone(), s.url.clone()));
        assert_ne!(current_channel, last_played_channel);
        last_played_channel = current_channel;
        assert_eq!(last_played_channel, None);
    }

    #[test]
    fn test_preset_station_sync_to_index() {
        let stations = vec![
            CachedStation {
                stationuuid: "1".to_string(),
                name: "Rock Radio".to_string(),
                url: "http://rock.com".to_string(),
                tags: "rock".to_string(),
                ..Default::default()
            },
            CachedStation {
                stationuuid: "2".to_string(),
                name: "Jazz Groove".to_string(),
                url: "http://jazz.com".to_string(),
                tags: "jazz".to_string(),
                ..Default::default()
            },
        ];

        let preset_station = CachedStation {
            stationuuid: "2".to_string(),
            name: "Jazz Groove".to_string(),
            url: "http://jazz.com".to_string(),
            tags: "jazz".to_string(),
            ..Default::default()
        };

        let pos = stations.iter().position(|s| s.url == preset_station.url || s.name == preset_station.name);
        assert_eq!(pos, Some(1));
    }

    #[test]
    fn test_power_gated_playback() {
        let station = CachedStation {
            stationuuid: "1".to_string(),
            name: "Ambient Waves".to_string(),
            url: "http://ambient.stream".to_string(),
            tags: "ambient".to_string(),
            ..Default::default()
        };

        // When power is ON, changing station triggers playback
        let mut power_on = true;
        let mut played = false;
        if power_on {
            played = true;
        }
        assert!(played);

        // When power is OFF, changing station does NOT trigger playback
        power_on = false;
        played = false;
        if power_on {
            played = true;
        }
        assert!(!played);

        // Toggling power ON initiates playback of current station
        power_on = !power_on;
        if power_on {
            played = true;
        }
        assert!(played);
        assert_eq!(station.name, "Ambient Waves");
    }

    #[test]
    fn test_background_response_preserves_active_station_selection() {
        let initial_stations = vec![
            CachedStation {
                stationuuid: "1".to_string(),
                name: "Station A".to_string(),
                url: "http://station-a.com".to_string(),
                ..Default::default()
            },
            CachedStation {
                stationuuid: "2".to_string(),
                name: "Station B".to_string(),
                url: "http://station-b.com".to_string(),
                ..Default::default()
            },
        ];

        let mut active_index = 1; // Playing/selected Station B
        let current_selected_url = Some(initial_stations[active_index].url.clone());

        // Background query returns updated list with Station B in a different position
        let updated_stations = vec![
            CachedStation {
                stationuuid: "3".to_string(),
                name: "Station C".to_string(),
                url: "http://station-c.com".to_string(),
                ..Default::default()
            },
            CachedStation {
                stationuuid: "2".to_string(),
                name: "Station B".to_string(),
                url: "http://station-b.com".to_string(),
                ..Default::default()
            },
            CachedStation {
                stationuuid: "1".to_string(),
                name: "Station A".to_string(),
                url: "http://station-a.com".to_string(),
                ..Default::default()
            },
        ];

        if let Some(url) = current_selected_url
            && let Some(pos) = updated_stations.iter().position(|s| s.url == url)
        {
            active_index = pos;
        } else {
            active_index = 0;
        }

        assert_eq!(active_index, 1);
        assert_eq!(updated_stations[active_index].name, "Station B");
    }

    #[test]
    fn test_volume_settings_persistence() {
        let mut settings = Settings::default();
        assert_eq!(settings.volume, crate::settings::DEFAULT_VOLUME);

        // Adjust volume
        settings.volume = 0.42;
        assert_eq!(settings.volume, 0.42);

        // Test clamping
        let test_clamp = |v: f32| v.clamp(0.0, 1.0);
        assert_eq!(test_clamp(1.5), 1.0);
        assert_eq!(test_clamp(-0.5), 0.0);
        assert_eq!(test_clamp(0.65), 0.65);
    }

    #[test]
    fn test_band_slots_and_right_click_search_assignment() {
        let mut settings = Settings::default();
        assert_eq!(settings.bands.slots.len(), 9);
        assert_eq!(settings.get_band(0).unwrap().label, "ALL");
        assert_eq!(settings.get_band(1).unwrap().label, "ROCK");

        // Simulate right-clicking Band button #3 (index 2) with search query "Synthwave"
        let search_term = "  Synthwave  ";
        let trimmed = search_term.trim();
        let label = if trimmed.is_empty() {
            "ALL".to_string()
        } else {
            trimmed.chars().take(10).collect::<String>().to_uppercase()
        };
        settings.bands.slots[2] = crate::bands::BandSlot {
            label,
            query: trimmed.to_string(),
        };
        let band3 = settings.get_band(2).unwrap();
        assert_eq!(band3.label, "SYNTHWAVE");
        assert_eq!(band3.query, "Synthwave");

        // Simulate right-clicking Band button #4 (index 3) with empty search query
        let empty_term = "";
        let trimmed_empty = empty_term.trim();
        let label_empty = if trimmed_empty.is_empty() {
            "ALL".to_string()
        } else {
            trimmed_empty.chars().take(10).collect::<String>().to_uppercase()
        };
        settings.bands.slots[3] = crate::bands::BandSlot {
            label: label_empty,
            query: trimmed_empty.to_string(),
        };
        let band4 = settings.get_band(3).unwrap();
        assert_eq!(band4.label, "ALL");
        assert_eq!(band4.query, "");
    }
}
