use raylib::prelude::Rectangle;

/// Screen orientation mode based on viewport aspect ratio
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Landscape,
    Portrait,
}

/// Calculated UI bounding geometry for all vintage stereo controls and expandable EQ/balance panel.
/// No coordinates are hardcoded; all rectangles are computed proportionally.
#[expect(dead_code)]
#[derive(Debug, Clone)]
pub struct StereoLayout {
    pub orientation: Orientation,
    pub screen_width: f32,
    pub screen_height: f32,
    pub eq_expanded: bool,

    // Main housing / outer bezel
    pub bezel_rect: Rectangle,

    // Top control strip
    pub power_btn_rect: Rectangle,
    pub eq_btn_rect: Rectangle,
    pub display_rect: Rectangle,
    pub vol_label_rect: Rectangle,
    pub vol_slider_rect: Rectangle,

    // Search bar row
    pub search_box_rect: Rectangle,
    pub search_clear_rect: Rectangle,

    // Waveband selector row (buttons for genre bands)
    pub band_btn_rects: Vec<Rectangle>,

    // Frequency tuning dial & controls
    pub dial_track_rect: Rectangle,
    pub coarse_prev_rect: Rectangle,
    pub fine_prev_rect: Rectangle,
    pub fine_next_rect: Rectangle,
    pub coarse_next_rect: Rectangle,

    // 6 Preset push-button slots
    pub preset_rects: [Rectangle; 6],

    // Tuning knob / dial surf area
    pub tune_knob_rect: Rectangle,

    // EQ and Balance Panel (when expanded)
    pub eq_panel_rect: Rectangle,
    pub eq_power_btn_rect: Rectangle,
    pub eq_reset_btn_rect: Rectangle,
    pub eq_band_rects: [Rectangle; 10],
    pub eq_band_label_rects: [Rectangle; 10],
    pub eq_band_val_rects: [Rectangle; 10],
    pub balance_label_rect: Rectangle,
    pub balance_slider_rect: Rectangle,
    pub balance_center_btn_rect: Rectangle,

    // Scaled typography sizes (pixels)
    pub font_display_large: i32,
    pub font_display_small: i32,
    pub font_ui_regular: i32,
    pub font_ui_small: i32,
}

