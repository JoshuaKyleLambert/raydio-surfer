use radiobrowser::{ApiStation, ApiTag, blocking::RadioBrowserAPI};
use raylib::prelude::*;
pub mod controls;

// Background color for the window,  rgb (25, 25, 25) is a dark gray color
const BACKGROUND_COLOR: Color = Color::new(25, 25, 35, 255);
const _TEXT_COLOR: Color = Color::DARKORCHID;

fn main() {
    //let station_list = get_stations();
    // Initialize the window using the raylib builder pattern
    let mut search_term = "  Search Terms  ".to_string();

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
        //d.draw_text("Internet Radio", 3, 3, 30, TEXT_COLOR);
        d.gui_group_box(Rectangle::new(5.0, 5.0, 790.0, 440.0), "Internet Radio");
        controls::search_box::build(&mut d, &mut search_term);
    }
}

fn _get_tags() -> Option<Vec<ApiTag>> {
    RadioBrowserAPI::new().ok()?.get_tags().send().ok()
}

fn _get_stations() -> Option<Vec<ApiStation>> {
    RadioBrowserAPI::new().ok()?.get_stations().send().ok()
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
