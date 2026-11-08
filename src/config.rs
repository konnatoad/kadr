use crate::keybinds::KeyBindings;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub window: WindowConfig,
    pub viewer: ViewerConfig,
    pub slideshow: SlideshowConfig,
    pub keybinds: KeyBindings,
    #[serde(default)]
    pub last_paths: Vec<PathBuf>,
    pub show_thumbnails: bool,
    pub thumbnail_size: f32,
    pub filter_images: bool,
    pub filter_videos: bool,
    #[serde(skip)]
    pub scan_subfolders: bool,
    #[serde(default = "default_true")]
    pub remember_last_folder: bool,
    #[serde(default)]
    pub preferred_monitor: usize,
    #[serde(default)]
    pub loop_videos: bool,
    pub upload: UploadConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UploadConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub folder_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub width: f32,
    pub height: f32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub maximized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerConfig {
    pub sort_mode: crate::fs::sorter::SortMode,
    pub sort_descending: bool,
    pub background_color: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideshowConfig {
    pub interval_secs: f64,
    pub loop_mode: bool,
    pub random_order: bool,
    pub lua_script: String,
    #[serde(default = "default_transition_secs")]
    pub transition_secs: f32,
}

fn default_transition_secs() -> f32 {
    0.5
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig {
                width: 1280.0,
                height: 800.0,
                x: None,
                y: None,
                maximized: false,
            },
            viewer: ViewerConfig {
                sort_mode: crate::fs::sorter::SortMode::Name,
                sort_descending: false,
                background_color: [0.08, 0.08, 0.08],
            },
            slideshow: SlideshowConfig {
                interval_secs: 3.0,
                loop_mode: true,
                random_order: false,
                lua_script: String::new(),
                transition_secs: 0.5,
            },
            keybinds: KeyBindings::default(),
            last_paths: Vec::new(),
            show_thumbnails: true,
            thumbnail_size: 80.0,
            filter_images: true,
            filter_videos: true,
            scan_subfolders: false,
            remember_last_folder: true,
            preferred_monitor: 0,
            loop_videos: false,
            upload: UploadConfig::default(),
        }
    }
}

fn default_true() -> bool {
    true
}

/// The directory that holds `config.toml`: `%APPDATA%\kadr` on Windows,
/// `$XDG_CONFIG_HOME/kadr` (usually `~/.config/kadr`) everywhere else.
pub fn config_dir() -> PathBuf {
    #[cfg(windows)]
    return appdata_dir();
    #[cfg(not(windows))]
    return xdg_dir("XDG_CONFIG_HOME", ".config");
}

/// The directory for crash logs and breadcrumbs: the same `%APPDATA%\kadr` on
/// Windows, `$XDG_STATE_HOME/kadr` (usually `~/.local/state/kadr`) elsewhere.
pub fn state_dir() -> PathBuf {
    #[cfg(windows)]
    return appdata_dir();
    #[cfg(not(windows))]
    return xdg_dir("XDG_STATE_HOME", ".local/state");
}

/// `%APPDATA%\kadr`, or `kadr` in the current directory if `APPDATA` is not set.
#[cfg(windows)]
fn appdata_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    base.join("kadr")
}

/// `$<var>/kadr`, else `~/<fallback>/kadr`. The XDG spec says a relative path
/// in the variable is invalid and must be ignored. With no `HOME` either, it's
/// `kadr` in the current directory, like Windows without `APPDATA`.
#[cfg(not(windows))]
fn xdg_dir(var: &str, fallback: &str) -> PathBuf {
    let base = std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(fallback)))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    base.join("kadr")
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        config_dir().join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(text) = std::fs::read_to_string(&path) {
            toml::from_str(&text).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)?;
        write_private(&path, text.as_bytes())?;
        Ok(())
    }
}

/// The config holds the upload API key, so on unix only its owner may read it.
/// (`%APPDATA%` on Windows is already private to the user.)
#[cfg(unix)]
fn write_private(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    // mode() only applies when the file is new; an older one may still be 0644
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(bytes)
}

#[cfg(not(unix))]
fn write_private(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)
}
