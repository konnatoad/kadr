# kadr

Fast, minimal image and video viewer built with Rust and egui.

## Features

- Images: JPEG, PNG, GIF, WebP, BMP, TIFF, AVIF, HEIC, ICO and more
- RAW: CR2, CR3, NEF, ARW, DNG, ORF, RAF and other common RAW formats
- Video: MP4, MKV, AVI, MOV, WebM and more (in-app playback via libmpv: the installer bundles it on Windows, the system's mpv on Linux)
- Thumbnail strip with lazy loading
- Folder and file scanning with optional subfolder traversal
- Sort by name, date, or size
- Zoom, pan, rotate, flip — all non-destructive except explicit save
- Slideshow with configurable interval, random order, and Lua scripting
- Combine folders utility
- Right-click context menu integration (Windows, optional, set via installer)
- Configurable keyboard shortcuts
- Multi-monitor support — choose which monitor to open on (Windows; on Linux, see [below](#opening-on-a-chosen-monitor))
- Crash reports — if kadr crashes, it says why in a popup and keeps a log (see [Configuration](#configuration))
- Drag-and-drop to open files and folders

## Default keyboard shortcuts

| Key | Action |
|-----|--------|
| `Arrow Right` / `Arrow Left` | Next / previous image |
| `Space` | Toggle zoom (fit / 100 %) |
| `+` / `-` | Zoom in / out |
| `0` | Reset zoom |
| `Arrow Up/Down/Left/Right` | Pan (when zoomed in) |
| `R` | Rotate 90° clockwise |
| `Shift+R` | Rotate 90° counter-clockwise |
| `H` | Flip horizontal |
| `V` | Flip vertical |
| `F11` | Toggle fullscreen |
| `T` | Toggle thumbnail strip |
| `S` | Toggle slideshow |
| `Delete` | Delete current file |
| `Ctrl+O` | Open folder |
| `Ctrl+Shift+O` | Open file |
| `Ctrl+E` | Combine folders |
| `Ctrl+,` | Settings |
| `Ctrl+Q` | Quit |

### Video shortcuts (when a video is the current entry)

| Key | Action |
|-----|--------|
| `Space` | Play / pause |
| `Arrow Left` | Seek back 5 s |
| `Arrow Right` | Seek forward 5 s |
| `Arrow Up` | Volume +5 % |
| `Arrow Down` | Volume -5 % |
| `PageUp` / `PageDown` | Previous / next file in folder |

All shortcuts are re-bindable from Settings (`Ctrl+,`).

## Building

### Windows

Prerequisites: Rust 1.85+ (2024 edition), Windows SDK.

```powershell
# Build kadr only
cargo build --release -p kadr

# The binary is at target/release/kadr.exe
```

### Linux

Prerequisites: Rust 1.85+ (2024 edition), a C compiler, pkg-config, and the
development files of libmpv, libheif and libraw. `nasm` is optional: with it,
JPEG decoding uses SIMD. `zenity` shows the crash popup.

```sh
# Arch
sudo pacman -S --needed base-devel mpv libheif libraw nasm zenity
# Debian / Ubuntu
sudo apt install build-essential pkg-config libmpv-dev libheif-dev libraw-dev nasm zenity

cargo build --release -p kadr --features libheif,libraw-native

# The binary is at target/release/kadr
```

On Arch, `packaging/PKGBUILD` builds and installs kadr with its menu entry and icon:

```sh
cd packaging && makepkg -si
```

Elsewhere, copy `target/release/kadr` to `~/.local/bin/`, `packaging/kadr.desktop`
to `~/.local/share/applications/` and `packaging/kadr.svg` to
`~/.local/share/icons/hicolor/scalable/apps/`.

#### Opening on a chosen monitor

Wayland doesn't let an app place its own window, so on Linux the window manager
decides where kadr opens. To keep it on one monitor, add a window rule for the
window class `kadr`: in Hyprland a `monitor` window rule, in Sway
`for_window [app_id="kadr"] move to output DP-1`.

## Configuration

Config is written automatically on exit, to `%APPDATA%\kadr\config.toml` on Windows
and `~/.config/kadr/config.toml` (`$XDG_CONFIG_HOME`) on Linux. Crash logs go to
`%APPDATA%\kadr\` on Windows and `~/.local/state/kadr/` (`$XDG_STATE_HOME`) on Linux.
