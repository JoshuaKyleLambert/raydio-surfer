use crate::api::CachedStation;
use crate::audio::{AudioController, PlayerStatus};
use crate::bands::GenreBand;
use crate::layout::StereoLayout;
use crate::presets::Presets;
use raylib::drawing::RaylibDrawHandle;
use raylib::prelude::*;

// Vintage Color Palette
const COLOR_CHASSIS_BG: Color = Color::new(22, 22, 28, 255);
const COLOR_BEZEL_OUTLINE: Color = Color::new(45, 45, 58, 255);
const COLOR_VFD_GLASS_BG: Color = Color::new(10, 24, 28, 255);
const COLOR_VFD_GLASS_BORDER: Color = Color::new(18, 48, 56, 255);
const COLOR_VFD_CYAN_GLOW: Color = Color::new(50, 240, 220, 255);
const COLOR_VFD_CYAN_DIM: Color = Color::new(25, 120, 110, 255);
const COLOR_VFD_AMBER: Color = Color::new(255, 185, 45, 255);
const COLOR_NEEDLE_RED: Color = Color::new(255, 60, 45, 255);
const COLOR_DIAL_TRACK_BG: Color = Color::new(14, 14, 20, 255);
const COLOR_DIAL_TICK: Color = Color::new(90, 90, 110, 255);
const COLOR_DIAL_TICK_TEXT: Color = Color::new(130, 130, 155, 255);

pub struct VintageUiState {
    pub search_input: String,
    pub active_band: GenreBand,
    pub active_index: usize,
    pub is_muted: bool,
    pub volume: f32,
    pub status_feedback: Option<(String, f32)>, // Feedback text and timer
}

impl VintageUiState {
    pub fn new(initial_volume: f32) -> Self {
        Self {
            search_input: String::with_capacity(32),
            active_band: GenreBand::All,
            active_index: 0,
            is_muted: false,
            volume: initial_volume,
            status_feedback: None,
        }
    }
}

pub struct StationViewContext<'a> {
    pub total_stations_count: usize,
    pub active_filtered_count: usize,
    pub current_station: Option<&'a CachedStation>,
}

