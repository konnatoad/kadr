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

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = match info.payload().downcast_ref::<&str>() {
            Some(s) => s.to_string(),
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => s.clone(),
                None => "unknown panic".to_string(),
            },
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        show_fatal_error_popup(&format!(
            "kadr ran into a problem and needs to close.\n\n{msg}\n\nat {location}"
        ));
    }));
}

#[cfg(windows)]
fn show_fatal_error_popup(text: &str) {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::winuser::{MB_ICONERROR, MB_OK, MB_SYSTEMMODAL, MessageBoxW};

    let wide_text: Vec<u16> = OsStr::new(text).encode_wide().chain(once(0)).collect();
    let wide_title: Vec<u16> = OsStr::new("kadr - Fatal Error").encode_wide().chain(once(0)).collect();

    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide_text.as_ptr(),
            wide_title.as_ptr(),
            MB_OK | MB_ICONERROR | MB_SYSTEMMODAL,
        );
    }
}

#[cfg(not(windows))]
fn show_fatal_error_popup(text: &str) {
    eprintln!("kadr - Fatal Error: {text}");
}

fn main() -> Result<()> {
    install_panic_hook();

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
