use raylib::drawing::RaylibDrawHandle;
use raylib::prelude::*;

pub fn build(
    d: &mut RaylibDrawHandle<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    search_box: &mut String,
) {
    d.gui_set_style(
        GuiControl::TEXTBOX,
        GuiControlProperty::TEXT_ALIGNMENT,
        raylib::ffi::GuiTextAlignment::TEXT_ALIGN_LEFT as i32,
    );
    d.gui_text_box(Rectangle::new(x, y, w, h), search_box, true);
}