pub fn render_vintage_stereo(
    d: &mut RaylibDrawHandle<'_>,
    layout: &StereoLayout,
    ui: &mut VintageUiState,
    presets: &mut Presets,
    audio: &AudioController,
    ctx: StationViewContext<'_>,
) {
    let current_station_name = ctx.current_station.map(|s| s.name.as_str()).unwrap_or("");
    let current_station_url = ctx.current_station.map(|s| s.url.as_str()).unwrap_or("");
    let active_filtered_count = ctx.active_filtered_count;
    let total_stations_count = ctx.total_stations_count;

    // 1. Draw outer chassis & bezel
    d.draw_rectangle_rounded(layout.bezel_rect, 0.04, 8, COLOR_CHASSIS_BG);
    d.draw_rectangle_rounded_lines(layout.bezel_rect, 0.04, 8, COLOR_BEZEL_OUTLINE);

    // 2. Draw Backlit Glass Display (VFD cyan / amber glow)
    d.draw_rectangle_rounded(layout.display_rect, 0.06, 6, COLOR_VFD_GLASS_BG);
    d.draw_rectangle_rounded_lines(layout.display_rect, 0.06, 6, COLOR_VFD_GLASS_BORDER);

    // Display Top Line: Active Band + Live Scope Counter + Stereo Indicator
    let scope_text = format!(
        "BAND: [{}]  |  TUNED: {} / {} (TOTAL: {})",
        ui.active_band.label(),
        if active_filtered_count > 0 {
            ui.active_index + 1
        } else {
            0
        },
        active_filtered_count,
        total_stations_count
    );
    let disp_x = (layout.display_rect.x + 12.0) as i32;
    let disp_y = (layout.display_rect.y + 8.0) as i32;
    d.draw_text(
        &scope_text,
        disp_x,
        disp_y,
        layout.font_display_small,
        COLOR_VFD_CYAN_DIM,
    );

    // Right-aligned STEREO badge
    let stereo_text = "[STEREO]";
    let stereo_x = (layout.display_rect.x + layout.display_rect.width - 70.0) as i32;
    d.draw_text(
        stereo_text,
        stereo_x,
        disp_y,
        layout.font_display_small,
        COLOR_VFD_AMBER,
    );

    // Display Middle Line: Station Title
    let main_title = if active_filtered_count > 0 {
        current_station_name
    } else {
        "NO STATIONS FOUND FOR SEARCH"
    };
    let title_y = disp_y + layout.font_display_small + 4;
    d.draw_text(
        main_title,
        disp_x,
        title_y,
        layout.font_display_large,
        COLOR_VFD_CYAN_GLOW,
    );

    // Display Bottom Line: Audio playback status or feedback message
    let status_y = title_y + layout.font_display_large + 3;
    if let Some((ref feedback, _)) = ui.status_feedback {
        d.draw_text(
            feedback,
            disp_x,
            status_y,
            layout.font_display_small,
            COLOR_VFD_AMBER,
        );
    } else {
        let player_status = audio.status();
        let (status_str, status_color) = match player_status {
            PlayerStatus::Stopped => ("STATUS: [STOPPED]", COLOR_VFD_CYAN_DIM),
            PlayerStatus::Connecting => ("STATUS: [CONNECTING / BUFFERING...]", COLOR_VFD_AMBER),
            PlayerStatus::Playing(_) => ("STATUS: [LIVE BROADCASTING]", COLOR_VFD_CYAN_GLOW),
            PlayerStatus::Error(ref err) => (err.as_str(), COLOR_NEEDLE_RED),
        };
        d.draw_text(
            status_str,
            disp_x,
            status_y,
            layout.font_display_small,
            status_color,
        );
    }

    // 3. Draw Power / Mute Button
    let power_label = if ui.is_muted {
        "#133#UNMUTE"
    } else {
        "#131#MUTE"
    };
    if d.gui_button(layout.power_btn_rect, power_label) {
        ui.is_muted = !ui.is_muted;
        if ui.is_muted {
            audio.set_volume(0.0);
        } else {
            audio.set_volume(ui.volume);
        }
    }

    // 4. Draw Volume Slider
    d.draw_text(
        &format!("VOL: {}%", (ui.volume * 100.0).round() as i32),
        layout.vol_label_rect.x as i32,
        layout.vol_label_rect.y as i32,
        layout.font_ui_small,
        Color::RAYWHITE,
    );
    let prev_vol = ui.volume;
    d.gui_slider(layout.vol_slider_rect, "", "", &mut ui.volume, 0.0, 1.0);
    if (ui.volume - prev_vol).abs() > 0.01 && !ui.is_muted {
        audio.set_volume(ui.volume);
    }

    // 5. Search Bar & Clear Button
    d.gui_set_style(
        GuiControl::TEXTBOX,
        GuiControlProperty::TEXT_ALIGNMENT,
        raylib::ffi::GuiTextAlignment::TEXT_ALIGN_LEFT as i32,
    );
    d.gui_text_box(layout.search_box_rect, &mut ui.search_input, true);
    if d.gui_button(layout.search_clear_rect, "#113#Clear") {
        ui.search_input.clear();
        ui.active_index = 0;
    }

    // 6. Waveband Push-Buttons
    for (idx, &band) in GenreBand::ALL_BANDS.iter().enumerate() {
        if idx < layout.band_btn_rects.len() {
            let rect = layout.band_btn_rects[idx];
            let is_active = ui.active_band == band;
            let label = if is_active {
                format!("#112#{}", band.label())
            } else {
                band.label().to_string()
            };

            if d.gui_button(rect, &label) {
                ui.active_band = band;
                ui.active_index = 0;
            }
        }
    }

    // 7. Frequency Dial & Sweeping Needle
    d.draw_rectangle_rounded(layout.dial_track_rect, 0.1, 4, COLOR_DIAL_TRACK_BG);
    d.draw_rectangle_rounded_lines(layout.dial_track_rect, 0.1, 4, COLOR_BEZEL_OUTLINE);

    // Draw Frequency Calibration Ticks (88, 92, 96, 100, 104, 108 MHz)
    let freq_labels = ["88", "92", "96", "100", "104", "108"];
    for (i, lbl) in freq_labels.iter().enumerate() {
        let frac = i as f32 / (freq_labels.len() - 1) as f32;
        let tx = layout.dial_track_rect.x + (layout.dial_track_rect.width * frac);
        let ty1 = layout.dial_track_rect.y + 4.0;
        let ty2 = layout.dial_track_rect.y + 12.0;

        d.draw_line(
            tx as i32,
            ty1 as i32,
            tx as i32,
            ty2 as i32,
            COLOR_DIAL_TICK,
        );
        d.draw_text(
            lbl,
            (tx - 8.0) as i32,
            (layout.dial_track_rect.y + layout.dial_track_rect.height - 14.0) as i32,
            layout.font_ui_small,
            COLOR_DIAL_TICK_TEXT,
        );
    }

    // Interactive Dial Tap / Drag to Jump
    let mouse_pos = d.get_mouse_position();
    let is_mouse_down = d.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);
    let dial_hovered = layout.dial_track_rect.check_collision_point_rec(mouse_pos);

    if dial_hovered && is_mouse_down && active_filtered_count > 0 {
        let prog = layout.needle_progress_from_x(mouse_pos.x);
        let new_idx = ((prog * (active_filtered_count - 1) as f32).round() as usize)
            .min(active_filtered_count - 1);
        if new_idx != ui.active_index {
            ui.active_index = new_idx;
        }
    }

    // Draw Orange/Red Needle
    let progress = if active_filtered_count > 1 {
        ui.active_index as f32 / (active_filtered_count - 1) as f32
    } else {
        0.5
    };
    let needle_x = layout.needle_x(progress);
    d.draw_line_ex(
        Vector2::new(needle_x, layout.dial_track_rect.y + 2.0),
        Vector2::new(
            needle_x,
            layout.dial_track_rect.y + layout.dial_track_rect.height - 2.0,
        ),
        2.5,
        COLOR_NEEDLE_RED,
    );
    // Needle tip glow
    d.draw_circle(
        needle_x as i32,
        (layout.dial_track_rect.y + (layout.dial_track_rect.height / 2.0)) as i32,
        3.5,
        COLOR_VFD_AMBER,
    );

    // 8. Coarse & Fine Step Buttons
    if d.gui_button(layout.coarse_prev_rect, "<< 100") && active_filtered_count > 0 {
        ui.active_index = ui.active_index.saturating_sub(100);
    }
    if d.gui_button(layout.fine_prev_rect, "< TUNE") && active_filtered_count > 0 {
        ui.active_index = ui.active_index.saturating_sub(1);
    }
    if d.gui_button(layout.fine_next_rect, "TUNE >")
        && active_filtered_count > 0
        && ui.active_index + 1 < active_filtered_count
    {
        ui.active_index += 1;
    }
    if d.gui_button(layout.coarse_next_rect, "100 >>") && active_filtered_count > 0 {
        ui.active_index = (ui.active_index + 100).min(active_filtered_count - 1);
    }

    // 9. Six Preset Push Buttons
    for i in 0..6 {
        let preset_rect = layout.preset_rects[i];
        let preset_station = presets.get_preset(i);
        let btn_label = if let Some(st) = preset_station {
            format!(
                "[{}] {}",
                i + 1,
                st.name.chars().take(10).collect::<String>()
            )
        } else {
            format!("[{}] <Empty>", i + 1)
        };

        let hovered = preset_rect.check_collision_point_rec(mouse_pos);

        // Left Click -> Recall Preset
        if d.gui_button(preset_rect, &btn_label) {
            if let Some(st) = preset_station {
                audio.play(st.name.clone(), st.url.clone());
                ui.status_feedback = Some((format!("Tuned to Preset [{}]", i + 1), 3.0));
            } else {
                ui.status_feedback = Some((
                    format!("Preset [{}] is empty. Right-click to save.", i + 1),
                    3.0,
                ));
            }
        }

        // Right Click -> Save Current Station to Preset
        if hovered
            && d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT)
            && active_filtered_count > 0
        {
            let st = CachedStation {
                stationuuid: String::new(),
                name: current_station_name.to_string(),
                url: current_station_url.to_string(),
                tags: String::new(),
            };
            presets.set_preset(i, st);
            ui.status_feedback = Some((format!("Saved to Preset [{}]!", i + 1), 3.0));
        }
    }

    // 10. Quick Play/Pause Action: Space or Double-Click
    if d.is_key_pressed(KeyboardKey::KEY_SPACE) && active_filtered_count > 0 {
        match audio.status() {
            PlayerStatus::Playing(_) | PlayerStatus::Connecting => audio.stop(),
            _ => audio.play(
                current_station_name.to_string(),
                current_station_url.to_string(),
            ),
        }
    }
}
