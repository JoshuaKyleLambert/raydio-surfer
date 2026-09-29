#!/usr/bin/env python3
"""Procedural icon generator for RaydioSurfer.

Renders a "cool" vintage car-stereo / radio-tuner icon (glowing amber tuning
dial with a needle, a backlit scale arc, and broadcast waves) entirely with the
Python standard library -- no third-party imaging dependencies required.

Outputs (next to this script):
  * icon.png  -- 256x256 RGBA, embedded into the binary and used as the window
                 / taskbar icon at runtime.
  * icon.ico  -- multi-resolution Windows icon (16..256) for the .exe / launcher.

Run:  python3 assets/generate_icon.py
"""

import math
import os
import struct
import zlib

# ----------------------------------------------------------------------------
# Small vector / drawing helpers (all operate in floating point pixel space).
# ----------------------------------------------------------------------------


def clamp(v, lo, hi):
    return lo if v < lo else hi if v > hi else v


def lerp(a, b, t):
    return a + (b - a) * t


def mix(c0, c1, t):
    return tuple(lerp(c0[i], c1[i], t) for i in range(len(c0)))


def over(dst, src):
    """Alpha-composite src (r,g,b,a in 0..1) over dst (r,g,b,a in 0..1)."""
    sa = src[3]
    if sa <= 0.0:
        return dst
    da = dst[3]
    out_a = sa + da * (1.0 - sa)
    if out_a <= 0.0:
        return (0.0, 0.0, 0.0, 0.0)
    r = (src[0] * sa + dst[0] * da * (1.0 - sa)) / out_a
    g = (src[1] * sa + dst[1] * da * (1.0 - sa)) / out_a
    b = (src[2] * sa + dst[2] * da * (1.0 - sa)) / out_a
    return (r, g, b, out_a)


def rounded_rect_sdf(px, py, cx, cy, hw, hh, r):
    qx = abs(px - cx) - (hw - r)
    qy = abs(py - cy) - (hh - r)
    ox = max(qx, 0.0)
    oy = max(qy, 0.0)
    return math.hypot(ox, oy) + min(max(qx, qy), 0.0) - r


def circle_sdf(px, py, cx, cy, r):
    return math.hypot(px - cx, py - cy) - r


def segment_dist(px, py, ax, ay, bx, by):
    vx, vy = bx - ax, by - ay
    wx, wy = px - ax, py - ay
    denom = vx * vx + vy * vy
    t = 0.0 if denom == 0.0 else clamp((wx * vx + wy * vy) / denom, 0.0, 1.0)
    dx = ax + vx * t - px
    dy = ay + vy * t - py
    return math.hypot(dx, dy)


# Palette (0..1 float RGB) -- warm amber vintage backlight on deep navy.
NAVY_TOP = (0.086, 0.086, 0.121)   # ~ (22,22,31)
NAVY_BOT = (0.035, 0.035, 0.062)   # ~ (9,9,16)
RIM_DARK = (0.05, 0.05, 0.07)
RIM_LIGHT = (0.24, 0.25, 0.30)
DIAL_FACE = (0.043, 0.047, 0.066)
AMBER = (1.0, 0.62, 0.18)          # ~ (255,158,46)
AMBER_HOT = (1.0, 0.86, 0.55)
TEAL = (0.20, 0.85, 0.80)


