//! Custom window chrome for the undecorated window: moving it by the background and
//! resizing it by its edges.
//!
//! "Window units" are the units of `get_window_position`, `set_window_position` and
//! `set_window_size`. With `FLAG_WINDOW_HIGHDPI` on Windows and X11 these are physical
//! pixels, while the mouse position and screen size are logical pixels. On macOS
//! everything is in logical points.

use raylib::prelude::*;

/// Thickness (logical pixels) of the invisible resize border along each window edge.
pub const RESIZE_BORDER: f32 = 5.0;
/// Length (logical pixels) along an edge, measured from a corner, that resizes diagonally.
pub const RESIZE_CORNER: f32 = 14.0;
/// Smallest window size (logical pixels) reachable by dragging the edges.
pub const MIN_WINDOW_WIDTH: f32 = 480.0;
pub const MIN_WINDOW_HEIGHT: f32 = 240.0;

/// Factor converting logical pixels into window units.
pub fn window_unit_scale(dpi_scale: Vector2) -> Vector2 {
    if cfg!(target_os = "macos") {
        return Vector2::new(1.0, 1.0);
    }
    Vector2::new(
        if dpi_scale.x > 0.0 { dpi_scale.x } else { 1.0 },
        if dpi_scale.y > 0.0 { dpi_scale.y } else { 1.0 },
    )
}

/// Current window size in window units, from the logical screen size and the physical
/// render (framebuffer) size.
pub fn window_units_size(screen: (i32, i32), render: (i32, i32)) -> Vector2 {
    if cfg!(target_os = "macos") || render.0 <= 0 || render.1 <= 0 {
        return Vector2::new(screen.0 as f32, screen.1 as f32);
    }
    Vector2::new(render.0 as f32, render.1 as f32)
}

/// Desktop position of the cursor in window units, from the window position and the
/// window-local logical mouse position. Unlike the mouse position itself it does not
/// change when the window moves under a stationary cursor.
pub fn cursor_in_window_units(window_pos: Vector2, mouse: Vector2, unit_scale: Vector2) -> Vector2 {
    Vector2::new(
        window_pos.x + mouse.x * unit_scale.x,
        window_pos.y + mouse.y * unit_scale.y,
    )
}

/// Compute the new window position that keeps the window-local `anchor` point under the cursor.
///
/// `mouse` and `anchor` are window-local logical coordinates; the offset between them is
/// converted to window units.
pub fn window_drag_position(
    window_pos: Vector2,
    mouse: Vector2,
    anchor: Vector2,
    dpi_scale: Vector2,
) -> (i32, i32) {
    let s = window_unit_scale(dpi_scale);
    let x = window_pos.x + (mouse.x - anchor.x) * s.x;
    let y = window_pos.y + (mouse.y - anchor.y) * s.y;
    (x.round() as i32, y.round() as i32)
}

/// Which window edges a resize grabs. Two adjacent edges form a corner.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResizeEdges {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

impl ResizeEdges {
    pub fn any(self) -> bool {
        self.left || self.right || self.top || self.bottom
    }

    /// Hit-test the window-local logical `mouse` position against the resize border of a
    /// `width` x `height` window. Points near a corner grab both adjacent edges.
    pub fn at(mouse: Vector2, width: f32, height: f32) -> Self {
        let (x, y) = (mouse.x, mouse.y);
        if x < 0.0 || y < 0.0 || x >= width || y >= height {
            return Self::default();
        }
        let left = x < RESIZE_BORDER;
        let right = x >= width - RESIZE_BORDER;
        let top = y < RESIZE_BORDER;
        let bottom = y >= height - RESIZE_BORDER;
        let on_horizontal = top || bottom;
        let on_vertical = left || right;
        Self {
            left: left || (on_horizontal && x < RESIZE_CORNER),
            right: right || (on_horizontal && x >= width - RESIZE_CORNER),
            top: top || (on_vertical && y < RESIZE_CORNER),
            bottom: bottom || (on_vertical && y >= height - RESIZE_CORNER),
        }
    }