impl StereoLayout {
    /// Find the window height (logical pixels) at which the main player box (bezel)
    /// has exactly `bezel_height` for the given width and EQ expansion state.
    /// The bezel height grows monotonically with window height, so a bisection over
    /// `compute` yields an exact inverse for both landscape and portrait layouts.
    pub fn window_height_for_bezel(
        width: f32,
        bezel_height: f32,
        num_bands: usize,
        eq_expanded: bool,
    ) -> i32 {
        let mut lo = 200.0_f32;
        let mut hi = 10_000.0_f32;
        for _ in 0..60 {
            let mid = (lo + hi) * 0.5;
            let h = Self::compute(width, mid, num_bands, eq_expanded).bezel_rect.height;
            if h < bezel_height {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        hi.round() as i32
    }

    /// Target window height after toggling the EQ/DSP panel: the main player box keeps
    /// its current geometry and only the vertical dimension changes.
    pub fn toggled_window_height(&self, width: f32, num_bands: usize) -> i32 {
        Self::window_height_for_bezel(width, self.bezel_rect.height, num_bands, !self.eq_expanded)
    }

    /// Compute a complete responsive layout from viewport dimensions, number of bands, and EQ expansion state.
    pub fn compute(width: f32, height: f32, num_bands: usize, eq_expanded: bool) -> Self {
        let width = width.max(300.0);
        let height = height.max(200.0);

        let aspect_ratio = width / height;
        let orientation = if aspect_ratio >= 1.15 {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        };

        match orientation {
            Orientation::Landscape => Self::compute_landscape(width, height, num_bands, eq_expanded),
            Orientation::Portrait => Self::compute_portrait(width, height, num_bands, eq_expanded),
        }
    }

    /// Compute landscape dash head-unit layout (Desktop, Web Canvas, Tablet, Phone Landscape)
    fn compute_landscape(width: f32, height: f32, num_bands: usize, eq_expanded: bool) -> Self {
        let margin = (width * 0.012).clamp(6.0, 12.0);
        let margin_x = margin;
        let margin_y = margin;

        let total_avail_h = height - (margin_y * 2.0);
        let (tuner_h, eq_h, module_gap) = if eq_expanded {
            let gap = (height * 0.015).clamp(4.0, 10.0);
            let h_tuner = (total_avail_h - gap) * 0.60;
            let h_eq = total_avail_h - gap - h_tuner;
            (h_tuner, h_eq, gap)
        } else {
            (total_avail_h, 0.0, 0.0)
        };

        let bezel_rect = Rectangle::new(
            margin_x,
            margin_y,
            width - (margin_x * 2.0),
            tuner_h,
        );

        let pad = (bezel_rect.width * 0.015).clamp(8.0, 16.0);
        let pad_x = pad;
        let pad_y = pad;
        let inner_x = bezel_rect.x + pad_x;
        let inner_w = bezel_rect.width - (pad_x * 2.0);
        let inner_y = bezel_rect.y + pad_y;
        let inner_h = (bezel_rect.height - (pad_y * 2.0)).max(80.0);

        // Scaled typography, derived from the main player box (not the full window) so text
        // keeps its size when the EQ panel is expanded or collapsed.
        let type_h = tuner_h + (margin_y * 2.0);
        let font_display_large = ((type_h * 0.055).round() as i32).clamp(15, 26);
        let font_display_small = ((type_h * 0.038).round() as i32).clamp(10, 16);
        let font_ui_regular = ((type_h * 0.040).round() as i32).clamp(11, 18);
        let font_ui_small = ((type_h * 0.034).round() as i32).clamp(9, 14);

        let row_gap = (inner_h * 0.038).clamp(4.0, 10.0);
        let total_gaps = 4.0 * row_gap;
        let available_content_h = (inner_h - total_gaps).max(40.0);

        let row1_h = available_content_h * 0.36;
        let row2_h = available_content_h * 0.13;
        let row3_h = available_content_h * 0.14;
        let row4_h = available_content_h * 0.23;
        let row5_h = row3_h;

        let row1_y = inner_y;
        let row2_y = row1_y + row1_h + row_gap;
        let row3_y = row2_y + row2_h + row_gap;
        let row4_y = row3_y + row3_h + row_gap;
        let row5_y = row4_y + row4_h + row_gap;

        let power_w = (inner_w * 0.09).clamp(50.0, 90.0);
        let eq_btn_w = (inner_w * 0.07).clamp(45.0, 70.0);
        let vol_w = (inner_w * 0.16).clamp(80.0, 150.0);
        let display_gap = inner_w * 0.012;

        let power_btn_rect = Rectangle::new(inner_x, row1_y, power_w, row1_h * 0.7);
        let eq_btn_rect = Rectangle::new(inner_x + power_w + 6.0, row1_y, eq_btn_w, row1_h * 0.7);

        let display_x = eq_btn_rect.x + eq_btn_rect.width + display_gap;
        let vol_x = inner_x + inner_w - vol_w;
        let display_w = (vol_x - display_gap - display_x).max(100.0);

        let display_rect = Rectangle::new(display_x, row1_y, display_w, row1_h);

        let vol_label_rect = Rectangle::new(vol_x, row1_y, vol_w, row1_h * 0.35);
        let vol_slider_rect = Rectangle::new(
            vol_x,
            row1_y + (row1_h * 0.45),
            vol_w,
            (row1_h * 0.45).clamp(16.0, 28.0),
        );

        let clear_w = (inner_w * 0.08).clamp(40.0, 70.0);
        let search_gap = 8.0;
        let search_w = inner_w - clear_w - search_gap;

        let search_box_rect = Rectangle::new(inner_x, row2_y, search_w, row2_h);
        let search_clear_rect =
            Rectangle::new(inner_x + search_w + search_gap, row2_y, clear_w, row2_h);

        let count = num_bands.max(1);
        let band_gap = (inner_w * 0.008).clamp(3.0, 10.0);
        let total_band_gaps = (count - 1) as f32 * band_gap;
        let single_band_w = (inner_w - total_band_gaps) / count as f32;

        let mut band_btn_rects = Vec::with_capacity(count);
        for i in 0..count {
            let bx = inner_x + (i as f32 * (single_band_w + band_gap));
            band_btn_rects.push(Rectangle::new(bx, row3_y, single_band_w, row3_h));
        }

        let step_btn_w = (inner_w * 0.09).clamp(45.0, 75.0);
        let step_btn_h = (row4_h * 0.75).clamp(18.0, 34.0);
        let step_btn_y = row4_y + ((row4_h - step_btn_h) / 2.0);

        let coarse_prev_rect = Rectangle::new(inner_x, step_btn_y, step_btn_w, step_btn_h);
        let fine_prev_rect = Rectangle::new(
            inner_x + step_btn_w + 6.0,
            step_btn_y,
            step_btn_w,
            step_btn_h,
        );

        let coarse_next_rect = Rectangle::new(
            inner_x + inner_w - step_btn_w,
            step_btn_y,
            step_btn_w,
            step_btn_h,
        );
        let fine_next_rect = Rectangle::new(
            coarse_next_rect.x - step_btn_w - 6.0,
            step_btn_y,
            step_btn_w,
            step_btn_h,
        );

        let dial_x = fine_prev_rect.x + fine_prev_rect.width + 8.0;
        let dial_w = (fine_next_rect.x - 8.0 - dial_x).max(100.0);
        let dial_track_rect = Rectangle::new(dial_x, row4_y, dial_w, row4_h);

        let preset_count = 6;
        let preset_gap = (inner_w * 0.008).clamp(3.0, 10.0);
        let total_preset_gaps = (preset_count - 1) as f32 * preset_gap;
        let single_preset_w = (inner_w - total_preset_gaps) / preset_count as f32;

        let mut preset_rects = [Rectangle::default(); 6];
        for (i, slot) in preset_rects.iter_mut().enumerate() {
            let px = inner_x + (i as f32 * (single_preset_w + preset_gap));
            *slot = Rectangle::new(px, row5_y, single_preset_w, row5_h);
        }

        let tune_knob_rect = Rectangle::default();

        // Compute EQ Panel Geometry when expanded
        let (
            eq_panel_rect,
            eq_power_btn_rect,
            eq_reset_btn_rect,
            eq_band_rects,
            eq_band_label_rects,
            eq_band_val_rects,
            balance_label_rect,
            balance_slider_rect,
            balance_center_btn_rect,
        ) = if eq_expanded && eq_h > 30.0 {
            let panel_rect = Rectangle::new(
                margin_x,
                bezel_rect.y + bezel_rect.height + module_gap,
                width - (margin_x * 2.0),
                eq_h,
            );
            let eq_inner_x = panel_rect.x + pad_x;
            let eq_inner_w = panel_rect.width - (pad_x * 2.0);
            let eq_inner_y = panel_rect.y + (pad_y * 0.6);
            let eq_inner_h = (panel_rect.height - (pad_y * 1.2)).max(20.0);

            let ctrl_row_h = (eq_inner_h * 0.26).clamp(18.0, 28.0);
            let eq_pwr = Rectangle::new(eq_inner_x, eq_inner_y, (eq_inner_w * 0.12).clamp(70.0, 100.0), ctrl_row_h);
            let eq_rst = Rectangle::new(eq_pwr.x + eq_pwr.width + 6.0, eq_inner_y, (eq_inner_w * 0.08).clamp(50.0, 70.0), ctrl_row_h);

            let bal_total_w = (eq_inner_w * 0.38).clamp(180.0, 300.0);
            let bal_x = eq_inner_x + eq_inner_w - bal_total_w;
            let bal_ctr_w = (bal_total_w * 0.18).clamp(36.0, 50.0);
            let bal_lbl_w = (bal_total_w * 0.32).clamp(60.0, 100.0);
            let bal_slide_w = bal_total_w - bal_ctr_w - bal_lbl_w - 8.0;

            let bal_lbl = Rectangle::new(bal_x, eq_inner_y, bal_lbl_w, ctrl_row_h);
            let bal_slide = Rectangle::new(bal_x + bal_lbl_w + 4.0, eq_inner_y, bal_slide_w, ctrl_row_h);
            let bal_ctr = Rectangle::new(bal_slide.x + bal_slide.width + 4.0, eq_inner_y, bal_ctr_w, ctrl_row_h);

            let faders_top_y = eq_inner_y + ctrl_row_h + 4.0;
            let faders_total_h = (eq_inner_y + eq_inner_h - faders_top_y).max(20.0);
            let val_lbl_h = (faders_total_h * 0.18).clamp(10.0, 15.0);
            let freq_lbl_h = (faders_total_h * 0.18).clamp(10.0, 15.0);
            let slider_h = (faders_total_h - val_lbl_h - freq_lbl_h).max(10.0);

            let band_gap = (eq_inner_w * 0.012).clamp(4.0, 12.0);
            let total_gaps = band_gap * 9.0;
            let single_w = (eq_inner_w - total_gaps) / 10.0;

            let mut b_rects = [Rectangle::default(); 10];
            let mut l_rects = [Rectangle::default(); 10];
            let mut v_rects = [Rectangle::default(); 10];

            for i in 0..10 {
                let bx = eq_inner_x + (i as f32 * (single_w + band_gap));
                v_rects[i] = Rectangle::new(bx, faders_top_y, single_w, val_lbl_h);
                b_rects[i] = Rectangle::new(bx, faders_top_y + val_lbl_h, single_w, slider_h);
                l_rects[i] = Rectangle::new(bx, faders_top_y + val_lbl_h + slider_h, single_w, freq_lbl_h);
            }

            (
                panel_rect,
                eq_pwr,
                eq_rst,
                b_rects,
                l_rects,
                v_rects,
                bal_lbl,
                bal_slide,
                bal_ctr,
            )
        } else {
            (
                Rectangle::default(),
                Rectangle::default(),
                Rectangle::default(),
                [Rectangle::default(); 10],
                [Rectangle::default(); 10],
                [Rectangle::default(); 10],
                Rectangle::default(),
                Rectangle::default(),
                Rectangle::default(),
            )
        };

        Self {
            orientation: Orientation::Landscape,
            screen_width: width,
            screen_height: height,
            eq_expanded,
            bezel_rect,
            power_btn_rect,
            eq_btn_rect,
            display_rect,
            vol_label_rect,
            vol_slider_rect,
            search_box_rect,
            search_clear_rect,
            band_btn_rects,
            dial_track_rect,
            coarse_prev_rect,
            fine_prev_rect,
            fine_next_rect,
            coarse_next_rect,
            preset_rects,
            tune_knob_rect,
            eq_panel_rect,
            eq_power_btn_rect,
            eq_reset_btn_rect,
            eq_band_rects,
            eq_band_label_rects,
            eq_band_val_rects,
            balance_label_rect,
            balance_slider_rect,
            balance_center_btn_rect,
            font_display_large,
            font_display_small,
            font_ui_regular,
            font_ui_small,
        }
    }

    /// Compute portrait pocket-radio layout (Mobile Phone Portrait)
    fn compute_portrait(width: f32, height: f32, num_bands: usize, eq_expanded: bool) -> Self {
        let margin_x = (width * 0.02).max(6.0);
        let margin_y = (height * 0.015).max(6.0);

        let total_avail_h = height - (margin_y * 2.0);
        let (tuner_h, eq_h, module_gap) = if eq_expanded {
            let gap = 8.0;
            let h_tuner = (total_avail_h - gap) * 0.65;
            let h_eq = total_avail_h - gap - h_tuner;
            (h_tuner, h_eq, gap)
        } else {
            (total_avail_h, 0.0, 0.0)
        };

        let bezel_rect = Rectangle::new(
            margin_x,
            margin_y,
            width - (margin_x * 2.0),
            tuner_h,
        );

        let pad_x = bezel_rect.width * 0.03;
        let inner_x = bezel_rect.x + pad_x;
        let inner_w = bezel_rect.width - (pad_x * 2.0);

        // Scaled typography, derived from the main player box (not the full window) so text
        // keeps its size when the EQ panel is expanded or collapsed.
        // Invert the height-dependent margin (max(1.5% of height, 6px)) to recover the
        // collapsed window height that yields this main player box height.
        let type_h = (tuner_h + 12.0).max(tuner_h / 0.97);
        let font_display_large = ((type_h * 0.028).round() as i32).clamp(13, 24);
        let font_display_small = ((type_h * 0.018).round() as i32).clamp(9, 15);
        let font_ui_regular = ((type_h * 0.022).round() as i32).clamp(11, 18);
        let font_ui_small = ((type_h * 0.018).round() as i32).clamp(9, 14);

        // Top bar: Power button, EQ toggle button, Volume slider
        let top_y = bezel_rect.y + (bezel_rect.height * 0.015);
        let top_h = (tuner_h * 0.05).clamp(26.0, 38.0);
        let power_w = (inner_w * 0.22).clamp(50.0, 80.0);
        let eq_btn_w = (inner_w * 0.18).clamp(40.0, 65.0);
        let vol_w = (inner_w - power_w - eq_btn_w - 12.0).max(60.0);

        let power_btn_rect = Rectangle::new(inner_x, top_y, power_w, top_h);
        let eq_btn_rect = Rectangle::new(inner_x + power_w + 4.0, top_y, eq_btn_w, top_h);
        let vol_label_rect = Rectangle::new(eq_btn_rect.x + eq_btn_w + 6.0, top_y, vol_w * 0.35, top_h);
        let vol_slider_rect = Rectangle::new(
            vol_label_rect.x + vol_label_rect.width + 2.0,
            top_y,
            vol_w * 0.65,
            top_h,
        );

        // Display (approx 16% of height)
        let disp_y = top_y + top_h + (tuner_h * 0.015);
        let disp_h = (tuner_h * 0.18).clamp(60.0, 130.0);
        let display_rect = Rectangle::new(inner_x, disp_y, inner_w, disp_h);

        // Search Bar (approx 5.5% of height)
        let search_y = disp_y + disp_h + (tuner_h * 0.012);
        let search_h = (tuner_h * 0.06).clamp(28.0, 42.0);
        let clear_w = (inner_w * 0.20).clamp(45.0, 70.0);
        let search_w = inner_w - clear_w - 8.0;

        let search_box_rect = Rectangle::new(inner_x, search_y, search_w, search_h);
        let search_clear_rect =
            Rectangle::new(inner_x + search_w + 8.0, search_y, clear_w, search_h);

        // Waveband selector grid (2 rows on mobile)
        let bands_y = search_y + search_h + (tuner_h * 0.012);
        let bands_h = (tuner_h * 0.05).clamp(24.0, 36.0);

        let count = num_bands.max(1);
        let bands_per_row = count.div_ceil(2);
        let single_band_w = (inner_w - (bands_per_row as f32 - 1.0) * 4.0) / bands_per_row as f32;

        let mut band_btn_rects = Vec::with_capacity(count);
        for i in 0..count {
            let row = i / bands_per_row;
            let col = i % bands_per_row;
            let bx = inner_x + (col as f32 * (single_band_w + 4.0));
            let by = bands_y + (row as f32 * (bands_h + 4.0));
            band_btn_rects.push(Rectangle::new(bx, by, single_band_w, bands_h));
        }

        let bands_total_h = if count > bands_per_row {
            (bands_h * 2.0) + 4.0
        } else {
            bands_h
        };

        // Dial Track
        let dial_y = bands_y + bands_total_h + (tuner_h * 0.015);
        let dial_h = (tuner_h * 0.09).clamp(30.0, 50.0);
        let dial_track_rect = Rectangle::new(inner_x, dial_y, inner_w, dial_h);

        // Tuning Step Buttons (4 buttons in a row)
        let tune_btn_y = dial_y + dial_h + (tuner_h * 0.012);
        let tune_btn_h = (tuner_h * 0.06).clamp(26.0, 40.0);
        let step_w = (inner_w - 18.0) / 4.0;

        let coarse_prev_rect = Rectangle::new(inner_x, tune_btn_y, step_w, tune_btn_h);
        let fine_prev_rect = Rectangle::new(inner_x + step_w + 6.0, tune_btn_y, step_w, tune_btn_h);
        let fine_next_rect = Rectangle::new(
            inner_x + (step_w * 2.0) + 12.0,
            tune_btn_y,
            step_w,
            tune_btn_h,
        );
        let coarse_next_rect = Rectangle::new(
            inner_x + (step_w * 3.0) + 18.0,
            tune_btn_y,
            step_w,
            tune_btn_h,
        );

        // Presets 2x3 Grid
        let presets_y = tune_btn_y + tune_btn_h + (tuner_h * 0.015);
        let single_preset_h = bands_h;
        let single_preset_w = (inner_w - 8.0) / 2.0;

        let mut preset_rects = [Rectangle::default(); 6];
        for (i, slot) in preset_rects.iter_mut().enumerate() {
            let row = i / 2;
            let col = i % 2;
            let px = inner_x + (col as f32 * (single_preset_w + 8.0));
            let py = presets_y + (row as f32 * (single_preset_h + 6.0));
            *slot = Rectangle::new(px, py, single_preset_w, single_preset_h);
        }

        let tune_knob_rect = Rectangle::default();

        // Compute EQ Panel Geometry when expanded in Portrait
        let (
            eq_panel_rect,
            eq_power_btn_rect,
            eq_reset_btn_rect,
            eq_band_rects,
            eq_band_label_rects,
            eq_band_val_rects,
            balance_label_rect,
            balance_slider_rect,
            balance_center_btn_rect,
        ) = if eq_expanded && eq_h > 30.0 {
            let panel_rect = Rectangle::new(
                margin_x,
                bezel_rect.y + bezel_rect.height + module_gap,
                width - (margin_x * 2.0),
                eq_h,
            );
            let eq_inner_x = panel_rect.x + pad_x;
            let eq_inner_w = panel_rect.width - (pad_x * 2.0);
            let eq_inner_y = panel_rect.y + 6.0;
            let eq_inner_h = (panel_rect.height - 12.0).max(20.0);

            let ctrl_row_h = (eq_inner_h * 0.24).clamp(18.0, 26.0);
            let eq_pwr = Rectangle::new(eq_inner_x, eq_inner_y, (eq_inner_w * 0.22).clamp(60.0, 85.0), ctrl_row_h);
            let eq_rst = Rectangle::new(eq_pwr.x + eq_pwr.width + 4.0, eq_inner_y, (eq_inner_w * 0.16).clamp(40.0, 60.0), ctrl_row_h);

            let bal_x = eq_rst.x + eq_rst.width + 6.0;
            let bal_rem_w = (eq_inner_x + eq_inner_w - bal_x).max(50.0);
            let bal_ctr_w = 34.0;
            let bal_slide_w = (bal_rem_w - bal_ctr_w - 4.0).max(30.0);

            let bal_lbl = Rectangle::new(bal_x, eq_inner_y, 0.0, 0.0);
            let bal_slide = Rectangle::new(bal_x, eq_inner_y, bal_slide_w, ctrl_row_h);
            let bal_ctr = Rectangle::new(bal_slide.x + bal_slide.width + 4.0, eq_inner_y, bal_ctr_w, ctrl_row_h);

            let faders_top_y = eq_inner_y + ctrl_row_h + 4.0;
            let faders_total_h = (eq_inner_y + eq_inner_h - faders_top_y).max(20.0);
            let val_lbl_h = (faders_total_h * 0.18).clamp(8.0, 13.0);
            let freq_lbl_h = (faders_total_h * 0.18).clamp(8.0, 13.0);
            let slider_h = (faders_total_h - val_lbl_h - freq_lbl_h).max(10.0);

            let band_gap = (eq_inner_w * 0.008).clamp(2.0, 6.0);
            let total_gaps = band_gap * 9.0;
            let single_w = (eq_inner_w - total_gaps) / 10.0;

            let mut b_rects = [Rectangle::default(); 10];
            let mut l_rects = [Rectangle::default(); 10];
            let mut v_rects = [Rectangle::default(); 10];

            for i in 0..10 {
                let bx = eq_inner_x + (i as f32 * (single_w + band_gap));
                v_rects[i] = Rectangle::new(bx, faders_top_y, single_w, val_lbl_h);
                b_rects[i] = Rectangle::new(bx, faders_top_y + val_lbl_h, single_w, slider_h);
                l_rects[i] = Rectangle::new(bx, faders_top_y + val_lbl_h + slider_h, single_w, freq_lbl_h);
            }

            (
                panel_rect,
                eq_pwr,
                eq_rst,
                b_rects,
                l_rects,
                v_rects,
                bal_lbl,
                bal_slide,
                bal_ctr,
            )
        } else {
            (
                Rectangle::default(),
                Rectangle::default(),
                Rectangle::default(),
                [Rectangle::default(); 10],
                [Rectangle::default(); 10],
                [Rectangle::default(); 10],
                Rectangle::default(),
                Rectangle::default(),
                Rectangle::default(),
            )
        };

        Self {
            orientation: Orientation::Portrait,
            screen_width: width,
            screen_height: height,
            eq_expanded,
            bezel_rect,
            power_btn_rect,
            eq_btn_rect,
            display_rect,
            vol_label_rect,
            vol_slider_rect,
            search_box_rect,
            search_clear_rect,
            band_btn_rects,
            dial_track_rect,
            coarse_prev_rect,
            fine_prev_rect,
            fine_next_rect,
            coarse_next_rect,
            preset_rects,
            tune_knob_rect,
            eq_panel_rect,
            eq_power_btn_rect,
            eq_reset_btn_rect,
            eq_band_rects,
            eq_band_label_rects,
            eq_band_val_rects,
            balance_label_rect,
            balance_slider_rect,
            balance_center_btn_rect,
            font_display_large,
            font_display_small,
            font_ui_regular,
            font_ui_small,
        }
    }

    /// Calculate needle position on dial track from normalized progress [0.0, 1.0]
    pub fn needle_x(&self, progress: f32) -> f32 {
        let pad = 10.0;
        let usable_w = self.dial_track_rect.width - (pad * 2.0);
        self.dial_track_rect.x + pad + (usable_w * progress.clamp(0.0, 1.0))
    }

    /// Convert mouse X coordinate on dial track to normalized progress [0.0, 1.0]
    pub fn needle_progress_from_x(&self, mouse_x: f32) -> f32 {
        let pad = 10.0;
        let usable_w = self.dial_track_rect.width - (pad * 2.0);
        if usable_w <= 0.0 {
            return 0.5;
        }
        ((mouse_x - (self.dial_track_rect.x + pad)) / usable_w).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_landscape_layout_sanity() {
        let layout = StereoLayout::compute(1280.0, 720.0, 8, false);
        assert_eq!(layout.orientation, Orientation::Landscape);
        assert_eq!(layout.band_btn_rects.len(), 8);
        assert_eq!(layout.preset_rects.len(), 6);

        // Sanity check coordinates are non-negative and finite
        assert!(layout.display_rect.x > 0.0);
        assert!(layout.display_rect.y > 0.0);
        assert!(layout.display_rect.width > 300.0);
        assert!(layout.display_rect.height > 50.0);

        // Presets should be distributed across width
        assert!(layout.preset_rects[0].x < layout.preset_rects[1].x);
        assert!(layout.preset_rects[4].x < layout.preset_rects[5].x);

        // Dial needle calculation
        let left_needle = layout.needle_x(0.0);
        let mid_needle = layout.needle_x(0.5);
        let right_needle = layout.needle_x(1.0);
        assert!(left_needle < mid_needle);
        assert!(mid_needle < right_needle);

        let prog = layout.needle_progress_from_x(
            layout.dial_track_rect.x + (layout.dial_track_rect.width * 0.5),
        );
        assert!((prog - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_landscape_margins_and_gaps_symmetry() {
        let width = 920.0;
        let height = 270.0;
        let layout = StereoLayout::compute(width, height, 8, false);

        // Verify outer margins around bezel are equal
        let outer_left = layout.bezel_rect.x;
        let outer_top = layout.bezel_rect.y;
        let outer_right = width - (layout.bezel_rect.x + layout.bezel_rect.width);
        let outer_bottom = height - (layout.bezel_rect.y + layout.bezel_rect.height);

        assert!((outer_left - outer_top).abs() < 0.01);
        assert!((outer_left - outer_right).abs() < 0.01);
        assert!((outer_left - outer_bottom).abs() < 0.01);

        // Verify inner padding inside bezel (top, bottom, left, right) is equal
        let inner_left = layout.power_btn_rect.x - layout.bezel_rect.x;
        let inner_top = layout.power_btn_rect.y - layout.bezel_rect.y;
        let inner_right = (layout.bezel_rect.x + layout.bezel_rect.width)
            - (layout.vol_slider_rect.x + layout.vol_slider_rect.width);
        let inner_bottom = (layout.bezel_rect.y + layout.bezel_rect.height)
            - (layout.preset_rects[0].y + layout.preset_rects[0].height);

        assert!((inner_top - inner_bottom).abs() < 0.01);
        assert!((inner_left - inner_right).abs() < 0.01);
        assert!((inner_top - inner_left).abs() < 0.01);
    }

    #[test]
    fn test_expanded_eq_panel_layout() {
        let width = 920.0;
        let height = 440.0;
        let layout = StereoLayout::compute(width, height, 8, true);

        assert!(layout.eq_expanded);
        assert!(layout.eq_panel_rect.height > 80.0);
        assert!(layout.eq_power_btn_rect.width > 50.0);
        assert!(layout.balance_slider_rect.width > 50.0);
        assert_eq!(layout.eq_band_rects.len(), 10);
        assert!(layout.eq_band_rects[0].x < layout.eq_band_rects[9].x);
    }

    #[test]
    fn test_portrait_layout_sanity() {
        let layout = StereoLayout::compute(390.0, 844.0, 8, false); // iPhone 14 dimensions
        assert_eq!(layout.orientation, Orientation::Portrait);

        assert!(layout.display_rect.width > 200.0);
        assert!(layout.preset_rects[0].width > 100.0);
        assert_eq!(
            layout.preset_rects[0].height,
            layout.band_btn_rects[0].height
        );
        assert!(layout.preset_rects[5].y > layout.preset_rects[0].y);
    }

    #[test]
    fn test_fonts_unchanged_when_eq_toggled() {
        for &(w, h) in &[(920.0_f32, 270.0_f32), (1280.0, 500.0), (390.0, 844.0)] {
            let collapsed = StereoLayout::compute(w, h, 9, false);
            let exp_h = collapsed.toggled_window_height(w, 9);
            let expanded = StereoLayout::compute(w, exp_h as f32, 9, true);
            assert_eq!(expanded.font_display_large, collapsed.font_display_large);
            assert_eq!(expanded.font_display_small, collapsed.font_display_small);
            assert_eq!(expanded.font_ui_regular, collapsed.font_ui_regular);
            assert_eq!(expanded.font_ui_small, collapsed.font_ui_small);
        }
    }

    #[test]
    fn test_small_window_resilience() {
        let layout = StereoLayout::compute(320.0, 240.0, 6, false);
        assert!(layout.display_rect.width > 0.0);
        assert!(layout.display_rect.height > 0.0);
        assert!(layout.font_display_large >= 10);
    }
}
