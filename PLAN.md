# RaydioSurfer: Architecture & Multi-Platform Implementation Plan

## 1. Executive Summary & Design Vision
**RaydioSurfer** is a simple, tactile desktop, web, and mobile internet radio player styled after a **vintage car stereo / dash head unit**.

With access to a global database of over **60,000 live radio stations** from the Radio Browser directory, RaydioSurfer pairs vintage physical controls (waveband selector switches, frequency dial & needle, coarse/fine tuning, and 6 push-button presets) with modern search, live catalog scope counters, and **fluid responsive rendering across Desktop, Web (WebAssembly), and Mobile**.

---

## 2. Core Priority: Raylib Dynamic Multi-Resolution & Responsive Layout System

To ensure RaydioSurfer looks gorgeous and remains perfectly usable across **Mobile phones (Portrait/Landscape), Tablets, Desktop (4K/1080p, Resizable windows), and WebAssembly (Browser Canvas)**, the UI strictly avoids hardcoded pixel positions.

```
+---------------------------------------------------------------------------------------------------+
|                                Responsive Viewport Scaling Pipeline                               |
+---------------------------------------------------------------------------------------------------+
|  1. Viewport & DPI Detection (FLAG_WINDOW_RESIZABLE, FLAG_WINDOW_HIGHDPI, GetScreenWidth/Height)  |
|                                                |                                                  |
|  2. Virtual Grid & Proportional Layout Engine (LayoutContext: Margins, Flex Columns, Rows, Gaps) |
|                                                |                                                  |
|  3. Dynamic Font & Touch Target Scaling (Calculated pt/dp, Min 48dp Touch Targets for Mobile)     |
|                                                |                                                  |
|  4. Optional Virtual Render Target (RenderTexture2D with Letterboxing / Smart Aspect Preservation)|
+---------------------------------------------------------------------------------------------------+
```

### Key Responsive Design Principles:
1. **Dynamic Grid / Flex Layout (`LayoutContext`)**:
   - All component bounds (`Rectangle`) are computed dynamically as fractions of available viewport width and height.
   - **Horizontal Head Unit (Desktop / Landscape Mobile / Tablet / Web)**: Wide dash layout with display in the center, knobs on flanks, and presets across the base.
   - **Vertical Pocket Radio (Portrait Mobile)**: Stacks display at top, search/bands in the middle, frequency dial, and large thumb-friendly preset buttons at bottom.
2. **Raylib Built-in Facilities**:
   - `FLAG_WINDOW_RESIZABLE` and `FLAG_WINDOW_HIGHDPI` for native window adaptations.
   - `RenderTexture2D` option for pixel-perfect vintage CRT/LCD post-processing and uniform aspect ratio scaling.
   - Automatic touch-to-mouse translation natively handled by Raylib on mobile/web targets.
3. **Dynamic Font & Icon Scaling**:
   - Font sizes dynamically scale relative to viewport height (`font_size = (screen_height * 0.035).clamp(12.0, 36.0)`).
4. **Touch-Friendly Minimum Sizing**:
   - Preset buttons and toggles enforce a minimum physical touch dimension (48×48 dp / pt equivalent) on mobile screens.

---

## 3. Head Unit Layout Wireframes

### A. Landscape / Desktop / Web Canvas Layout
```
+---------------------------------------------------------------------------------------------------+
|  [ POWER ]  +--------------------------------------------------------------------+  ( VOL / MUTE )|
|             |  >>> BAND: [ELECTRONIC]  |  TUNED: 412 / 3,850  (TOTAL: 63,480) <<< |       (o)     |
|             |  [STEREO]  104.3 SYNTHWAVE RADIO USA - 320kbps MP3                  |   Vol: [ 75% ]|
|             +--------------------------------------------------------------------+                |
|                                                                                                   |
|  [ SEARCH: [ Type station, artist, or country... ] ]  [ X Clear ]                                 |
|                                                                                                   |
|  BANDS: [ ALL ] [ ROCK ] [ JAZZ ] [ ELECTRONIC ] [ POP ] [ CLASSIC ] [ AMBIENT ] [ NEWS ] [ 80s ] |
|                                                                                                   |
|  [ << 100 ] [ < TUNE ]  |======|======|======|======|======|======|======|   [ TUNE > ] [ 100 >> ]  |
|                         88     92     96    100    104    108    112                              |
|                                         ( | Needle )                                              |
|                                                                                                   |
|  PRESETS:   +-------+   +-------+   +-------+   +-------+   +-------+   +-------+  ( TUNE KNOB )  |
|             |  [1]  |   |  [2]  |   |  [3]  |   |  [4]  |   |  [5]  |   |  [6]  |       ((O))     |
|             +-------+   +-------+   +-------+   +-------+   +-------+   +-------+  [Surfing Dial] |
+---------------------------------------------------------------------------------------------------+
```