    /// Mouse cursor shape that matches these edges.
    pub fn cursor(self) -> MouseCursor {
        match (self.left, self.right, self.top, self.bottom) {
            (true, _, true, _) | (_, true, _, true) => MouseCursor::MOUSE_CURSOR_RESIZE_NWSE,
            (_, true, true, _) | (true, _, _, true) => MouseCursor::MOUSE_CURSOR_RESIZE_NESW,
            (true, _, _, _) | (_, true, _, _) => MouseCursor::MOUSE_CURSOR_RESIZE_EW,
            (_, _, true, _) | (_, _, _, true) => MouseCursor::MOUSE_CURSOR_RESIZE_NS,
            _ => MouseCursor::MOUSE_CURSOR_DEFAULT,
        }
    }
}

/// An edge resize in progress. All values are in window units.
#[derive(Debug, Clone, Copy)]
pub struct ResizeDrag {
    pub edges: ResizeEdges,
    pub start_cursor: Vector2,
    pub start_pos: Vector2,
    pub start_size: Vector2,
}

impl ResizeDrag {
    /// Window rectangle `(x, y, width, height)` for the current desktop `cursor` position.
    ///
    /// Dragged edges follow the cursor while the opposite edges stay put, so resizing from
    /// the left or top also moves the window. The size never drops below `min_size` (or the
    /// starting size, if the window already was smaller).
    pub fn rect_for_cursor(&self, cursor: Vector2, min_size: Vector2) -> (i32, i32, i32, i32) {
        let dx = cursor.x - self.start_cursor.x;
        let dy = cursor.y - self.start_cursor.y;
        let min_w = min_size.x.min(self.start_size.x);
        let min_h = min_size.y.min(self.start_size.y);

        let (mut x, mut w) = (self.start_pos.x, self.start_size.x);
        if self.edges.right {
            w = (self.start_size.x + dx).max(min_w);
        } else if self.edges.left {
            w = (self.start_size.x - dx).max(min_w);
            x = self.start_pos.x + self.start_size.x - w;
        }

        let (mut y, mut h) = (self.start_pos.y, self.start_size.y);
        if self.edges.bottom {
            h = (self.start_size.y + dy).max(min_h);
        } else if self.edges.top {
            h = (self.start_size.y - dy).max(min_h);
            y = self.start_pos.y + self.start_size.y - h;
        }

        (x.round() as i32, y.round() as i32, w.round() as i32, h.round() as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_drag_position_keeps_anchor_under_cursor() {
        let pos = Vector2::new(100.0, 200.0);
        let anchor = Vector2::new(50.0, 10.0);

        // Mouse still on the anchor: window does not move
        assert_eq!(
            window_drag_position(pos, anchor, anchor, Vector2::new(1.5, 1.5)),
            (100, 200)
        );

        let mouse = Vector2::new(60.0, 4.0); // moved +10, -6 logical
        if cfg!(target_os = "macos") {
            assert_eq!(window_drag_position(pos, mouse, anchor, Vector2::new(2.0, 2.0)), (110, 194));
            return;
        }
        // No scaling
        assert_eq!(window_drag_position(pos, mouse, anchor, Vector2::new(1.0, 1.0)), (110, 194));
        // 150% scaling: logical offset becomes physical
        assert_eq!(window_drag_position(pos, mouse, anchor, Vector2::new(1.5, 1.5)), (115, 191));
        // Invalid DPI falls back to 1.0
        assert_eq!(window_drag_position(pos, mouse, anchor, Vector2::new(0.0, 0.0)), (110, 194));
    }

    #[test]
    fn test_window_units_size_and_cursor() {
        if cfg!(target_os = "macos") {
            assert_eq!(window_units_size((920, 270), (1840, 540)), Vector2::new(920.0, 270.0));
            return;
        }
        assert_eq!(window_units_size((920, 270), (1380, 405)), Vector2::new(1380.0, 405.0));
        // Missing framebuffer size falls back to the screen size
        assert_eq!(window_units_size((920, 270), (0, 0)), Vector2::new(920.0, 270.0));

        let scale = window_unit_scale(Vector2::new(1.5, 1.5));
        let c = cursor_in_window_units(Vector2::new(100.0, 50.0), Vector2::new(10.0, 20.0), scale);
        assert_eq!(c, Vector2::new(115.0, 80.0));
    }

    #[test]
    fn test_resize_edges_hit_test() {
        let (w, h) = (920.0, 270.0);
        let at = |x: f32, y: f32| ResizeEdges::at(Vector2::new(x, y), w, h);

        // Interior and outside the window: nothing
        assert!(!at(460.0, 135.0).any());
        assert!(!at(-1.0, 135.0).any());
        assert!(!at(460.0, 270.0).any());

        // Plain edges
        assert_eq!(at(2.0, 135.0), ResizeEdges { left: true, ..Default::default() });
        assert_eq!(at(917.0, 135.0), ResizeEdges { right: true, ..Default::default() });
        assert_eq!(at(460.0, 1.0), ResizeEdges { top: true, ..Default::default() });
        assert_eq!(at(460.0, 268.0), ResizeEdges { bottom: true, ..Default::default() });

        // Corners, including the extended corner zone along each edge
        assert_eq!(at(1.0, 1.0), ResizeEdges { left: true, top: true, ..Default::default() });
        assert_eq!(at(10.0, 2.0), ResizeEdges { left: true, top: true, ..Default::default() });
        assert_eq!(at(918.0, 260.0), ResizeEdges { right: true, bottom: true, ..Default::default() });
        assert_eq!(at(2.0, 265.0), ResizeEdges { left: true, bottom: true, ..Default::default() });
        assert_eq!(at(912.0, 3.0), ResizeEdges { right: true, top: true, ..Default::default() });
    }

    #[test]
    fn test_resize_edges_cursor() {
        let e = |left, right, top, bottom| ResizeEdges { left, right, top, bottom };
        let c = |edges: ResizeEdges| edges.cursor() as i32;
        assert_eq!(c(e(false, false, false, false)), MouseCursor::MOUSE_CURSOR_DEFAULT as i32);
        assert_eq!(c(e(true, false, false, false)), MouseCursor::MOUSE_CURSOR_RESIZE_EW as i32);
        assert_eq!(c(e(false, true, false, false)), MouseCursor::MOUSE_CURSOR_RESIZE_EW as i32);
        assert_eq!(c(e(false, false, true, false)), MouseCursor::MOUSE_CURSOR_RESIZE_NS as i32);
        assert_eq!(c(e(false, false, false, true)), MouseCursor::MOUSE_CURSOR_RESIZE_NS as i32);
        assert_eq!(c(e(true, false, true, false)), MouseCursor::MOUSE_CURSOR_RESIZE_NWSE as i32);
        assert_eq!(c(e(false, true, false, true)), MouseCursor::MOUSE_CURSOR_RESIZE_NWSE as i32);
        assert_eq!(c(e(false, true, true, false)), MouseCursor::MOUSE_CURSOR_RESIZE_NESW as i32);
        assert_eq!(c(e(true, false, false, true)), MouseCursor::MOUSE_CURSOR_RESIZE_NESW as i32);
    }

    #[test]
    fn test_resize_drag_rect() {
        let drag = |edges: ResizeEdges| ResizeDrag {
            edges,
            start_cursor: Vector2::new(1000.0, 500.0),
            start_pos: Vector2::new(100.0, 200.0),
            start_size: Vector2::new(920.0, 270.0),
        };
        let min = Vector2::new(480.0, 240.0);
        let cursor = Vector2::new(1050.0, 530.0); // +50, +30

        // Bottom-right grows; position fixed
        let br = drag(ResizeEdges { right: true, bottom: true, ..Default::default() });
        assert_eq!(br.rect_for_cursor(cursor, min), (100, 200, 970, 300));

        // Left edge: moving right shrinks, right edge stays at 1020
        let l = drag(ResizeEdges { left: true, ..Default::default() });
        assert_eq!(l.rect_for_cursor(cursor, min), (150, 200, 870, 270));

        // Top-left: moving up-left grows and moves the window
        let tl = drag(ResizeEdges { left: true, top: true, ..Default::default() });
        assert_eq!(tl.rect_for_cursor(Vector2::new(980.0, 490.0), min), (80, 190, 940, 280));

        // Clamped to the minimum size; the opposite edge stays fixed
        assert_eq!(l.rect_for_cursor(Vector2::new(2000.0, 500.0), min), (540, 200, 480, 270));
        assert_eq!(br.rect_for_cursor(Vector2::new(0.0, 0.0), min), (100, 200, 480, 240));

        // A window already below the minimum is not forced to jump larger
        let small = ResizeDrag { start_size: Vector2::new(400.0, 200.0), ..br };
        assert_eq!(small.rect_for_cursor(Vector2::new(1000.0, 500.0), min), (100, 200, 400, 200));
    }
}
