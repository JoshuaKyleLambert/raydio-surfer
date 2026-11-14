# RaydioSurfer

RaydioSurfer is a tactile desktop internet radio player styled after a vintage car stereo / dash head unit. Built with Rust and Raylib, it connects to the global Radio Browser directory of over 60,000 live stations worldwide, combining retro mechanical aesthetics with modern streaming technology and digital signal processing.

---

## Features

- **Vintage Car-Stereo Interface**: Custom undecorated UI featuring an illuminated LCD station marquee, retro analog tuning dial with a sweep needle, waveband selector switches, and mechanical push buttons.
- **60,000+ Live Radio Stations**: Integrated search and catalog filtering via the Radio Browser API by genre, tag, country, or keyword.
- **6 Quick Presets**: Store your favorite stations into 6 quick-access presets (hold to save, click to tune).
- **Custom Waveband Slots**: Switch between preset genres (Rock, Jazz, Electronic, Classical, etc.) or right-click to map a band slot to a custom search query.
- **10-Band Graphic Equalizer & DSP**: Integrated audio DSP pipeline with preamp gain (-12 dB to +12 dB), stereo balance control, and a 10-band biquad equalizer (31 Hz – 16 kHz) housed in an expandable bottom drawer.
- **Robust Audio Streaming**: Powered by Rodio and Symphonia with asynchronous streaming (`LiveStreamReader`) supporting direct HTTP streams, HLS (`.m3u8`), M3U, PLS, XSPF, and ASX playlist formats.
- **Responsive Layout & Window Persistence**: Adapts dynamically across window sizes and multi-monitor setups; automatically saves and restores window geometry, volume, DSP preferences, and the last tuned station.

---

## Tech Stack

