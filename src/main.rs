#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod config;
mod crash;
mod fs;
#[cfg(feature = "libheif")]
mod heif;
mod icon;
mod keybinds;
#[cfg(feature = "libraw-native")]
mod libraw_native;
mod media;
// Windows only: Wayland doesn't let an app place its own window
#[cfg(windows)]
mod monitor;
mod slideshow;
mod ui;
mod upload;
mod video;

use anyhow::Result;
use app::KadrApp;

#[cfg(not(any(windows, target_os = "linux")))]
compile_error!("kadr is built for Windows and Linux");

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--symbolize") => {
            if let Some(log) = args.next() {
                crash::symbolize(std::path::Path::new(&log));
            }
            return Ok(());
        }
        // what kadr's Linux crash handler turns the crashed process into
        #[cfg(not(windows))]
        Some("--crash-report") => {
            if let Some(pending) = args.next() {
                std::process::exit(crash::report_pending(std::path::Path::new(&pending)));
            }
            return Ok(());
        }
        _ => {}
    }
    crash::install();
    if std::env::args().nth(1).as_deref() == Some("--crash-test") {
        crash::test(std::env::args().nth(2).as_deref().unwrap_or(""));
        // only the usage message gets here, and that's not a crash
        crash::mark_normal_exit();
        return Ok(());
    }
    crash::symbolize_pending();
    if std::env::args().nth(1).as_deref() == Some("--com-test") {
        crash::enable_com_test();
    }

    if std::env::var("KADR_LOG").is_ok() {
        env_logger::init();
    }

    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("kadr {}", env!("CARGO_PKG_VERSION"));
        crash::mark_normal_exit();
        return Ok(());
    }

    let open_path = std::env::args()
        .nth(1)
        .filter(|a| !a.starts_with("--"))
        .map(std::path::PathBuf::from);
    let config = config::AppConfig::load();

    const WIN_W: f32 = 1280.0;
    const WIN_H: f32 = 800.0;

    let viewport = egui::ViewportBuilder::default()
        .with_title("kadr")
        .with_min_inner_size([640.0, 480.0])
        .with_inner_size([WIN_W, WIN_H])
        .with_drag_and_drop(true)
        .with_icon(std::sync::Arc::new(icon::egui_icon()));

    #[cfg(windows)]
    let viewport = match monitor::initial_position(config.preferred_monitor, WIN_W, WIN_H) {
        Some(pos) => viewport.with_position(pos),
        None => viewport,
    };

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
