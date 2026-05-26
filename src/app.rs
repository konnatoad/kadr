use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use egui::{Color32, ColorImage, TextureHandle, Vec2, vec2};

use crate::config::AppConfig;
use crate::fs::combine::combine_folders;
use crate::fs::scanner::{ScanOptions, scan_folder};
use crate::fs::sorter::sort_entries;
use crate::keybinds::KeyAction;
use crate::media::formats::{MediaEntry, MediaType};
use crate::media::loader::{
    LoadedImage, apply_flip_horizontal, apply_flip_vertical, apply_rotation, save_image,
    try_exif_thumbnail_from_bytes, load_jpeg_from_bytes,
};
use crate::slideshow::engine::{SlideshowEngine, TickResult};
use crate::slideshow::lua_script::LuaSlideshowScript;
use crate::slideshow::lua_script::SlideContext;
use crate::ui::combine_dialog::{CombineAction, CombineDialog};
use crate::ui::lua_editor::{LuaEditor, LuaEditorAction};
use crate::ui::settings_dialog::{SettingsAction, SettingsDialog};
use crate::ui::thumbnail_strip::{ThumbEntry, ThumbnailStrip};
use crate::ui::toolbar::show_toolbar;
use crate::ui::viewer::{TransitionData, ViewerState, show_viewer};
use crate::ui::video_controls::{ControlsAction, show_video_controls};
use crate::video::VideoContext;

const THUMB_CACHE_LIMIT: usize = 200;

pub struct KadrApp {
    thumb_order: Vec<usize>,
    config: AppConfig,
    entries: Vec<MediaEntry>,
    current_index: usize,
    current_texture: Option<TextureHandle>,
    thumb_textures: HashMap<usize, TextureHandle>,
    viewer_state: ViewerState,
    slideshow: SlideshowEngine,
    lua_script: Option<LuaSlideshowScript>,
    fullscreen: bool,
    combine_dialog: CombineDialog,
    settings_dialog: SettingsDialog,
    lua_editor: LuaEditor,
    loading: Arc<Mutex<Option<LoadResult>>>,
    /// EXIF thumbnail slot — filled fast (first 256 KB of file) before full decode finishes.
    thumb_loading: Arc<Mutex<Option<LoadResult>>>,
    /// Background slot for the *next* image — filled as soon as the current one loads.
    preload_loading: Arc<Mutex<Option<LoadResult>>>,
    /// Ready-to-use preloaded texture (promoted from preload_loading).
    preload_texture: Option<TextureHandle>,
    /// Which index is sitting in the preload slot (in-flight or ready).
    preload_index: Option<usize>,
    /// Stored so background threads can call `request_repaint()`.
    egui_ctx: egui::Context,
    status_msg: Option<(String, std::time::Instant)>,
    video_ctx: Option<VideoContext>,
    video_texture: Option<TextureHandle>,
    video_volume: f64,
    thumb_pending: std::collections::HashSet<usize>,
    thumb_results: Arc<Mutex<Vec<(usize, ColorImage)>>>,
    /// The outgoing image, kept alive during a crossfade.
    prev_texture: Option<TextureHandle>,
    /// Pixel dimensions of `prev_texture`.
    prev_image_size: Vec2,
    /// Zoom the outgoing image had at the moment the transition started.
    prev_zoom: f32,
    /// Pan offset the outgoing image had at the moment the transition started.
    prev_offset: Vec2,
    /// Crossfade progress: 0.0 = fully prev, 1.0 = fully current.
    transition_t: f32,
    // Physical monitor rect to snap the window to on the first frame.
    // Using SetWindowPos (physical px) is more reliable than with_position()
    // which can be overridden by Windows' "remember window locations" feature.
    #[cfg(windows)]
    monitor_snap: Option<(i32, i32, u32, u32)>,
}

struct LoadResult {
    index: usize,
    image: Option<ColorImage>,
    error: Option<String>,
}

