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

    let (mut rl, thread) = raylib::init()
        .size(860, 480)
        .title("RaydioSurfer - Vintage Internet Radio")
        .resizable()
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

        // Clamp active index
        if !active_stations.is_empty() && ui.active_index >= active_stations.len() {
            ui.active_index = active_stations.len() - 1;
        }

        let current_station = if !active_stations.is_empty() {
            Some(&active_stations[ui.active_index])
        } else {
            None
        };

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
                if ui.is_muted {
                    ui.is_muted = false;
                    audio.set_volume(ui.volume);
                }
                audio.play(st.name.clone(), st.url.clone());
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

        // Enter key to play currently tuned station
        if rl.is_key_pressed(KeyboardKey::KEY_ENTER)
            && let Some(st) = current_station
        {
            if ui.is_muted {
                ui.is_muted = false;
                audio.set_volume(ui.volume);
            }
            audio.play(st.name.clone(), st.url.clone());
        }

        // M key for mute / play toggle
        if rl.is_key_pressed(KeyboardKey::KEY_M) {
            let is_playing_or_connecting = matches!(
                audio.status(),
                crate::audio::PlayerStatus::Playing(_) | crate::audio::PlayerStatus::Connecting
            );
            if ui.is_muted {
                ui.is_muted = false;
                audio.set_volume(ui.volume);
                if !is_playing_or_connecting
                    && let Some(st) = current_station
                {
                    audio.play(st.name.clone(), st.url.clone());
                }
            } else if !is_playing_or_connecting {
                audio.set_volume(ui.volume);
                if let Some(st) = current_station {
                    audio.play(st.name.clone(), st.url.clone());
                }
            } else {
                ui.is_muted = true;
                audio.set_volume(0.0);
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
    // A dummy test that always passes to verify your environment works
    #[test]
    fn test_environment_initialization() {
        let execution_status = true;
        assert!(execution_status);
    }
}
