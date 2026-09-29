# Icons & desktop integration

The application icon lives in [`assets/`](../assets):

- `assets/icon.png` — 256×256 RGBA icon, embedded into the binary and applied at
  runtime via `set_window_icon` (shows in the taskbar / dock, Alt-Tab, and the
  window title bar). No installation step is required for this.
- `assets/icon.ico` — multi-resolution Windows icon (16–256 px). It is embedded
  into the `.exe` at build time by `build.rs`, so Explorer, the taskbar, and the
  pinned launcher show the icon.

Both files are generated procedurally (no binary editing tools needed):

```bash
python3 assets/generate_icon.py
```

## Linux launcher (clickable menu / dock entry)

`RaydioSurfer.desktop` provides a launcher entry. To install it for the current
user:

```bash
# 1. Install the icon under the hicolor theme
install -Dm644 assets/icon.png \
  ~/.local/share/icons/hicolor/256x256/apps/raydiosurfer.png

# 2. Install the desktop entry (make sure RaydioSurfer is on your PATH,
#    or edit Exec= to point at the built binary)
install -Dm644 packaging/RaydioSurfer.desktop \
  ~/.local/share/applications/RaydioSurfer.desktop

# 3. Refresh caches (optional)
update-desktop-database ~/.local/share/applications || true
gtk-update-icon-cache ~/.local/share/icons/hicolor || true
```

The app will then appear in the application menu with the icon and can be
launched (and pinned) with a click.
