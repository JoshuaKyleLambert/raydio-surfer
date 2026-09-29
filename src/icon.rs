//! Procedurally generated application / window icon for RaydioSurfer.
//!
//! The icon is a vintage tuner-dial motif (a warm amber radio knob with a
//! pointer needle and a small broadcast-wave flourish) painted directly into
//! an RGBA8 pixel buffer. Drawing everything by hand keeps the icon fully
//! self-contained (no external asset files) and portable across every build
//! target, and it only relies on raylib's stable C FFI to hand the finished
//! image to the window.

use raylib::prelude::RaylibHandle;
use std::f32::consts::PI;

/// Rendered icon edge length in pixels. A single crisp 256x256 image lets the
/// window manager down-scale to whatever size it needs (taskbar, dock, alt-tab).
const ICON_SIZE: i32 = 256;

/// Builds the icon on the fly and installs it as the current window icon.
///
/// Must be called after the raylib window has been created; the `RaylibHandle`
/// reference is taken purely to enforce that ordering at the type level.
pub fn set_app_icon(_rl: &RaylibHandle) {
    let mut pixels = render_icon(ICON_SIZE);

    let image = raylib::ffi::Image {
        data: pixels.as_mut_ptr() as *mut std::os::raw::c_void,
        width: ICON_SIZE,
        height: ICON_SIZE,
        mipmaps: 1,
        format: raylib::ffi::PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 as i32,
    };

    // SAFETY: `image` points at a valid, fully-initialised RGBA8 buffer that
    // stays alive for the duration of this call. `SetWindowIcon` (via GLFW)
    // copies the pixel data before returning, so `pixels` can be dropped right
    // after at the end of the function.
    unsafe {
        raylib::ffi::SetWindowIcon(image);
    }
}

/// Renders the complete icon and returns the raw RGBA8 pixel buffer.
fn render_icon(size: i32) -> Vec<u8> {
    let mut buf = vec![0u8; (size * size * 4) as usize];
    let s = size as f32 / 256.0; // uniform scale from the 256px reference design

    draw_panel(&mut buf, size, s);
    draw_dial(&mut buf, size, s);
    draw_ticks(&mut buf, size, s);
    draw_needle(&mut buf, size, s);
    draw_hub(&mut buf, size, s);
    draw_waves(&mut buf, size, s);

    buf
}

/// Dark rounded-square body with a soft vertical gradient and an amber rim.
fn draw_panel(buf: &mut [u8], size: i32, s: f32) {
    let (cx, cy) = (128.0 * s, 128.0 * s);
    let (hw, hh) = (112.0 * s, 112.0 * s);
    let rr = 40.0 * s;

    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let d = sd_round_rect(px, py, cx, cy, hw, hh, rr);

            let cov = (0.5 - d).clamp(0.0, 1.0);
            if cov > 0.0 {
                let t = ((py - (cy - hh)) / (2.0 * hh)).clamp(0.0, 1.0);
                let r = lerp(34.0, 14.0, t);
                let g = lerp(34.0, 15.0, t);
                let b = lerp(50.0, 22.0, t);
                put(buf, size, x, y, [r / 255.0, g / 255.0, b / 255.0, cov]);
            }

            // Amber rim inset a few pixels from the panel edge.
            let edge = (1.0 - (d + 3.0 * s).abs() / (1.6 * s)).clamp(0.0, 1.0);
            if edge > 0.0 {
                put(buf, size, x, y, rgba(255, 178, 74, edge * 0.9));
            }
        }
    }
}

/// The main tuner dial: a dark bezel, a radial amber face, and concentric grooves.
fn draw_dial(buf: &mut [u8], size: i32, s: f32) {
    let (dcx, dcy) = (128.0 * s, 140.0 * s);
    let bezel_r = 74.0 * s;
    let face_r = 66.0 * s;
    let grooves = [58.0 * s, 46.0 * s, 34.0 * s];

    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dist = ((px - dcx).powi(2) + (py - dcy).powi(2)).sqrt();

            // Dark bezel ring around the face.
            let bezel_cov = (0.5 - (dist - bezel_r)).clamp(0.0, 1.0);
            if bezel_cov > 0.0 {
                put(buf, size, x, y, rgba(22, 17, 13, bezel_cov));
            }

            // Amber face with a soft radial gradient.
            let face_cov = (0.5 - (dist - face_r)).clamp(0.0, 1.0);
            if face_cov > 0.0 {
                let t = (dist / face_r).clamp(0.0, 1.0);
                let r = lerp(255.0, 188.0, t);
                let g = lerp(168.0, 96.0, t);
                let b = lerp(70.0, 28.0, t);
                put(buf, size, x, y, [r / 255.0, g / 255.0, b / 255.0, face_cov]);
            }

            // Engraved concentric grooves.
            for &gr in &grooves {
                let groove = (1.0 - (dist - gr).abs() / (1.3 * s)).clamp(0.0, 1.0);
                if groove > 0.0 {
                    put(buf, size, x, y, rgba(150, 78, 26, groove * 0.45));
                }
            }
        }
    }
}

