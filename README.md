# kadr

Fast, minimal image and video viewer built with Rust and egui.

## Features

- Images: JPEG, PNG, GIF, WebP, BMP, TIFF, AVIF, HEIC, ICO and more
- RAW: CR2, CR3, NEF, ARW, DNG, ORF, RAF and other common RAW formats
- Video: MP4, MKV, AVI, MOV, WebM and more (in-app playback via libmpv, installed alongside kadr)
- Thumbnail strip with lazy loading
- Folder and file scanning with optional subfolder traversal
- Sort by name, date, or size
- Zoom, pan, rotate, flip — all non-destructive except explicit save
- Slideshow with configurable interval, random order, and Lua scripting
- Combine folders utility
- Right-click context menu integration (optional, set via installer)
- Configurable keyboard shortcuts
- Multi-monitor support — choose which monitor to open on
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

Prerequisites: Rust 1.85+ (2024 edition), Windows SDK.

```powershell
# Build kadr only
cargo build --release -p kadr

# The binary is at target/release/kadr.exe
```

## Configuration

Config is stored at `%APPDATA%\kadr\config.toml` and is written automatically on exit.
