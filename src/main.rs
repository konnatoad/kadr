#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod fs;
#[cfg(feature = "libheif")]
mod heif;
mod icon;
mod keybinds;
#[cfg(feature = "libraw-native")]
mod libraw_native;
mod media;
mod monitor;
mod slideshow;
mod ui;
mod video;

use anyhow::Result;
use app::KadrApp;

fn main() -> Result<()> {
    if std::env::var("KADR_LOG").is_ok() {
        env_logger::init();
    }

    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("kadr {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let open_path = std::env::args().nth(1).map(std::path::PathBuf::from);
    let config = config::AppConfig::load();

    const WIN_W: f32 = 1280.0;
    const WIN_H: f32 = 800.0;

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("kadr")
        .with_min_inner_size([640.0, 480.0])
        .with_inner_size([WIN_W, WIN_H])
        .with_drag_and_drop(true)
        .with_icon(std::sync::Arc::new(icon::egui_icon()));

    if let Some(pos) = monitor::initial_position(config.preferred_monitor, WIN_W, WIN_H) {
        viewport = viewport.with_position(pos);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        "kadr",
        native_options,
        Box::new(|cc| Ok(Box::new(KadrApp::new(cc, open_path, config)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))
}