/// Twelve tick marks around the dial rim, like a rotary tuning scale.
fn draw_ticks(buf: &mut [u8], size: i32, s: f32) {
    let (dcx, dcy) = (128.0 * s, 140.0 * s);
    let inner = 60.0 * s;
    let outer = 68.0 * s;
    let col = rgba(255, 232, 190, 0.85);

    for i in 0..12 {
        let a = i as f32 / 12.0 * 2.0 * PI;
        let (ca, sa) = (a.cos(), a.sin());
        let start = (dcx + inner * ca, dcy + inner * sa);
        let end = (dcx + outer * ca, dcy + outer * sa);
        draw_thick_segment(buf, size, start, end, 1.6 * s, col);
    }
}

/// The bright tuner pointer needle sweeping to the upper-left of the dial.
fn draw_needle(buf: &mut [u8], size: i32, s: f32) {
    let (dcx, dcy) = (128.0 * s, 140.0 * s);
    let a = -110.0_f32.to_radians();
    let dir = (a.cos(), a.sin());

    let tip = (dcx + 56.0 * s * dir.0, dcy + 56.0 * s * dir.1);
    let tail = (dcx - 16.0 * s * dir.0, dcy - 16.0 * s * dir.1);

    // Short dark counterweight behind the hub.
    draw_thick_segment(buf, size, (dcx, dcy), tail, 3.0 * s, rgba(40, 26, 16, 0.9));
    // Bright cream needle toward the tip.
    draw_thick_segment(buf, size, (dcx, dcy), tip, 3.2 * s, rgba(255, 244, 214, 0.95));
}

/// Central knob hub that anchors the needle.
fn draw_hub(buf: &mut [u8], size: i32, s: f32) {
    let (dcx, dcy) = (128.0 * s, 140.0 * s);

    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dist = ((px - dcx).powi(2) + (py - dcy).powi(2)).sqrt();

            let ring = (1.0 - (dist - 15.0 * s).abs() / (2.0 * s)).clamp(0.0, 1.0);
            if ring > 0.0 {
                put(buf, size, x, y, rgba(255, 190, 96, ring));
            }

            let cap = (0.5 - (dist - 12.0 * s)).clamp(0.0, 1.0);
            if cap > 0.0 {
                put(buf, size, x, y, rgba(26, 19, 14, cap));
            }

            // Tiny specular highlight on the cap.
            let hl_dist = ((px - (dcx - 4.0 * s)).powi(2) + (py - (dcy - 4.0 * s)).powi(2)).sqrt();
            let hl = (0.5 - (hl_dist - 3.0 * s)).clamp(0.0, 1.0);
            if hl > 0.0 {
                put(buf, size, x, y, rgba(120, 110, 110, hl * 0.5));
            }
        }
    }
}

/// A small teal broadcast-wave flourish in the top-right corner.
fn draw_waves(buf: &mut [u8], size: i32, s: f32) {
    let (ex, ey) = (196.0 * s, 66.0 * s);
    let dir = (-45.0_f32).to_radians(); // waves open toward the upper-right
    let radii = [13.0 * s, 22.0 * s, 31.0 * s];
    let half = 1.4 * s;
    let sector = 0.9;
    let col = rgba(96, 226, 220, 0.9);

    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dx = px - ex;
            let dy = py - ey;
            let dist = (dx * dx + dy * dy).sqrt();

            // Solid emitter dot.
            let dot = (0.5 - (dist - 4.0 * s)).clamp(0.0, 1.0);
            if dot > 0.0 {
                put(buf, size, x, y, col_with_alpha(col, dot));
            }

            let ang_off = ang_norm(dy.atan2(dx) - dir).abs();
            if ang_off < sector {
                let fade = ((sector - ang_off) / 0.25).clamp(0.0, 1.0);
                for &r in &radii {
                    let cov = (1.0 - (dist - r).abs() / half).clamp(0.0, 1.0);
                    if cov > 0.0 {
                        put(buf, size, x, y, col_with_alpha(col, cov * fade));
                    }
                }
            }
        }
    }
}

