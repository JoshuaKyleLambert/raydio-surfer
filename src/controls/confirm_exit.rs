use raylib::ffi::Color;
use raylib::prelude::*;

fn _build(
    d: &mut RaylibDrawHandle<'_>,
    show: &mut bool,
    exit: &mut bool,
    screen_w: i32,
    screen_h: i32,
) {
    if *show {
        d.draw_rectangle(0, 0, screen_w, screen_h, Color::RAYWHITE.alpha(0.8));
        let result = d.gui_message_box(
            Rectangle::new(
                screen_w as f32 / 2.0 - 125.0,
                screen_h as f32 / 2.0 - 50.0,
                250.0,
                100.0,
            ),
            "#159#Close Window",
            "Do you really want to exit?",
            "Yes;No",
        );

        match result {
            0 | 2 => *show = false,
            1 => *exit = true,
            _ => {}
        }
    }
}