impl KadrApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        open_path: Option<PathBuf>,
        config: AppConfig,
    ) -> Self {
        apply_theme(&cc.egui_ctx);

        let mut app = Self {
            thumb_order: Vec::new(),
            config: config.clone(),
            entries: Vec::new(),
            current_index: 0,
            current_texture: None,
            thumb_textures: HashMap::new(),
            viewer_state: ViewerState::default(),
            slideshow: SlideshowEngine::new(&config.slideshow),
            lua_script: LuaSlideshowScript::from_str(&config.slideshow.lua_script).ok(),
            fullscreen: false,
            combine_dialog: CombineDialog::default(),
            settings_dialog: SettingsDialog {
                show_thumbnails: config.show_thumbnails,
                scan_subfolders: config.scan_subfolders,
                filter_images: config.filter_images,
                filter_videos: config.filter_videos,
                sort_mode: config.viewer.sort_mode.clone(),
                remember_last_folder: config.remember_last_folder,
                preferred_monitor: config.preferred_monitor,
                bg_color: config.viewer.background_color,
                thumb_size: config.thumbnail_size,
                slideshow_interval: config.slideshow.interval_secs,
                slideshow_transition: config.slideshow.transition_secs,
                slideshow_loop: config.slideshow.loop_mode,
                slideshow_random: config.slideshow.random_order,
                lua_code: config.slideshow.lua_script.clone(),
                ..Default::default()
            },
            lua_editor: LuaEditor::default(),
            loading: Arc::new(Mutex::new(None)),
            thumb_loading: Arc::new(Mutex::new(None)),
            preload_loading: Arc::new(Mutex::new(None)),
            preload_texture: None,
            preload_index: None,
            egui_ctx: cc.egui_ctx.clone(),
            status_msg: None,
            video_ctx: None,
            video_texture: None,
            video_volume: 1.0,
            thumb_pending: std::collections::HashSet::new(),
            thumb_results: Arc::new(Mutex::new(Vec::new())),
            prev_texture: None,
            prev_image_size: Vec2::ZERO,
            prev_zoom: 1.0,
            prev_offset: Vec2::ZERO,
            transition_t: 1.0,
            #[cfg(windows)]
            monitor_snap: if config.preferred_monitor > 0 {
                crate::monitor::enumerate()
                    .get(config.preferred_monitor - 1)
                    .map(|m| (m.x, m.y, m.width, m.height))
            } else {
                None
            },
        };

        let restore = if config.remember_last_folder {
            config.last_path.clone()
        } else {
            None
        };
        if let Some(path) = open_path.or(restore) {
            app.open_path(path);
        }

        app
    }

    fn open_path(&mut self, path: PathBuf) {
        let opts = ScanOptions {
            include_images: self.config.filter_images,
            include_videos: self.config.filter_videos,
            recursive: self.config.scan_subfolders,
        };

        let mut entries = if path.is_file() {
            let folder = path.parent().unwrap_or(&path).to_path_buf();
            scan_folder(&folder, &opts)
        } else {
            scan_folder(&path, &opts)
        };

        sort_entries(&mut entries, &self.config.viewer.sort_mode);

        let start_index = if path.is_file() {
            entries.iter().position(|e| e.path == path).unwrap_or(0)
        } else {
            0
        };

        self.entries = entries;
        self.current_index = start_index;
        self.thumb_textures.clear();
        self.thumb_pending.clear();
        self.thumb_results.lock().unwrap().clear();
        self.current_texture = None;
        self.viewer_state.reset();
        self.video_ctx = None;
        self.video_texture = None;

        let folder = if path.is_file() {
            path.parent().unwrap_or(&path).to_path_buf()
        } else {
            path
        };
        self.config.last_path = Some(folder);

        self.load_current_image();
    }

    fn navigate(&mut self, delta: i64) {
        if self.entries.is_empty() {
            return;
        }
        let len = self.entries.len() as i64;
        if len == 0 {
            return;
        }

        let new_idx = ((self.current_index as i64 + delta).rem_euclid(len)) as usize;
        self.go_to(new_idx);
    }

    fn go_to(&mut self, index: usize) {
        if index >= self.entries.len() {
            return;
        }
        self.current_index = index;
        self.current_texture = None;
        self.viewer_state.reset();
        self.video_ctx = None;
        self.video_texture = None;
        // Cancel any in-progress crossfade — manual navigation is always instant.
        self.prev_texture = None;
        self.transition_t = 1.0;
        // Discard any preload state — it's for the wrong index now.
        self.preload_texture = None;
        self.preload_index = None;
        *self.preload_loading.lock().unwrap() = None;
        // Any in-flight EXIF thumbnail is also stale — discard it.
        *self.thumb_loading.lock().unwrap() = None;
        self.load_current_image();
    }

    /// Kick off a background load of `index` into the preload slot.
    fn start_preload(&mut self, index: usize) {
        if index >= self.entries.len() {
            return;
        }
        if self.entries[index].media_type == MediaType::Video {
            return;
        }

        let path = self.entries[index].path.clone();
        let slot = Arc::clone(&self.preload_loading);
        let expected = index;

        *slot.lock().unwrap() = None;
        self.preload_index = Some(index);
        self.preload_texture = None;

        thread::spawn(move || {
            if let Ok(img) = LoadedImage::load(&path) {
                let result = LoadResult {
                    index: expected,
                    image: Some((*img.to_egui_image()).clone()),
                    error: None,
                };

                slot.lock().unwrap().replace(result);
            }
        });
    }

    fn load_current_image(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let entry = &self.entries[self.current_index];
        if entry.media_type == MediaType::Video {
            let path = entry.path.clone();
            match VideoContext::new(&path, self.egui_ctx.clone()) {
                Ok(ctx) => {
                    ctx.set_volume(self.video_volume);
                    self.video_ctx = Some(ctx);
                }
                Err(e) => self.set_status(format!("Video error: {e}")),
            }
            return;
        }

        // Discard any thumbnail still in-flight from the previous image.
        *self.thumb_loading.lock().unwrap() = None;

        let path = entry.path.clone();
        let index = self.current_index;
        let result_slot = Arc::clone(&self.loading);
        let thumb_slot  = Arc::clone(&self.thumb_loading);
        let ctx         = self.egui_ctx.clone();

        let is_jpeg = matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase())
                .as_deref(),
            Some("jpg") | Some("jpeg") | Some("jfif")
        );

        thread::spawn(move || {
            if is_jpeg {
                // ── Single-pass JPEG load ─────────────────────────────────
                // Open the file once and read it in two steps so the EXIF
                // thumbnail can be emitted before the full 65+ MB is in RAM.
                use std::io::Read;

                let mut f = match std::fs::File::open(&path) {
                    Ok(f) => f,
                    Err(e) => {
                        *result_slot.lock().unwrap() = Some(LoadResult {
                            index, image: None, error: Some(e.to_string()),
                        });
                        ctx.request_repaint();
                        return;
                    }
                };

                // Phase 1 — read the first 256 KB and emit the EXIF thumbnail.
                let mut header = Vec::with_capacity(256 * 1024);
                let _ = f.by_ref().take(256 * 1024).read_to_end(&mut header);

                if let Some(thumb) = try_exif_thumbnail_from_bytes(&header) {
                    *thumb_slot.lock().unwrap() = Some(LoadResult {
                        index, image: Some(thumb), error: None,
                    });
                    ctx.request_repaint();
                }

                // Phase 2 — read the rest, combine, decode from memory.
                // No second file open: the OS page-cache already has the
                // first 256 KB warm, so this is one sequential read total.
                let mut tail = Vec::new();
                let _ = f.read_to_end(&mut tail);
                header.append(&mut tail);
                let full_data = header;

                let result = match load_jpeg_from_bytes(&full_data) {
                    Ok(img) => LoadResult {
                        index,
                        image: Some((*img.to_egui_image()).clone()),
                        error: None,
                    },
                    Err(e) => LoadResult {
                        index, image: None, error: Some(e.to_string()),
                    },
                };
                *result_slot.lock().unwrap() = Some(result);
                ctx.request_repaint();
            } else {
                // ── Non-JPEG (PNG, RAW, …) — existing path ────────────────
                let result = match LoadedImage::load(&path) {
                    Ok(img) => LoadResult {
                        index,
                        image: Some((*img.to_egui_image()).clone()),
                        error: None,
                    },
                    Err(e) => LoadResult {
                        index, image: None, error: Some(e.to_string()),
                    },
                };
                *result_slot.lock().unwrap() = Some(result);
                ctx.request_repaint();
            }
        });
    }

    fn load_thumb(&mut self, index: usize) {
        if self.thumb_textures.contains_key(&index)
            || self.thumb_pending.contains(&index)
            || index >= self.entries.len()
        {
            return;
        }
        let entry = &self.entries[index];
        if entry.media_type == MediaType::Video {
            return;
        }
        let path = entry.path.clone();
        let results = Arc::clone(&self.thumb_results);
        let egui_ctx = self.egui_ctx.clone();
        self.thumb_pending.insert(index);
        thread::spawn(move || {
            if let Ok(img) = LoadedImage::load(&path) {
                let thumb = make_thumbnail(img.to_egui_image(), 80);
                results.lock().unwrap().push((index, thumb));
                egui_ctx.request_repaint();
            }
        });
    }

    fn apply_sort(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let current_path = self.entries[self.current_index].path.clone();
        sort_entries(&mut self.entries, &self.config.viewer.sort_mode);
        self.thumb_textures.clear();
        self.current_index = self
            .entries
            .iter()
            .position(|e| e.path == current_path)
            .unwrap_or(0);
    }

    fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = Some((msg.into(), std::time::Instant::now()));
    }

    fn bg_color32(&self) -> Color32 {
        let [r, g, b] = self.config.viewer.background_color;
        Color32::from_rgb((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        // When on a video, arrow keys + space control playback instead of image navigation.
        let is_video = !self.entries.is_empty()
            && self.entries[self.current_index].media_type == MediaType::Video;

        if is_video {
            let mut space = false;
            let mut left = false;
            let mut right = false;
            let mut up = false;
            let mut down = false;
            let mut pgdn = false;
            let mut pgup = false;
            let mut do_fullscreen = false;
            let mut do_quit = false;

            ctx.input(|i| {
                space = i.key_pressed(egui::Key::Space);
                left = i.key_pressed(egui::Key::ArrowLeft);
                right = i.key_pressed(egui::Key::ArrowRight);
                up = i.key_pressed(egui::Key::ArrowUp);
                down = i.key_pressed(egui::Key::ArrowDown);
                pgdn = i.key_pressed(egui::Key::PageDown);
                pgup = i.key_pressed(egui::Key::PageUp);
                do_fullscreen = i.key_pressed(egui::Key::F11);
                do_quit = i.modifiers.ctrl && i.key_pressed(egui::Key::Q);
            });

            if space {
                if let Some(vc) = &self.video_ctx { vc.play_pause(); }
            }
            if left {
                if let Some(vc) = &self.video_ctx { vc.seek_relative(-5.0); }
            }
            if right {
                if let Some(vc) = &self.video_ctx { vc.seek_relative(5.0); }
            }
            if up {
                self.video_volume = (self.video_volume + 0.05).min(1.0);
                if let Some(vc) = &self.video_ctx { vc.set_volume(self.video_volume); }
            }
            if down {
                self.video_volume = (self.video_volume - 0.05).max(0.0);
                if let Some(vc) = &self.video_ctx { vc.set_volume(self.video_volume); }
            }
            if pgdn {
                self.navigate(1);
            }
            if pgup {
                self.navigate(-1);
            }
            if do_fullscreen {
                self.fullscreen = !self.fullscreen;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
            }
            if do_quit {
                let _ = self.config.save();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            return;
        }

        let image_size = self
            .current_texture
            .as_ref()
            .map(|t| Vec2::new(t.size()[0] as f32, t.size()[1] as f32))
            .unwrap_or(Vec2::splat(1.0));
        // fit_mode always fits to viewport, so overflowing is only possible in manual zoom
        let overflowing = !self.viewer_state.fit_mode
            && self
                .viewer_state
                .is_overflowing(image_size, ctx.viewport_rect().size());

        let bindings = self.config.keybinds.clone();

        let mut nav_next = false;
        let mut nav_prev = false;
        let mut do_toggle_zoom = false;
        let mut do_zoom_in = false;
        let mut do_zoom_out = false;
        let mut do_zoom_reset = false;
        let mut do_pan_up = false;
        let mut do_pan_down = false;
        let mut do_pan_left = false;
        let mut do_pan_right = false;
        let mut do_fullscreen = false;
        let mut do_toggle_thumbs = false;
        let mut do_rotate_cw = false;
        let mut do_rotate_ccw = false;
        let mut do_flip_h = false;
        let mut do_flip_v = false;
        let mut do_open_folder = false;
        let mut do_open_file = false;
        let mut do_combine = false;
        let mut do_slideshow = false;
        let mut do_settings = false;
        let mut do_quit = false;

        ctx.input(|input| {
            nav_next = bindings.is_action(&KeyAction::NextImage, input);
            nav_prev = bindings.is_action(&KeyAction::PrevImage, input);
            do_toggle_zoom = bindings.is_action(&KeyAction::ToggleZoom, input);
            do_zoom_in = bindings.is_action(&KeyAction::ZoomIn, input);
            do_zoom_out = bindings.is_action(&KeyAction::ZoomOut, input);
            do_zoom_reset = bindings.is_action(&KeyAction::ZoomReset, input);
            do_pan_up = bindings.is_action(&KeyAction::PanUp, input);
            do_pan_down = bindings.is_action(&KeyAction::PanDown, input);
            do_pan_left = bindings.is_action(&KeyAction::PanLeft, input);
            do_pan_right = bindings.is_action(&KeyAction::PanRight, input);
            do_fullscreen = bindings.is_action(&KeyAction::Fullscreen, input);
            do_toggle_thumbs = bindings.is_action(&KeyAction::ToggleThumbnails, input);
            do_rotate_cw = bindings.is_action(&KeyAction::RotateCW, input);
            do_rotate_ccw = bindings.is_action(&KeyAction::RotateCCW, input);
            do_flip_h = bindings.is_action(&KeyAction::FlipHorizontal, input);
            do_flip_v = bindings.is_action(&KeyAction::FlipVertical, input);
            do_open_folder = bindings.is_action(&KeyAction::OpenFolder, input);
            do_open_file = bindings.is_action(&KeyAction::OpenFile, input);
            do_combine = bindings.is_action(&KeyAction::CombineFolders, input);
            do_slideshow = bindings.is_action(&KeyAction::ToggleSlideshow, input);
            do_settings = bindings.is_action(&KeyAction::OpenSettings, input);
            do_quit = bindings.is_action(&KeyAction::Quit, input);
        });

        if overflowing {
            if do_pan_up {
                self.viewer_state.pan(Vec2::new(0.0, 40.0));
            }
            if do_pan_down {
                self.viewer_state.pan(Vec2::new(0.0, -40.0));
            }
            if do_pan_left {
                self.viewer_state.pan(Vec2::new(40.0, 0.0));
            }
            if do_pan_right {
                self.viewer_state.pan(Vec2::new(-40.0, 0.0));
            }
        } else {
            if nav_next {
                self.navigate(1);
            }
            if nav_prev {
                self.navigate(-1);
            }
        }

        if do_toggle_zoom {
            let viewport = ctx.viewport_rect().size();
            self.viewer_state.toggle_zoom(image_size, viewport);
        }
        if do_zoom_in {
            self.viewer_state.zoom_by(1.15, None, Vec2::splat(1.0));
        }
        if do_zoom_out {
            self.viewer_state
                .zoom_by(1.0 / 1.15, None, Vec2::splat(1.0));
        }
        if do_zoom_reset {
            self.viewer_state.reset();
        }
        if do_fullscreen {
            self.fullscreen = !self.fullscreen;
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
        }
        if do_toggle_thumbs {
            self.config.show_thumbnails = !self.config.show_thumbnails;
        }
        if do_rotate_cw {
            self.transform_current(90);
        }
        if do_rotate_ccw {
            self.transform_current(-90);
        }
        if do_flip_h {
            self.flip_current(true);
        }
        if do_flip_v {
            self.flip_current(false);
        }
        if do_open_folder {
            self.pick_folder();
        }
        if do_open_file {
            self.pick_file();
        }
        if do_combine {
            self.combine_dialog.open = true;
        }
        if do_slideshow {
            self.slideshow.toggle();
        }
        if do_settings {
            self.settings_dialog.open = true;
        }
        if do_quit {
            let _ = self.config.save();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn transform_current(&mut self, degrees: i32) {
        if self.entries.is_empty() {
            return;
        }
        let path = self.entries[self.current_index].path.clone();
        if let Ok(img) = LoadedImage::load(&path) {
            let rotated = apply_rotation(img.image, degrees);
            if save_image(&rotated, &path).is_ok() {
                self.thumb_textures.remove(&self.current_index);
                self.current_texture = None;
                self.load_current_image();
                if !self.entries.is_empty() {
                    let next = (self.current_index + 1) % self.entries.len();
                    self.start_preload(next);
                }
                self.set_status("Saved.");
            }
        }
    }

    fn flip_current(&mut self, horizontal: bool) {
        if self.entries.is_empty() {
            return;
        }
        let path = self.entries[self.current_index].path.clone();
        if let Ok(img) = LoadedImage::load(&path) {
            let flipped = if horizontal {
                apply_flip_horizontal(img.image)
            } else {
                apply_flip_vertical(img.image)
            };
            if save_image(&flipped, &path).is_ok() {
                self.thumb_textures.remove(&self.current_index);
                self.current_texture = None;
                self.load_current_image();
                self.set_status("Saved.");
            }
        }
    }

    fn pick_folder(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.open_path(path);
        }
    }

    fn pick_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Images",
                &[
                    "jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "tif", "avif", "heic",
                    "cr2", "cr3", "nef", "arw", "dng", "orf", "rw2", "raf",
                ],
            )
            .add_filter("Videos", &["mp4", "mkv", "avi", "mov", "wmv", "webm"])
            .add_filter("All media", &["*"])
            .pick_file()
        {
            self.open_path(path);
        }
    }
}

impl eframe::App for KadrApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Force window onto the preferred monitor using physical-pixel SetWindowPos.
        // This runs once on the first frame, after Windows has had its chance to
        // apply its own "remember window locations" repositioning.
        #[cfg(windows)]
        if let Some((mx, my, mw, mh)) = self.monitor_snap.take() {
            unsafe {
                use std::os::windows::ffi::OsStrExt;
                use winapi::shared::windef::RECT;
                use winapi::um::winuser::{
                    FindWindowW, GetWindowRect, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
                    SetWindowPos,
                };
                let title: Vec<u16> = std::ffi::OsStr::new("kadr")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
                if !hwnd.is_null() {
                    let mut r = RECT {
                        left: 0,
                        top: 0,
                        right: 0,
                        bottom: 0,
                    };
                    GetWindowRect(hwnd, &mut r);
                    let ww = r.right - r.left;
                    let wh = r.bottom - r.top;
                    let x = mx + ((mw as i32 - ww) / 2).max(0);
                    let y = my + ((mh as i32 - wh) / 2).max(0);
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        x,
                        y,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            }
        }

        // Poll EXIF thumbnail — show low-res preview instantly while full decode runs.
        // Only applied when no full image is present yet; index check discards stale results.
        let thumb_result = self.thumb_loading.lock().unwrap().take();
        if let Some(result) = thumb_result {
            if result.index == self.current_index && self.current_texture.is_none() {
                if let Some(color_img) = result.image {
                    let tex =
                        ctx.load_texture("current_image", color_img, egui::TextureOptions::LINEAR);
                    self.current_texture = Some(tex);
                }
            }
        }

        // Poll async image load — guard dropped before mutable calls
        let load_result = self.loading.lock().unwrap().take();
        if let Some(result) = load_result {
            if result.index == self.current_index {
                if let Some(color_img) = result.image {
                    let tex =
                        ctx.load_texture("current_image", color_img, egui::TextureOptions::LINEAR);
                    self.current_texture = Some(tex);
                    // Immediately start preloading the next image (always,
                    // not just during slideshows — makes manual navigation instant too).
                    if !self.entries.is_empty() {
                        let len = self.entries.len();
                        let next = (self.current_index + 1) % len;
                        self.start_preload(next);
                    }
                } else if let Some(err) = result.error {
                    self.set_status(format!("Error: {err}"));
                }
            }
        }

        // Promote completed preload into a ready texture.
        let preload_result = self.preload_loading.lock().unwrap().take();
        if let Some(result) = preload_result {
            if Some(result.index) == self.preload_index {
                if let Some(color_img) = result.image {
                    let tex =
                        ctx.load_texture("preload_image", color_img, egui::TextureOptions::LINEAR);
                    self.preload_texture = Some(tex);
                }
            }
        }

        // ── Collect async thumbnail results ──────────────────────────────────
        let thumb_done: Vec<(usize, ColorImage)> = {
            let mut lock = self.thumb_results.lock().unwrap();
            std::mem::take(&mut *lock)
        };
        for (idx, img) in thumb_done {
            self.thumb_pending.remove(&idx);
            if self.thumb_textures.len() >= THUMB_CACHE_LIMIT {
                if let Some(oldest) = self.thumb_order.first().copied() {
                    self.thumb_textures.remove(&oldest);
                    self.thumb_order.remove(0);
                }
            }
            let tex = ctx.load_texture(
                format!("thumb_{idx}"),
                img,
                egui::TextureOptions::LINEAR,
            );
            self.thumb_order.push(idx);
            self.thumb_textures.insert(idx, tex);
        }

        // ── Slideshow tick ───────────────────────────────────────────────────
        match self.slideshow.tick() {
            TickResult::Nothing => {}

            TickResult::BeginTransition => {
                // Capture outgoing zoom/pan before reset so the prev image
                // continues its Ken Burns motion during the crossfade.
                self.prev_zoom = self.viewer_state.zoom;
                self.prev_offset = self.viewer_state.offset;

                // Capture the outgoing image before advancing the index.
                self.prev_texture = self.current_texture.take();
                self.prev_image_size = self
                    .prev_texture
                    .as_ref()
                    .map(|t| Vec2::new(t.size()[0] as f32, t.size()[1] as f32))
                    .unwrap_or(Vec2::ZERO);
                self.transition_t = 0.0;

                // Advance to the next image.
                if !self.entries.is_empty() {
                    let len = self.entries.len() as i64;
                    let next = ((self.current_index as i64 + 1).rem_euclid(len)) as usize;
                    self.current_index = next;
                    self.viewer_state.reset();
                    self.video_ctx = None;
                    self.video_texture = None;

                    if self.preload_index == Some(next) && self.preload_texture.is_some() {
                        if let Some(tex) = self.preload_texture.take() {
                            // Preload finished — use it immediately, no wait.
                            self.current_texture = Some(tex);
                        } else {
                            // Preload still in flight — swap its slot to the main
                            // slot so the arriving result is picked up normally.
                            *self.loading.lock().unwrap() =
                                self.preload_loading.lock().unwrap().take();
                        }
                        self.preload_index = None;
                    } else {
                        self.load_current_image();
                    }

                    // on_advance: let Lua set initial zoom/pan for the incoming image.
                    let advance_cmd = self.lua_script.as_ref().and_then(|lua| {
                        lua.on_advance(&SlideContext {
                            current_index: self.current_index,
                            total: self.entries.len(),
                            interval_secs: self.slideshow.interval_secs(),
                            elapsed_secs: 0.0,
                        })
                        .ok()
                    });
                    if let Some(cmd) = advance_cmd {
                        let vp = ctx.viewport_rect().size();
                        if let Some(v) = cmd.zoom_target {
                            if let Some(tex) = &self.current_texture {
                                let sz = Vec2::new(tex.size()[0] as f32, tex.size()[1] as f32);
                                self.viewer_state.apply_lua_zoom(v, sz, vp);
                            }
                        }
                        apply_lua_cmd(&mut self.viewer_state, &cmd);
                    }
                }
                ctx.request_repaint();
            }

            TickResult::TransitionProgress(t) => {
                // Hold at 0 visually if the new image hasn't finished loading yet.
                self.transition_t = if self.current_texture.is_some() {
                    t
                } else {
                    0.0
                };
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }

            TickResult::TransitionDone => {
                self.prev_texture = None;
                self.transition_t = 1.0;
                ctx.request_repaint();
            }
        }

        // on_interval: ~60 Hz Lua callbacks for zoom / pan animation.
        if self.slideshow.active && !self.entries.is_empty() {
            let interval_cmd = self.lua_script.as_ref().and_then(|lua| {
                lua.on_interval(&SlideContext {
                    current_index: self.current_index,
                    total: self.entries.len(),
                    interval_secs: self.slideshow.interval_secs(),
                    elapsed_secs: self.slideshow.elapsed_secs(),
                })
                .ok()
            });
            if let Some(cmd) = interval_cmd {
                let vp = ctx.viewport_rect().size();
                if let Some(v) = cmd.zoom_target {
                    if let Some(tex) = &self.current_texture {
                        let sz = Vec2::new(tex.size()[0] as f32, tex.size()[1] as f32);
                        self.viewer_state.apply_lua_zoom(v, sz, vp);
                    }
                }
                apply_lua_cmd(&mut self.viewer_state, &cmd);
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        // Keyboard
        if !self.settings_dialog.open && !self.combine_dialog.open {
            self.handle_keyboard(&ctx);
        }

        // Drag-and-drop
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if let Some(path) = dropped.into_iter().next() {
            self.open_path(path);
        }

        let bg = self.bg_color32();

        if self.fullscreen {
            ui.painter()
                .rect_filled(ui.available_rect_before_wrap(), 0.0, bg);

            // In fullscreen, video uses the same rendering path as the normal panel —
            // return early here so it falls through to the CentralPanel below.
            let is_video = !self.entries.is_empty()
                && self.entries[self.current_index].media_type == MediaType::Video;
            if !is_video {
            let (tex, size) = if let Some(t) = self.current_texture.clone() {
                let s = Vec2::new(t.size()[0] as f32, t.size()[1] as f32);
                (t, s)
            } else if let Some(t) = self.prev_texture.clone() {
                (t, self.prev_image_size)
            } else {
                return;
            };
            let prev_clone = self.prev_texture.clone();
            let transition = prev_clone
                .as_ref()
                .filter(|_| self.current_texture.is_some())
                .map(|p| TransitionData {
                    prev_texture: p,
                    prev_size: self.prev_image_size,
                    prev_zoom: self.prev_zoom,
                    prev_offset: self.prev_offset,
                    t: self.transition_t,
                });
            show_viewer(ui, &tex, &mut self.viewer_state, size, bg, transition);
            return;
            } // end !is_video
        }

        egui::Panel::top("toolbar").show_inside(ui, |ui| {
            let sort_mode = self.config.viewer.sort_mode.clone();
            let toolbar_resp = show_toolbar(
                ui,
                &sort_mode,
                self.config.filter_images,
                self.config.filter_videos,
                self.config.scan_subfolders,
                self.slideshow.active,
                self.entries.len(),
                if self.entries.is_empty() {
                    None
                } else {
                    Some(self.current_index)
                },
            );

            if toolbar_resp.open_folder {
                self.pick_folder();
            }
            if toolbar_resp.open_file {
                self.pick_file();
            }
            if toolbar_resp.combine {
                self.combine_dialog.open = true;
            }
            if toolbar_resp.settings {
                self.settings_dialog.open = true;
            }
            if toolbar_resp.slideshow {
                self.slideshow.toggle();
            }

            if toolbar_resp.toggle_images {
                self.config.filter_images = !self.config.filter_images;
                if let Some(folder) = self.config.last_path.clone() {
                    self.open_path(folder);
                }
            }
            if toolbar_resp.toggle_videos {
                self.config.filter_videos = !self.config.filter_videos;
                if let Some(folder) = self.config.last_path.clone() {
                    self.open_path(folder);
                }
            }
            if toolbar_resp.toggle_subfolders {
                self.config.scan_subfolders = !self.config.scan_subfolders;
                if let Some(folder) = self.config.last_path.clone() {
                    self.open_path(folder);
                }
            }
            if let Some(mode) = toolbar_resp.sort_changed {
                self.config.viewer.sort_mode = mode;
                self.apply_sort();
            }
        });

        if self.config.show_thumbnails && !self.entries.is_empty() {
            let thumb_height = self.config.thumbnail_size + 10.0;
            let thumb_size = self.config.thumbnail_size;
            let current_index = self.current_index;

            egui::Panel::bottom("thumbnails")
                .exact_size(thumb_height)
                .resizable(false)
                .show_inside(ui, |ui| {
                    let strip = ThumbnailStrip {
                        thumb_size,
                        height: thumb_height,
                    };

                    let start = current_index.saturating_sub(10);
                    let end = (current_index + 10).min(self.entries.len());
                    for i in start..end {
                        self.load_thumb(i);
                    }

                    let clicked_idx = {
                        let thumb_entries: Vec<ThumbEntry<'_>> = self
                            .entries
                            .iter()
                            .enumerate()
                            .map(|(i, e)| ThumbEntry {
                                texture: self.thumb_textures.get(&i),
                                label: &e.file_name,
                                is_video: e.media_type == MediaType::Video,
                            })
                            .collect();
                        strip.show(ui, &thumb_entries, current_index).clicked_index
                    };

                    if let Some(idx) = clicked_idx {
                        self.go_to(idx);
                    }
                });
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(bg))
            .show_inside(ui, |ui| {
                if self.entries.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new(
                                "Open a folder or file to get started\n\nDrag & drop also works",
                            )
                            .color(Color32::from_gray(120))
                            .size(18.0),
                        );
                    });
                    return;
                }

                let is_video = self.entries[self.current_index].media_type == MediaType::Video;
                if is_video {
                    // Poll for new decoded frame
                    if let Some(frame) = self.video_ctx.as_mut().and_then(|vc| vc.poll_frame()) {
                        if let Some(tex) = self.video_texture.as_mut() {
                            tex.set(frame, egui::TextureOptions::LINEAR);
                        } else {
                            self.video_texture = Some(ctx.load_texture(
                                "video_frame",
                                frame,
                                egui::TextureOptions::LINEAR,
                            ));
                        }
                    }

                    let avail = ui.available_rect_before_wrap();

                    if let Some(tex) = &self.video_texture {
                        let tw = tex.size()[0] as f32;
                        let th = tex.size()[1] as f32;
                        if tw > 0.0 && th > 0.0 {
                            // Letterbox: scale to fit, preserve aspect ratio
                            let scale = (avail.width() / tw).min(avail.height() / th);
                            let dw = tw * scale;
                            let dh = th * scale;
                            let center = avail.center();
                            let img_rect = egui::Rect::from_center_size(center, vec2(dw, dh));
                            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                            ui.painter().image(tex.id(), img_rect, uv, Color32::WHITE);
                        }
                    } else {
                        // Waiting for first frame
                        ui.centered_and_justified(|ui| { ui.spinner(); });
                    }

                    // Controls overlay
                    let (pos, dur, paused, vol) = if let Some(vc) = &self.video_ctx {
                        (vc.get_position(), vc.get_duration(), vc.is_paused(), vc.get_volume())
                    } else {
                        (0.0, 0.0, true, self.video_volume)
                    };

                    match show_video_controls(ui, avail, paused, pos, dur, vol) {
                        ControlsAction::PlayPause => {
                            if let Some(vc) = &self.video_ctx { vc.play_pause(); }
                        }
                        ControlsAction::SeekTo(s) => {
                            if let Some(vc) = &self.video_ctx { vc.seek_absolute(s); }
                        }
                        ControlsAction::SetVolume(v) => {
                            self.video_volume = v;
                            if let Some(vc) = &self.video_ctx { vc.set_volume(v); }
                        }
                        ControlsAction::None => {}
                    }

                    return;
                }

                // Determine what to display: current (possibly fading in), prev (held while
                // current loads), or a spinner if nothing is available yet.
                let (tex, size) = if let Some(t) = self.current_texture.clone() {
                    let s = Vec2::new(t.size()[0] as f32, t.size()[1] as f32);
                    (Some(t), s)
                } else if let Some(t) = self.prev_texture.clone() {
                    // New image still loading — keep showing prev at full opacity.
                    (Some(t), self.prev_image_size)
                } else {
                    (None, Vec2::ZERO)
                };

                if let Some(tex) = tex {
                    // Only render crossfade when both images are present.
                    let prev_clone = self.prev_texture.clone();
                    let transition = prev_clone.as_ref()
                        .filter(|_| self.current_texture.is_some())
                        .map(|p| TransitionData {
                            prev_texture: p,
                            prev_size:    self.prev_image_size,
                            prev_zoom:    self.prev_zoom,
                            prev_offset:  self.prev_offset,
                            t:            self.transition_t,
                        });
                    show_viewer(ui, &tex, &mut self.viewer_state, size, bg, transition);
                } else {
                    ui.centered_and_justified(|ui| { ui.spinner(); });
                }

                // Filename overlay — pill background for readability
                let file_name = self.entries[self.current_index].file_name.clone();
                let screen = ctx.viewport_rect();
                let thumb_offset = if self.config.show_thumbnails {
                    self.config.thumbnail_size + 24.0
                } else {
                    24.0
                };
                {
                    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("overlay")));
                    let font_id = egui::FontId::proportional(13.0);
                    let text_color = Color32::from_rgba_premultiplied(230, 230, 235, 220);
                    // Approximate pill width (avg ~7px per char at 13pt)
                    let approx_w = (file_name.chars().count() as f32 * 7.2).max(60.0);
                    let text_h = 16.0_f32;
                    let x0 = 12.0_f32;
                    let y_bottom = screen.bottom() - thumb_offset - 8.0;
                    let pill_rect = egui::Rect::from_min_size(
                        egui::pos2(x0, y_bottom - text_h - 2.0),
                        egui::vec2(approx_w + 16.0, text_h + 6.0),
                    );
                    painter.rect_filled(
                        pill_rect,
                        egui::CornerRadius::same(8),
                        Color32::from_rgba_premultiplied(0, 0, 0, 155),
                    );
                    painter.text(
                        egui::pos2(x0 + 8.0, y_bottom),
                        egui::Align2::LEFT_BOTTOM,
                        &file_name,
                        font_id,
                        text_color,
                    );
                }

                // Status overlay — check elapsed before borrowing mutably
                let should_clear = match &self.status_msg {
                    Some((msg, since)) if since.elapsed().as_secs_f32() < 2.0 => {
                        ctx.layer_painter(egui::LayerId::new(
                            egui::Order::Foreground,
                            egui::Id::new("status_overlay"),
                        ))
                        .text(
                            egui::pos2(12.0, screen.bottom() - thumb_offset - 20.0),
                            egui::Align2::LEFT_BOTTOM,
                            msg.as_str(),
                            egui::FontId::proportional(13.0),
                            Color32::from_rgb(100, 220, 100),
                        );
                        ctx.request_repaint();
                        false
                    }
                    Some(_) => true,
                    None => false,
                };
                if should_clear {
                    self.status_msg = None;
                }
            });

        // Dialogs
        let combine_action = self.combine_dialog.show(&ctx);
        match combine_action {
            CombineAction::PickSource => {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.combine_dialog.source_path = Some(path);
                }
            }
            CombineAction::PickDest => {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.combine_dialog.dest_parent = Some(path);
                }
            }
            CombineAction::Run { source, dest } => match combine_folders(&source, &dest) {
                Ok(r) => {
                    self.combine_dialog.result_msg = Some(format!(
                        "Done: {} copied, {} renamed, {} errors",
                        r.copied,
                        r.renamed,
                        r.errors.len()
                    ));
                }
                Err(e) => {
                    self.combine_dialog.result_msg = Some(format!("Error: {e}"));
                }
            },
            CombineAction::Close => {
                self.combine_dialog.open = false;
                self.combine_dialog.result_msg = None;
            }
            CombineAction::None => {}
        }

        // Open Lua editor if settings requested it
        open_lua_editor_from_settings(&mut self.settings_dialog, &mut self.lua_editor);

        // Lua editor window
        match self.lua_editor.show(&ctx) {
            LuaEditorAction::Saved(code) => {
                self.settings_dialog.lua_code = code.clone();
                self.config.slideshow.lua_script = code.clone();
                match crate::slideshow::lua_script::LuaSlideshowScript::from_str(&code) {
                    Ok(script) => {
                        self.lua_script = Some(script);
                        self.settings_dialog.lua_error = None;
                    }
                    Err(e) if !code.trim().is_empty() => {
                        self.settings_dialog.lua_error = Some(e.to_string());
                        self.lua_script = None;
                        // Keep editor open so the user can fix the error
                        self.lua_editor.error =
                            Some(self.settings_dialog.lua_error.clone().unwrap_or_default());
                        self.lua_editor.open = true;
                    }
                    _ => {
                        self.lua_script = None;
                        self.settings_dialog.lua_error = None;
                    }
                }
            }
            LuaEditorAction::Closed | LuaEditorAction::None => {}
        }

        let settings_action = self.settings_dialog.show(&ctx, &mut self.config.keybinds);
        match settings_action {
            SettingsAction::Save | SettingsAction::Close => {
                // Detect scan-related changes before overwriting config
                let needs_rescan = self.settings_dialog.scan_subfolders
                    != self.config.scan_subfolders
                    || self.settings_dialog.filter_images != self.config.filter_images
                    || self.settings_dialog.filter_videos != self.config.filter_videos
                    || self.settings_dialog.sort_mode != self.config.viewer.sort_mode;

                self.config.show_thumbnails = self.settings_dialog.show_thumbnails;
                self.config.scan_subfolders = self.settings_dialog.scan_subfolders;
                self.config.filter_images = self.settings_dialog.filter_images;
                self.config.filter_videos = self.settings_dialog.filter_videos;
                self.config.viewer.sort_mode = self.settings_dialog.sort_mode.clone();
                self.config.remember_last_folder = self.settings_dialog.remember_last_folder;
                self.config.preferred_monitor = self.settings_dialog.preferred_monitor;
                self.config.viewer.background_color = self.settings_dialog.bg_color;
                self.config.thumbnail_size = self.settings_dialog.thumb_size;
                self.config.slideshow.interval_secs = self.settings_dialog.slideshow_interval;
                self.config.slideshow.transition_secs = self.settings_dialog.slideshow_transition;
                self.config.slideshow.loop_mode = self.settings_dialog.slideshow_loop;
                self.config.slideshow.random_order = self.settings_dialog.slideshow_random;
                self.config.slideshow.lua_script = self.settings_dialog.lua_code.clone();
                self.slideshow
                    .update_interval(self.settings_dialog.slideshow_interval);
                self.slideshow.transition_secs = self.settings_dialog.slideshow_transition;

                if needs_rescan {
                    if let Some(folder) = self.config.last_path.clone() {
                        self.open_path(folder);
                    }
                }

                // Recompile Lua script, show error in dialog if invalid
                match LuaSlideshowScript::from_str(&self.settings_dialog.lua_code) {
                    Ok(script) => {
                        self.lua_script = Some(script);
                        self.settings_dialog.lua_error = None;
                    }
                    Err(e) if !self.settings_dialog.lua_code.trim().is_empty() => {
                        self.settings_dialog.lua_error = Some(e.to_string());
                        self.lua_script = None;
                    }
                    _ => {
                        self.lua_script = None;
                        self.settings_dialog.lua_error = None;
                    }
                }

                if matches!(settings_action, SettingsAction::Save) {
                    if let Err(e) = self.config.save() {
                        self.set_status(format!("Save failed: {e}"));
                    } else {
                        self.set_status("Settings saved.");
                    }
                }
                self.settings_dialog.open = false;
            }
            SettingsAction::None => {}
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.config.save();
    }
}