// --- small rasterisation helpers -------------------------------------------

/// Alpha-composites `col` (straight-alpha RGBA in 0..1) over pixel (x, y).
fn put(buf: &mut [u8], size: i32, x: i32, y: i32, col: [f32; 4]) {
    if x < 0 || y < 0 || x >= size || y >= size {
        return;
    }
    let sa = col[3].clamp(0.0, 1.0);
    if sa <= 0.0 {
        return;
    }

    let idx = ((y * size + x) * 4) as usize;
    let dr = buf[idx] as f32 / 255.0;
    let dg = buf[idx + 1] as f32 / 255.0;
    let db = buf[idx + 2] as f32 / 255.0;
    let da = buf[idx + 3] as f32 / 255.0;

    let oa = sa + da * (1.0 - sa);
    let blend = |src: f32, dst: f32| {
        if oa <= 0.0 {
            0.0
        } else {
            (src * sa + dst * da * (1.0 - sa)) / oa
        }
    };

    buf[idx] = to_u8(blend(col[0], dr));
    buf[idx + 1] = to_u8(blend(col[1], dg));
    buf[idx + 2] = to_u8(blend(col[2], db));
    buf[idx + 3] = to_u8(oa);
}

/// Rasterises a rounded thick segment (from `a` to `b`) with anti-aliased edges.
fn draw_thick_segment(buf: &mut [u8], size: i32, a: (f32, f32), b: (f32, f32), half: f32, col: [f32; 4]) {
    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let d = sd_segment((px, py), a, b) - half;
            let cov = (0.5 - d).clamp(0.0, 1.0);
            if cov > 0.0 {
                put(buf, size, x, y, col_with_alpha(col, cov));
            }
        }
    }
}

/// Signed distance from point `p` to a rounded rectangle.
fn sd_round_rect(px: f32, py: f32, cx: f32, cy: f32, hw: f32, hh: f32, r: f32) -> f32 {
    let qx = (px - cx).abs() - (hw - r);
    let qy = (py - cy).abs() - (hh - r);
    let ax = qx.max(0.0);
    let ay = qy.max(0.0);
    (ax * ax + ay * ay).sqrt() + qx.max(qy).min(0.0) - r
}

/// Distance from point `p` to the line segment `a`-`b`.
fn sd_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let pax = p.0 - a.0;
    let pay = p.1 - a.1;
    let bax = b.0 - a.0;
    let bay = b.1 - a.1;
    let denom = bax * bax + bay * bay;
    let h = if denom <= 0.0 {
        0.0
    } else {
        ((pax * bax + pay * bay) / denom).clamp(0.0, 1.0)
    };
    let dx = pax - bax * h;
    let dy = pay - bay * h;
    (dx * dx + dy * dy).sqrt()
}

/// Normalises an angle to the range (-PI, PI].
fn ang_norm(mut a: f32) -> f32 {
    while a > PI {
        a -= 2.0 * PI;
    }
    while a < -PI {
        a += 2.0 * PI;
    }
    a
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn rgba(r: u8, g: u8, b: u8, a: f32) -> [f32; 4] {
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a]
}

fn col_with_alpha(col: [f32; 4], a: f32) -> [f32; 4] {
    [col[0], col[1], col[2], col[3] * a]
}

fn to_u8(v: f32) -> u8 {
    (v * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_buffer_has_expected_rgba_size() {
        let buf = render_icon(ICON_SIZE);
        assert_eq!(buf.len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
    }

    #[test]
    fn icon_is_not_blank() {
        // At least some pixels must be opaque, otherwise the icon would be invisible.
        let buf = render_icon(64);
        let opaque = buf.chunks_exact(4).filter(|px| px[3] > 0).count();
        assert!(opaque > 0, "rendered icon should contain visible pixels");
    }

    #[test]
    fn sd_round_rect_inside_is_negative() {
        // Centre of the rectangle is well inside, so the signed distance is negative.
        assert!(sd_round_rect(50.0, 50.0, 50.0, 50.0, 40.0, 40.0, 10.0) < 0.0);
    }

    #[test]
    fn sd_segment_endpoint_is_zero() {
        let d = sd_segment((0.0, 0.0), (0.0, 0.0), (10.0, 0.0));
        assert!(d.abs() < 1e-6);
    }

    #[test]
    fn ang_norm_wraps_into_range() {
        let a = ang_norm(3.0 * PI);
        assert!(a > -PI && a <= PI + 1e-6);
    }
}
