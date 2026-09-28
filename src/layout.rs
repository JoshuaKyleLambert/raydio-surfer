use raylib::prelude::Rectangle;

/// Screen orientation mode based on viewport aspect ratio
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Landscape,
    Portrait,
}

/// Calculated UI bounding geometry for all vintage stereo controls.
/// No coordinates are hardcoded; all rectangles are computed proportionally.
#[expect(dead_code)]
#[derive(Debug, Clone)]
pub struct StereoLayout {
    pub orientation: Orientation,
    pub screen_width: f32,
    pub screen_height: f32,

    // Main housing / outer bezel
    pub bezel_rect: Rectangle,

    // Top control strip
    pub power_btn_rect: Rectangle,
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

    // Scaled typography sizes (pixels)
    pub font_display_large: i32,
    pub font_display_small: i32,
    pub font_ui_regular: i32,
    pub font_ui_small: i32,
}

impl StereoLayout {
    /// Compute a complete responsive layout from viewport dimensions and number of bands.
    pub fn compute(width: f32, height: f32, num_bands: usize) -> Self {
        let width = width.max(300.0);
        let height = height.max(200.0);

        let aspect_ratio = width / height;
        let orientation = if aspect_ratio >= 1.15 {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        };

        match orientation {
            Orientation::Landscape => Self::compute_landscape(width, height, num_bands),
            Orientation::Portrait => Self::compute_portrait(width, height, num_bands),
        }
    }

    /// Compute landscape dash head-unit layout (Desktop, Web Canvas, Tablet, Phone Landscape)
    fn compute_landscape(width: f32, height: f32, num_bands: usize) -> Self {
        let margin = (width * 0.012).clamp(6.0, 12.0);
        let margin_x = margin;
        let margin_y = margin;

        let bezel_rect = Rectangle::new(
            margin_x,
            margin_y,
            width - (margin_x * 2.0),
            height - (margin_y * 2.0),
        );

        let pad = (bezel_rect.width * 0.015).clamp(8.0, 16.0);
        let pad_x = pad;
        let pad_y = pad;
        let inner_x = bezel_rect.x + pad_x;
        let inner_w = bezel_rect.width - (pad_x * 2.0);
        let inner_y = bezel_rect.y + pad_y;
        let inner_h = (bezel_rect.height - (pad_y * 2.0)).max(100.0);

        // Scaled typography
        let font_display_large = ((height * 0.065).round() as i32).clamp(16, 28);
        let font_display_small = ((height * 0.042).round() as i32).clamp(11, 18);
        let font_ui_regular = ((height * 0.045).round() as i32).clamp(12, 20);
        let font_ui_small = ((height * 0.038).round() as i32).clamp(10, 15);

        let row_gap = (inner_h * 0.038).clamp(6.0, 12.0);
        let total_gaps = 4.0 * row_gap;
        let available_content_h = (inner_h - total_gaps).max(40.0);

        // Proportional row distribution summing to 1.0 (with row5_h == row3_h):
        // Row 1 (Display/Power/Vol): 36%
        // Row 2 (Search Bar): 13%
        // Row 3 (Search Presets / Bands): 14%
        // Row 4 (Dial Scale / Step Buttons): 23%
        // Row 5 (Preset Station Buttons): 14% (= Row 3 height)
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
        let vol_w = (inner_w * 0.16).clamp(80.0, 150.0);
        let display_gap = inner_w * 0.015;

        let power_btn_rect = Rectangle::new(inner_x, row1_y, power_w, row1_h * 0.7);

        let display_x = power_btn_rect.x + power_btn_rect.width + display_gap;
        let vol_x = inner_x + inner_w - vol_w;
        let display_w = (vol_x - display_gap - display_x).max(120.0);

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
        let step_btn_h = (row4_h * 0.75).clamp(20.0, 34.0);
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
            coarse_next_rect.x - 6.0 - step_btn_w,
            step_btn_y,
            step_btn_w,
            step_btn_h,
        );

