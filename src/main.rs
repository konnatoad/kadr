#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod fs;
mod icon;
mod keybinds;
mod media;
mod slideshow;
mod ui;

use anyhow::Result;
use app::KadrApp;

fn main() -> Result<()> {
    if std::env::var("KADR_LOG").is_ok() {
        env_logger::init();
    }

    let open_path = std::env::args().nth(1).map(std::path::PathBuf::from);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("kadr")
            .with_min_inner_size([640.0, 480.0])
            .with_inner_size([1280.0, 800.0])
            .with_drag_and_drop(true)
            .with_icon(std::sync::Arc::new(icon::egui_icon())),
        ..Default::default()
    };

    eframe::run_native(
        "kadr",
        native_options,
        Box::new(|cc| Ok(Box::new(KadrApp::new(cc, open_path)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))
}