def shade(x, y, size):
    """Return the final (r,g,b,a) colour for a point in a `size` px canvas."""
    s = size / 256.0  # scale factor so geometry is authored at 256px

    # Everything authored in 256-space, then scaled.
    px, py = x / s, y / s

    cx, cy = 128.0, 134.0

    col = (0.0, 0.0, 0.0, 0.0)

    # --- Panel: rounded-rect background with a vertical gradient ------------
    margin = 8.0
    panel = rounded_rect_sdf(px, py, 128.0, 128.0, 128.0 - margin, 128.0 - margin, 52.0)
    if panel > 0.75:
        return col  # fully outside the icon body -> transparent
    grad_t = clamp(py / 256.0, 0.0, 1.0)
    base = mix(NAVY_TOP, NAVY_BOT, grad_t)
    panel_a = clamp(0.75 - panel, 0.0, 1.0)
    col = over(col, (base[0], base[1], base[2], panel_a))

    # subtle inner top highlight
    hi = clamp(1.0 - (py - margin) / 90.0, 0.0, 1.0) * 0.06
    col = over(col, (1.0, 1.0, 1.0, hi * panel_a))

    # --- Warm radial glow behind the dial ----------------------------------
    gd = math.hypot(px - cx, py - cy)
    glow = clamp(1.0 - gd / 120.0, 0.0, 1.0)
    glow = glow * glow
    col = over(col, (AMBER[0], AMBER[1], AMBER[2], 0.16 * glow * panel_a))

    # --- Metallic outer ring -----------------------------------------------
    outer_r = 96.0
    ring = abs(circle_sdf(px, py, cx, cy, outer_r)) - 7.0
    if ring < 1.0:
        rt = clamp((py - (cy - outer_r)) / (2.0 * outer_r), 0.0, 1.0)
        ring_col = mix(RIM_LIGHT, RIM_DARK, rt)
        col = over(col, (ring_col[0], ring_col[1], ring_col[2], clamp(1.0 - ring, 0.0, 1.0)))

    # --- Dial face ----------------------------------------------------------
    face = circle_sdf(px, py, cx, cy, 86.0)
    if face < 1.0:
        col = over(col, (DIAL_FACE[0], DIAL_FACE[1], DIAL_FACE[2], clamp(1.0 - face, 0.0, 1.0)))

    # --- Tick marks around the top arc (-150deg .. -30deg) -----------------
    n_ticks = 11
    for i in range(n_ticks):
        ang = math.radians(-150.0 + (120.0 / (n_ticks - 1)) * i)
        ca, sa = math.cos(ang), math.sin(ang)
        major = (i % 5 == 0)
        r_in = 62.0
        r_out = 78.0 if major else 72.0
        ax, ay = cx + ca * r_in, cy + sa * r_in
        bx, by = cx + ca * r_out, cy + sa * r_out
        d = segment_dist(px, py, ax, ay, bx, by)
        w = 2.6 if major else 1.6
        a = clamp(w - d, 0.0, 1.0)
        if a > 0.0:
            tc = AMBER_HOT if major else (0.65, 0.68, 0.72)
            col = over(col, (tc[0], tc[1], tc[2], a * 0.95))

    # --- Glowing "tuned" arc (-150deg .. -78deg) ---------------------------
    ang = math.atan2(py - cy, px - cx)
    deg = math.degrees(ang)
    arc_r = 70.0
    darc = abs(math.hypot(px - cx, py - cy) - arc_r)
    if -151.0 <= deg <= -77.0:
        a_glow = clamp(6.0 - darc, 0.0, 1.0)
        col = over(col, (AMBER[0], AMBER[1], AMBER[2], 0.25 * a_glow))
        a_core = clamp(3.0 - darc, 0.0, 1.0)
        col = over(col, (AMBER_HOT[0], AMBER_HOT[1], AMBER_HOT[2], 0.95 * a_core))

    # --- Broadcast waves (upper right) -------------------------------------
    for k, rr in enumerate((30.0, 44.0, 58.0)):
        dw = abs(math.hypot(px - cx, py - cy) - rr)
        if -42.0 <= deg <= 6.0:
            a = clamp(2.4 - dw, 0.0, 1.0) * (0.7 - k * 0.18)
            col = over(col, (TEAL[0], TEAL[1], TEAL[2], a))

    # --- Needle (points to ~ -84deg, up & slightly right) ------------------
    nang = math.radians(-84.0)
    tipx, tipy = cx + math.cos(nang) * 80.0, cy + math.sin(nang) * 80.0
    tailx, taily = cx - math.cos(nang) * 16.0, cy - math.sin(nang) * 16.0
    dn = segment_dist(px, py, tailx, taily, tipx, tipy)
    # soft glow
    col = over(col, (AMBER[0], AMBER[1], AMBER[2], clamp(5.0 - dn, 0.0, 1.0) * 0.35))
    # solid tapered core
    col = over(col, (AMBER_HOT[0], AMBER_HOT[1], AMBER_HOT[2], clamp(2.4 - dn, 0.0, 1.0)))

    # --- Center hub ---------------------------------------------------------
    hub = circle_sdf(px, py, cx, cy, 13.0)
    col = over(col, (0.10, 0.10, 0.13, clamp(1.0 - hub, 0.0, 1.0)))
    hub2 = circle_sdf(px, py, cx, cy, 8.0)
    col = over(col, (AMBER[0], AMBER[1], AMBER[2], clamp(1.0 - hub2, 0.0, 1.0)))
    hub3 = circle_sdf(px, py, cx, cy, 3.5)
    col = over(col, (AMBER_HOT[0], AMBER_HOT[1], AMBER_HOT[2], clamp(1.0 - hub3, 0.0, 1.0)))

    return col