        let dial_x = fine_prev_rect.x + fine_prev_rect.width + 12.0;
        let dial_w = (fine_next_rect.x - 12.0 - dial_x).max(60.0);
        let dial_track_rect = Rectangle::new(dial_x, row4_y, dial_w, row4_h);

        // Row 5: 6 Presets (Matching search preset button height with 80s/90s wide rectangular aspect ratio)
        let preset_gap = (inner_w * 0.008).clamp(3.0, 10.0);
        let total_preset_gaps = 5.0 * preset_gap;
        let single_preset_w = (inner_w - total_preset_gaps) / 6.0;

        let mut preset_rects = [Rectangle::default(); 6];
        for (i, slot) in preset_rects.iter_mut().enumerate() {
            let px = inner_x + (i as f32 * (single_preset_w + preset_gap));
            *slot = Rectangle::new(px, row5_y, single_preset_w, row5_h);
        }

        let tune_knob_rect = Rectangle::default();

        Self {
            orientation: Orientation::Landscape,
            screen_width: width,
            screen_height: height,
            bezel_rect,
            power_btn_rect,
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
            font_display_large,
            font_display_small,
            font_ui_regular,
            font_ui_small,
        }
    }

    /// Compute portrait pocket-radio layout (Mobile Phone Portrait)
    fn compute_portrait(width: f32, height: f32, num_bands: usize) -> Self {
        let margin_x = (width * 0.02).max(6.0);
        let margin_y = (height * 0.015).max(6.0);

        let bezel_rect = Rectangle::new(
            margin_x,
            margin_y,
            width - (margin_x * 2.0),
            height - (margin_y * 2.0),
        );

        let pad_x = bezel_rect.width * 0.03;
        let inner_x = bezel_rect.x + pad_x;
        let inner_w = bezel_rect.width - (pad_x * 2.0);

        // Scaled typography
        let font_display_large = ((height * 0.028).round() as i32).clamp(13, 24);
        let font_display_small = ((height * 0.018).round() as i32).clamp(9, 15);
        let font_ui_regular = ((height * 0.022).round() as i32).clamp(11, 18);
        let font_ui_small = ((height * 0.018).round() as i32).clamp(9, 14);

        // Top bar: Power button and Volume slider
        let top_y = bezel_rect.y + (bezel_rect.height * 0.015);
        let top_h = (height * 0.045).clamp(28.0, 42.0);
        let power_w = (inner_w * 0.22).clamp(60.0, 90.0);
        let vol_w = inner_w - power_w - 12.0;

        let power_btn_rect = Rectangle::new(inner_x, top_y, power_w, top_h);
        let vol_label_rect = Rectangle::new(inner_x + power_w + 12.0, top_y, vol_w * 0.3, top_h);
        let vol_slider_rect = Rectangle::new(
            vol_label_rect.x + vol_label_rect.width + 4.0,
            top_y,
            vol_w * 0.65,
            top_h,
        );

        // Display (approx 16% of height)
        let disp_y = top_y + top_h + (height * 0.015);
        let disp_h = (height * 0.16).clamp(70.0, 140.0);
        let display_rect = Rectangle::new(inner_x, disp_y, inner_w, disp_h);

        // Search Bar (approx 5.5% of height)
        let search_y = disp_y + disp_h + (height * 0.012);
        let search_h = (height * 0.055).clamp(32.0, 46.0);
        let clear_w = (inner_w * 0.20).clamp(50.0, 75.0);
        let search_w = inner_w - clear_w - 8.0;

        let search_box_rect = Rectangle::new(inner_x, search_y, search_w, search_h);
        let search_clear_rect =
            Rectangle::new(inner_x + search_w + 8.0, search_y, clear_w, search_h);

        // Waveband selector grid (2 rows on mobile)
        let bands_y = search_y + search_h + (height * 0.012);
        let bands_h = (height * 0.045).clamp(28.0, 40.0);

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

        // Dial Track (approx 10% of height)
        let dial_y = bands_y + bands_total_h + (height * 0.015);
        let dial_h = (height * 0.08).clamp(35.0, 55.0);
        let dial_track_rect = Rectangle::new(inner_x, dial_y, inner_w, dial_h);

        // Tuning Step Buttons (4 buttons in a row)
        let tune_btn_y = dial_y + dial_h + (height * 0.012);
        let tune_btn_h = (height * 0.05).clamp(30.0, 44.0);
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

        // Presets 2x3 Grid (Matching search preset button height)
        let presets_y = tune_btn_y + tune_btn_h + (height * 0.015);
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

        let tune_knob_rect = Rectangle::new(0.0, 0.0, 0.0, 0.0); // Hidden in portrait or integrated in step buttons

        Self {
            orientation: Orientation::Portrait,
            screen_width: width,
            screen_height: height,
            bezel_rect,
            power_btn_rect,
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
            font_display_large,
            font_display_small,
            font_ui_regular,
            font_ui_small,
        }
    }

    /// Calculate the X coordinate along the dial track for a normalized needle position (0.0 to 1.0)
    pub fn needle_x(&self, progress: f32) -> f32 {
        let clamped = progress.clamp(0.0, 1.0);
        self.dial_track_rect.x + (self.dial_track_rect.width * clamped)
    }

    /// Convert a click/touch X coordinate on the dial track to a normalized position (0.0 to 1.0)
    pub fn needle_progress_from_x(&self, x: f32) -> f32 {
        if self.dial_track_rect.width <= 0.0 {
            return 0.0;
        }
        let rel = x - self.dial_track_rect.x;
        (rel / self.dial_track_rect.width).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_landscape_layout_sanity() {
        let layout = StereoLayout::compute(1280.0, 720.0, 8);
        assert_eq!(layout.orientation, Orientation::Landscape);

        // Verify bounding boxes are positive and within screen bounds
        assert!(layout.bezel_rect.width > 0.0);
        assert!(layout.display_rect.width > 100.0);
        assert!(layout.search_box_rect.width > 100.0);
        assert_eq!(layout.band_btn_rects.len(), 8);
        assert!(layout.preset_rects[0].width > 30.0);
        // Presets match search preset (band) button height
        assert_eq!(
            layout.preset_rects[0].height,
            layout.band_btn_rects[0].height
        );
        assert!(layout.dial_track_rect.width > 50.0);

        // Needle conversion check
        let nx_0 = layout.needle_x(0.0);
        let nx_1 = layout.needle_x(1.0);
        assert_eq!(nx_0, layout.dial_track_rect.x);
        assert_eq!(
            nx_1,
            layout.dial_track_rect.x + layout.dial_track_rect.width
        );

        let prog = layout.needle_progress_from_x(
            layout.dial_track_rect.x + (layout.dial_track_rect.width * 0.5),
        );
        assert!((prog - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_landscape_margins_and_gaps_symmetry() {
        let width = 920.0;
        let height = 270.0;
        let layout = StereoLayout::compute(width, height, 8);

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
    fn test_portrait_layout_sanity() {
        let layout = StereoLayout::compute(390.0, 844.0, 8); // iPhone 14 dimensions
        assert_eq!(layout.orientation, Orientation::Portrait);

        assert!(layout.display_rect.width > 200.0);
        assert!(layout.preset_rects[0].width > 100.0); // 2 columns of touch presets
        // Presets match search preset (band) button height
        assert_eq!(
            layout.preset_rects[0].height,
            layout.band_btn_rects[0].height
        );
        assert!(layout.preset_rects[5].y > layout.preset_rects[0].y); // Row stacking
    }

    #[test]
    fn test_small_window_resilience() {
        let layout = StereoLayout::compute(320.0, 240.0, 6);
        assert!(layout.display_rect.width > 0.0);
        assert!(layout.display_rect.height > 0.0);
        assert!(layout.font_display_large >= 10);
    }
}
