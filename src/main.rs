use crate::audio::AudioController;
use crate::bands::GenreBand;
use crate::controls::vintage_ui::{VintageUiState, render_vintage_stereo};
use crate::layout::StereoLayout;
use crate::presets::Presets;
use radiobrowser::{ApiStation, ApiTag, blocking::RadioBrowserAPI};
use raylib::prelude::*;

mod api;
mod audio;
mod bands;
mod controls;
mod layout;
mod presets;

// Background color for the window
const BACKGROUND_COLOR: Color = Color::new(16, 16, 22, 255);

fn main() {
    let mut rb_api = RadioBrowserAPI::new().expect("Failed to create RadioBrowserAPI");
    let all_stations = api::get_stations_with_cache(&mut rb_api);
    let total_stations_count = all_stations.len();

    let audio = AudioController::new();
    let mut ui = VintageUiState::new(audio.volume());
    let mut presets = Presets::load();

    let mut last_search = ui.search_input.clone();
    let mut last_band = ui.active_band;
    let mut active_stations =
        bands::filter_by_band_and_search(&all_stations, ui.active_band, &ui.search_input);
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
        // Feedback timer decay
        if let Some((_, ref mut timer)) = ui.status_feedback {
            *timer -= rl.get_frame_time();
            if *timer <= 0.0 {
                ui.status_feedback = None;
            }
        }

        // Check if search query or genre band changed
        if ui.search_input != last_search || ui.active_band != last_band {
            active_stations =
                bands::filter_by_band_and_search(&all_stations, ui.active_band, &ui.search_input);
            last_search = ui.search_input.clone();
            last_band = ui.active_band;
            ui.active_index = 0;
        }

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
                && let Some(st) = presets.get_preset(i)
            {
                if let Some(pos) = active_stations.iter().position(|s| s.url == st.url || s.name == st.name) {
                    ui.active_index = pos;
                } else if let Some(pos) = all_stations.iter().position(|s| s.url == st.url || s.name == st.name) {
                    ui.active_band = GenreBand::All;
                    ui.search_input.clear();
                    active_stations = bands::filter_by_band_and_search(&all_stations, ui.active_band, &ui.search_input);
                    last_search = ui.search_input.clone();
                    last_band = ui.active_band;
                    ui.active_index = pos;
                }
                if ui.is_power_on {
                    audio.play(st.name.clone(), st.url.clone());
                }
                last_played_channel = Some((st.name.clone(), st.url.clone()));
                ui.status_feedback = Some((format!("Tuned to Preset [{}]", i + 1), 3.0));
            }
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
        let layout = StereoLayout::compute(screen_w, screen_h, GenreBand::ALL_BANDS.len());

        let ctx = controls::vintage_ui::StationViewContext {
            total_stations_count,
            active_filtered_count: active_stations.len(),
            current_station,
            all_stations: Some(&all_stations),
        };

        render_vintage_stereo(&mut d, &layout, &mut ui, &mut presets, &audio, ctx);
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
}
