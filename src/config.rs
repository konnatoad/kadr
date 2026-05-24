use crate::keybinds::KeyBindings;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf; // used by last_path and config_path

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub window: WindowConfig,
    pub viewer: ViewerConfig,
    pub slideshow: SlideshowConfig,
    pub keybinds: KeyBindings,
    pub last_path: Option<PathBuf>,
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

fn default_transition_secs() -> f32 { 0.5 }

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
            last_path: None,
            show_thumbnails: true,
            thumbnail_size: 80.0,
            filter_images: true,
            filter_videos: true,
            scan_subfolders: false,
            remember_last_folder: true,
            preferred_monitor: 0,
        }
    }
}

fn default_true() -> bool { true }

impl AppConfig {
    pub fn config_path() -> PathBuf {
        let dir = dirs_next();
        dir.join("kadr").join("config.toml")
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
        std::fs::write(&path, text)?;
        Ok(())
    }
}

fn dirs_next() -> PathBuf {
    if let Some(dir) = std::env::var_os("APPDATA") {
        PathBuf::from(dir)
    } else {
        std::env::current_dir().unwrap_or_default()
    }
}
