use raylib::drawing::RaylibDrawHandle;
use raylib::prelude::*;

pub fn build(d: &mut RaylibDrawHandle<'_>, search_box: &mut String) {
    d.gui_set_style(
        GuiControl::TEXTBOX,
        GuiControlProperty::TEXT_ALIGNMENT,
        raylib::ffi::GuiTextAlignment::TEXT_ALIGN_CENTER as i32,
    );
    d.gui_set_style(
        GuiControl::TEXTBOX,
        GuiControlProperty::BASE_COLOR_NORMAL,
        Color::new(25, 25, 30, 255).color_to_int(),
    );
    d.gui_text_box(Rectangle::new(10.0, 10.0, 200.0, 25.0), search_box, true);
}