// Small helper — not part of App impl
fn open_lua_editor_from_settings(
    settings: &mut crate::ui::settings_dialog::SettingsDialog,
    editor: &mut LuaEditor,
) {
    if settings.open_lua_editor {
        settings.open_lua_editor = false;
        editor.open_with(&settings.lua_code);
    }
}

fn apply_lua_cmd(
    state: &mut crate::ui::viewer::ViewerState,
    cmd: &crate::slideshow::lua_script::SlideCommand,
) {
    if let Some(v) = cmd.pan_x {
        state.lua_pan.x = v;
    }
    if let Some(v) = cmd.pan_y {
        state.lua_pan.y = v;
    }
    if let Some(v) = cmd.opacity {
        state.lua_opacity = v;
    }
}

fn apply_theme(ctx: &egui::Context) {
    // Slightly blue-tinted darks — more character than pure gray
    let bg = Color32::from_rgb(10, 10, 13);
    let surface = Color32::from_rgb(16, 16, 20);
    let surface2 = Color32::from_rgb(22, 22, 28);
    let surface3 = Color32::from_rgb(33, 33, 42);
    let surface4 = Color32::from_rgb(48, 48, 60);
    let accent = Color32::from_rgb(99, 155, 255);
    let text = Color32::from_rgb(222, 222, 228);
    let dim = Color32::from_rgb(105, 105, 120);
    let radius = egui::CornerRadius::same(7);
    let none = egui::Stroke::NONE;

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = surface;
    visuals.window_fill = surface2;
    visuals.faint_bg_color = bg;
    visuals.extreme_bg_color = bg;
    visuals.override_text_color = Some(text);
    visuals.window_corner_radius = egui::CornerRadius::same(10);
    visuals.popup_shadow = egui::Shadow::NONE;
    visuals.window_shadow = egui::Shadow::NONE;

    visuals.selection.bg_fill = Color32::from_rgba_premultiplied(99, 155, 255, 55);
    visuals.selection.stroke = egui::Stroke::new(1.0, accent);

    visuals.hyperlink_color = accent;

    visuals.widgets.noninteractive.bg_fill = surface;
    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, Color32::from_rgb(30, 30, 38));
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, dim);
    visuals.widgets.noninteractive.corner_radius = radius;
    visuals.widgets.noninteractive.expansion = 0.0;

    visuals.widgets.inactive.bg_fill = surface3;
    visuals.widgets.inactive.bg_stroke = none;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text);
    visuals.widgets.inactive.corner_radius = radius;
    visuals.widgets.inactive.expansion = 0.0;

    visuals.widgets.hovered.bg_fill = surface4;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(60, 60, 80));
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, text);
    visuals.widgets.hovered.corner_radius = radius;
    visuals.widgets.hovered.expansion = 1.0;

    visuals.widgets.active.bg_fill = Color32::from_rgb(50, 52, 72);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, accent);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, text);
    visuals.widgets.active.corner_radius = radius;
    visuals.widgets.active.expansion = 0.0;

    visuals.widgets.open.bg_fill = surface3;
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, accent);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, text);
    visuals.widgets.open.corner_radius = radius;
    visuals.widgets.open.expansion = 0.0;

    ctx.set_visuals(visuals);

    let mut style = (*ctx.global_style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(11.0, 5.0);
    style.spacing.indent = 14.0;
    style.spacing.interact_size = egui::vec2(36.0, 27.0);
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::new(13.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::new(13.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::new(11.0, egui::FontFamily::Proportional),
    );
    ctx.set_global_style(style);
}


fn make_thumbnail(img: &ColorImage, max_size: usize) -> ColorImage {
    let [w, h] = img.size;
    let scale = (max_size as f32 / w.max(h) as f32).min(1.0);
    let nw = ((w as f32 * scale) as usize).max(1);
    let nh = ((h as f32 * scale) as usize).max(1);

    let mut pixels = Vec::with_capacity(nw * nh);
    for y in 0..nh {
        for x in 0..nw {
            let sx = (x * w / nw).min(w - 1);
            let sy = (y * h / nh).min(h - 1);
            pixels.push(img.pixels[sy * w + sx]);
        }
    }
    ColorImage {
        size: [nw, nh],
        pixels,
        source_size: egui::Vec2::new(w as f32, h as f32),
    }
}
