use radiobrowser::{ApiStation, ApiTag, blocking::RadioBrowserAPI};
use raylib::prelude::*;

use crate::api::{CachedStation, stations_to_string};
mod api;
mod controls;

// Background color for the window,  rgb (25, 25, 25) is a dark gray color
const BACKGROUND_COLOR: Color = Color::new(25, 25, 35, 255);
const TEXT_COLOR: Color = Color::DARKORCHID;

fn main() {

    let mut rb_api = RadioBrowserAPI::new().expect("Failed to create RadioBrowserAPI");
    let stations_cache = api::get_stations_with_cache(&mut rb_api)
        .unwrap_or(vec![CachedStation {
            stationuuid: String::new(),
            name: "Failed To Load Stations".to_string(),
            url: String::new(),
            tags: String::new(),
        }]);

    let mut search_term = String::new();
    search_term.reserve(16);
    let mut old_search_term = search_term.clone();

    let mut stations_string = "nothing here yet".to_string();

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
        controls::search_box::build(&mut d, 10.0, 10.0, 100.0, 20.0, &mut search_term);

        if !old_search_term.eq(&search_term) {
            let stations = api::filter_stations(search_term.as_str(), &stations_cache);
            stations_string = stations_to_string(&stations);
            old_search_term = search_term.clone();
        }

        d.draw_text(stations_string.as_str(), 10, 70, 30, TEXT_COLOR);
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
