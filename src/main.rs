use crate::api::stations_to_string;
use crate::audio::{AudioController, PlayerStatus};
use radiobrowser::{ApiStation, ApiTag, blocking::RadioBrowserAPI};
use raylib::prelude::*;
mod api;
mod audio;
mod controls;

// Background color for the window,  rgb (25, 25, 25) is a dark gray color
const BACKGROUND_COLOR: Color = Color::new(25, 25, 35, 255);
const TEXT_COLOR: Color = Color::DARKORCHID;

fn main() {
    let mut rb_api = RadioBrowserAPI::new().expect("Failed to create RadioBrowserAPI");
    let stations_cache = api::get_stations_with_cache(&mut rb_api);
    let mut search_term = String::with_capacity(16);
    let mut old_search_term = search_term.clone();
    let mut filtered_stations = api::filter_stations("", &stations_cache);
    let mut stations_string = stations_to_string(&filtered_stations);

    let audio = AudioController::new();
    let mut volume = audio.volume();

    let (mut rl, thread) = raylib::init()
        .size(800, 450)
        .title("RaydioSurfer")
        // .undecorated()
        // .borderless_windowed_mode()
        .build();

    rl.set_target_fps(60);
    // Main game loop
    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(BACKGROUND_COLOR);

        d.gui_group_box(Rectangle::new(5.0, 5.0, 790.0, 440.0), "Internet Radio");
        controls::search_box::build(&mut d, 15.0, 15.0, 180.0, 24.0, &mut search_term);

        // Only update the stations string if the search term has changed
        if !old_search_term.eq(&search_term) {
            filtered_stations = api::filter_stations(search_term.as_str(), &stations_cache);
            stations_string = stations_to_string(&filtered_stations);
            old_search_term = search_term.clone();
        }

        // Quick Play button to play the first matched station
        let has_station = !filtered_stations.is_empty();
        if d.gui_button(Rectangle::new(205.0, 15.0, 90.0, 24.0), "#131#Play First") && has_station {
            let station = &filtered_stations[0];
            audio.play(station.name.clone(), station.url.clone());
        }

        // Stop button
        if d.gui_button(Rectangle::new(305.0, 15.0, 70.0, 24.0), "#133#Stop") {
            audio.stop();
        }

        // Volume slider
        let prev_vol = volume;
        d.gui_slider(
            Rectangle::new(430.0, 17.0, 90.0, 20.0),
            "Vol",
            "",
            &mut volume,
            0.0,
            1.0,
        );
        if (volume - prev_vol).abs() > 0.01 {
            audio.set_volume(volume);
        }

        // Display current audio playback status
        let status_text = match audio.status() {
            PlayerStatus::Stopped => "Status: Stopped".to_string(),
            PlayerStatus::Connecting => "Status: Connecting...".to_string(),
            PlayerStatus::Playing(ref name) => format!("Status: Playing [{}]", name),
            PlayerStatus::Error(ref err) => format!("Status: Error [{}]", err),
        };
        d.draw_text(&status_text, 540, 20, 13, Color::RAYWHITE);

        // Display station list
        d.draw_text(stations_string.as_str(), 15, 60, 13, TEXT_COLOR);
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