### B. Portrait Mobile Layout (Adaptive Stacking)
```
+---------------------------------------------+
| [ POWER ]                       ( VOL/MUTE )|
| +-----------------------------------------+ |
| | >>> BAND: [ROCK] | 1,420 / 63,480 <<<   | |
| | [STEREO] 98.5 CLASSIC ROCK - 128kbps    | |
| +-----------------------------------------+ |
|                                             |
| [ SEARCH: [ Station / Tag... ] ]  [ Clear ] |
|                                             |
| BANDS: [ALL] [ROCK] [JAZZ] [ELECTRO] [POP]  |
|                                             |
| |=======|=======|=======|=======|=======|   |
| 88      92      96     100     104    108   |
|                   ( | Needle )              |
| [ << 100 ]  [ < TUNE ]  [ TUNE > ]  [ 100 >>]|
|                                             |
| +-------------+  +-------------+            |
| |     [1]     |  |     [2]     |            |
| +-------------+  +-------------+            |
| +-------------+  +-------------+            |
| |     [3]     |  |     [4]     |            |
| +-------------+  +-------------+            |
| +-------------+  +-------------+            |
| |     [5]     |  |     [6]     |            |
| +-------------+  +-------------+            |
+---------------------------------------------+
```

---

## 4. Target Platforms & Build Matrix

```
                       +-------------------------------+
                       |      Shared Rust Core         |
                       |  - api.rs / presets.rs        |
                       |  - audio.rs / LiveStreamReader|
                       |  - bands.rs & state engine    |
                       |  - responsive layout engine   |
                       +-------------------------------+
                                       |
    +----------------------------------+----------------------------------+
    |                                  |                                  |
    v                                  v                                  v
+-----------------------+  +-----------------------+  +-----------------------+
|   Desktop Targets     |  |   WebAssembly Target  |  |    Mobile Targets     |
| - Windows (MSVC)      |  | - wasm32-unknown-     |  | - Android (cargo-ndk) |
| - macOS (Intel/Apple) |  |   emscripten          |  | - iOS (cargo-apple)   |
| - Linux (X11/Wayland) |  | - HTML5 WebAudio      |  | - Background Services |
| - High-DPI & Resize   |  | - Web canvas resize   |  | - Lockscreen controls |
+-----------------------+  +-----------------------+  +-----------------------+
```

---

## 5. Phased Implementation Roadmap

### Phase 1: Core Foundation & Desktop Audio MVP (Completed)
- [x] Resolve all compiler and Clippy lints (`warnings = "deny"`).
- [x] Build seekable `LiveStreamReader` buffering asynchronous HTTP streams.
- [x] Implement non-blocking `AudioController` channel.
- [x] Initial single-channel playback verification in main loop.

### Phase 2: Responsive Engine & Vintage State Model (Current Target)
- [ ] **Responsive Layout Engine (`src/layout.rs`)**:
  - Implement dynamic proportional coordinate calculators based on `GetScreenWidth()` and `GetScreenHeight()`.
  - Automatic Landscape vs. Portrait layout switching.
  - Dynamic font sizing and scalable touch target margins.
- [ ] **State & Presets (`src/presets.rs`, `src/bands.rs`)**:
  - Waveband definitions and fast indexed tag filtering.
  - 6 preset slots with serialization to `presets.json`.
- [ ] **Vintage UI Controls (`src/controls/vintage_ui.rs`)**:
  - Backlit glass display with live scope counter (`X / Y (Total: ~60k)`).
  - Prominent search bar with quick clear.
  - Waveband push-button row.
  - Horizontal frequency scale with draggable needle and click-to-jump.
  - Coarse (`<< 100` / `100 >>`) and fine (`< TUNE >`) step buttons.
  - 6 mechanical push buttons with active indicators and hold-to-save.

### Phase 3: Polish, Input Synergy & Audio Enhancements
- [ ] Keyboard shortcuts: Number keys `1`–`6` for presets, `Left`/`Right` arrows for tuning, `Space` for play/stop, `M` for mute.
- [ ] Smooth mouse wheel / touch drag over dial and rotary knobs.
- [ ] Error recovery & visual buffering spinner on the vintage display.

### Phase 4: Mobile & WebAssembly Deployment
- [ ] **WebAssembly**: Build pipeline for `wasm32-unknown-emscripten` with responsive canvas resizing.
- [ ] **Android**: `cargo-ndk` setup, `MediaSessionService` foreground playback, lock-screen notification, and `WAKE_LOCK`.
- [ ] **iOS**: `cargo-apple` Xcode project, `AVAudioSessionCategoryPlayback`, and `MPNowPlayingInfoCenter`.

### Phase 5: CI/CD Packaging Matrix
- [ ] GitHub Actions workflow building automated releases for Windows (`.exe`), macOS, Linux, Web (`index.html`/wasm), Android (`.apk`), and iOS (`.ipa`).