def render(size, ss=4):
    """Render an RGBA byte buffer of `size`x`size` with `ss`x supersampling."""
    inv = 1.0 / (ss * ss)
    row = bytearray()
    out = bytearray()
    for y in range(size):
        row.clear()
        for x in range(size):
            r = g = b = a = 0.0
            for sy in range(ss):
                fy = y + (sy + 0.5) / ss
                for sx in range(ss):
                    fx = x + (sx + 0.5) / ss
                    c = shade(fx, fy, size)
                    r += c[0] * c[3]
                    g += c[1] * c[3]
                    b += c[2] * c[3]
                    a += c[3]
            a_avg = a * inv
            if a_avg > 0.0:
                # un-premultiply
                rr = clamp((r / a) if a > 0 else 0.0, 0.0, 1.0)
                gg = clamp((g / a) if a > 0 else 0.0, 0.0, 1.0)
                bb = clamp((b / a) if a > 0 else 0.0, 0.0, 1.0)
            else:
                rr = gg = bb = 0.0
            row += bytes((
                int(rr * 255 + 0.5),
                int(gg * 255 + 0.5),
                int(bb * 255 + 0.5),
                int(clamp(a_avg, 0.0, 1.0) * 255 + 0.5),
            ))
        out += row
    return bytes(out)


def encode_png(rgba, size):
    def chunk(tag, data):
        return (struct.pack(">I", len(data)) + tag + data +
                struct.pack(">I", zlib.crc32(tag + data) & 0xffffffff))

    raw = bytearray()
    stride = size * 4
    for y in range(size):
        raw.append(0)  # filter type 0 (none)
        raw += rgba[y * stride:(y + 1) * stride]
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", ihdr)
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    png += chunk(b"IEND", b"")
    return png


def build_ico(sizes):
    pngs = [(sz, encode_png(render(sz), sz)) for sz in sizes]
    n = len(pngs)
    header = struct.pack("<HHH", 0, 1, n)
    offset = 6 + 16 * n
    entries = bytearray()
    body = bytearray()
    for sz, data in pngs:
        w = 0 if sz >= 256 else sz
        h = 0 if sz >= 256 else sz
        entries += struct.pack("<BBBBHHII", w, h, 0, 0, 1, 32, len(data), offset)
        offset += len(data)
        body += data
    return bytes(header) + bytes(entries) + bytes(body)


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    png_path = os.path.join(here, "icon.png")
    ico_path = os.path.join(here, "icon.ico")

    print("Rendering icon.png (256x256)...")
    with open(png_path, "wb") as f:
        f.write(encode_png(render(256), 256))

    print("Rendering icon.ico (multi-resolution)...")
    with open(ico_path, "wb") as f:
        f.write(build_ico([16, 32, 48, 64, 128, 256]))

    print("Done:")
    print("  ", png_path, os.path.getsize(png_path), "bytes")
    print("  ", ico_path, os.path.getsize(ico_path), "bytes")


if __name__ == "__main__":
    main()