- **Language**: Rust (2024 Edition)
- **GUI & Graphics**: [Raylib 6.0](https://www.raylib.com/) (Rust bindings via `raylib` with `raygui`)
- **Audio Playback & DSP**: [Rodio 0.23](https://github.com/RustAudio/rodio) with `symphonia-all` decoders and custom biquad DSP filters
- **Networking & Async Runtime**: [Reqwest 0.13](https://github.com/seanmonstar/reqwest), [Tokio 1.54](https://tokio.rs), [Futures-Util](https://docs.rs/futures-util)
- **Radio Catalog API**: [radiobrowser 0.6](https://crates.io/crates/radiobrowser) (Community Radio Browser directory)
- **Serialization & State**: [Serde](https://serde.rs), [Serde JSON](https://crates.io/crates/serde_json), [directories](https://crates.io/crates/directories)
- **Package Manager & Build**: Cargo, `winres` (Windows application icon embedding)
- **Asset Generation**: Python 3 standard library (pure math procedural PNG/ICO icon generator)

---

## Requirements

### Prerequisites

1. **Rust Toolchain**: Rust 1.85+ (supporting the 2024 edition) with `cargo`.
   - Install via [rustup.rs](https://rustup.rs/):
     ```bash
     curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
     ```

2. **C/C++ Build Tools & Native Dependencies**:
   - **Windows**: Visual Studio C++ Build Tools (MSVC) or MinGW-w64.
   - **Linux**: Build essentials, ALSA audio development libraries, and X11/OpenGL packages:
     ```bash
     sudo apt update
     sudo apt install -y build-essential pkg-config libasound2-dev libx11-dev \
                         libxcursor-dev libxrandr-dev libxinerama-dev libxi-dev libgl1-mesa-dev
     ```
   - **macOS**: Xcode Command Line Tools:
     ```bash
     xcode-select --install
     ```

3. **Python 3** *(Optional)*:
   - Only required if you wish to re-generate the application icons from `assets/generate_icon.py`.

---

## Setup & Run

### 1. Clone the Repository
```bash
git clone https://github.com/your-username/raydio-surfer.git
cd raydio-surfer
```

### 2. Run in Development Mode
```bash
cargo run
```

### 3. Build for Release
```bash
cargo build --release
```
The compiled executable will be located at:
- **Windows**: `target/release/RaydioSurfer.exe`
- **Linux/macOS**: `target/release/RaydioSurfer`

---

## Scripts & Development Tools

### Procedural Icon Generator
RaydioSurfer generates its application icons (`assets/icon.png` and `assets/icon.ico`) procedurally using pure Python standard library routines (no Pillow or external dependencies needed):
```bash
python3 assets/generate_icon.py
```

### Git Pre-Commit Hook
A pre-commit hook is provided under `.githooks/` to ensure formatting (`cargo fmt`) and clippy lints (`cargo clippy`) pass before every commit.

To activate the hook:
```bash
git config core.hooksPath .githooks
```

### Linux Desktop & Launcher Integration
To install the application icon and `.desktop` launcher entry for your current user on Linux:

```bash
# 1. Install application icon to the hicolor icon theme
install -Dm644 assets/icon.png ~/.local/share/icons/hicolor/256x256/apps/raydiosurfer.png

# 2. Install desktop entry (ensure RaydioSurfer is in your PATH or update Exec path)
install -Dm644 packaging/RaydioSurfer.desktop ~/.local/share/applications/RaydioSurfer.desktop

# 3. Update desktop and icon caches
update-desktop-database ~/.local/share/applications || true
gtk-update-icon-cache ~/.local/share/icons/hicolor || true
```

---

## Environment Variables & Configuration

### Environment Variables
RaydioSurfer does not require mandatory custom environment variables for typical operation. Standard Rust / Cargo variables are supported:
- `RUST_LOG`: Configure logging levels for diagnostics (e.g. `RUST_LOG=debug`).
- `RUST_BACKTRACE`: Set to `1` or `full` to enable detailed backtraces on panics.
- Standard C/cross-compilation flags (`CC`, `CXX`, `PKG_CONFIG_PATH`) when compiling Raylib on non-standard setups.

### Configuration & Cache Files
Application state is saved using standard OS application data paths (via the `directories` crate):

| Platform | Settings File (`settings.json`) | Station Cache (`stations_cache.json`) |
|---|---|---|
| **Windows** | `%APPDATA%\RaydioSurfer\RaydioSurfer\config\settings.json` | `%LOCALAPPDATA%\RaydioSurfer\RaydioSurfer\cache\stations_cache.json` |
| **Linux** | `~/.config/RaydioSurfer/settings.json` | `~/.cache/RaydioSurfer/stations_cache.json` |
| **macOS** | `~/Library/Application Support/com.RaydioSurfer.RaydioSurfer/settings.json` | `~/Library/Caches/com.RaydioSurfer.RaydioSurfer/stations_cache.json` |

*Note: If standard configuration directories are unavailable, files fall back to the current working directory.*

---

## Testing & Quality Assurance

RaydioSurfer includes a comprehensive test suite covering audio decoders, playlist parsers, DSP biquad filtering, UI marquee/layout logic, window management, and JSON storage serialization.

### Run All Unit and Integration Tests
```bash
cargo test
```

### Run Linter Checks
```bash
cargo clippy --all-targets --all-features
```

### Check Code Formatting
```bash
cargo fmt -- --check
```

---

## Project Structure

```
raydio-surfer/
├── .githooks/
│   └── pre-commit              # Git pre-commit script running fmt and clippy checks
├── assets/
│   ├── generate_icon.py        # Standalone Python script to generate icons
│   ├── icon.ico                # Multi-resolution Windows executable icon
│   └── icon.png                # Embedded 256x256 RGBA application icon
├── packaging/
│   ├── README.md               # Desktop integration instructions
│   └── RaydioSurfer.desktop    # Linux desktop launcher entry
├── src/
│   ├── controls/
│   │   ├── confirm_exit.rs     # Exit confirmation modal dialog
│   │   ├── mod.rs              # UI controls module exports
│   │   ├── search_box.rs       # Search input widget
│   │   └── vintage_ui.rs       # Vintage dashboard layout and renderer
│   ├── api.rs                  # Radio Browser API client and station cache loader
│   ├── audio.rs                # Rodio controller, playlist parsers, & stream buffering
│   ├── bands.rs                # Waveband definitions and genre matching logic
│   ├── dsp.rs                  # Equalizer biquad filters and DSP audio processing
│   ├── layout.rs               # Responsive UI coordinates and dynamic rectangle layout
│   ├── main.rs                 # Main application entry point and event loop
│   ├── paths.rs                # Cross-platform config/cache directory path resolver
│   ├── presets.rs              # 6-channel preset station storage
│   ├── settings.rs             # Configuration schema, migration, and persistence
│   ├── storage.rs              # JSON serialization helpers and atomic file writes
│   └── window_frame.rs         # Custom window frame, resizing, and dragging logic
├── build.rs                    # Windows build script for embedding application icon
├── Cargo.toml                  # Rust project manifest and dependencies
├── Cargo.lock                  # Pinned dependency lockfile
├── LICENSE-APACHE              # Apache License 2.0
├── LICENSE-MIT                 # MIT License
├── PLAN.md                     # Architecture specification and roadmap
└── README.md                   # Project documentation
```

---

## License

This project is dual-licensed under either:

- **MIT License** ([LICENSE-MIT](LICENSE-MIT))
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE))

at your option.
