use std::ffi::{CString, c_int, c_void};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use super::mpv_ffi::*;

// Shared between the mpv internal thread (update callback) and the main thread.
struct CbData {
    flag: AtomicBool,
    ctx: egui::Context,
}

unsafe extern "C" fn update_cb(raw: *mut c_void) {
    let d = unsafe { &*(raw as *const CbData) };
    d.flag.store(true, Ordering::Relaxed);
    d.ctx.request_repaint();
}

pub struct VideoContext {
    handle: *mut MpvHandle,
    render_ctx: *mut MpvRenderCtx,
    cb_data: *mut CbData,
    /// Reused pixel buffer — resized when video dimensions change.
    render_buf: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Monotonic instant of the last successful render, used for FPS limiting.
    pub last_render: Option<Instant>,
}

// SAFETY: VideoContext is only ever used from the main thread.
unsafe impl Send for VideoContext {}
unsafe impl Sync for VideoContext {}

impl VideoContext {
    pub fn new(path: &std::path::Path, egui_ctx: egui::Context) -> anyhow::Result<Self> {
        let path_str = path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("video path is not valid UTF-8"))?;

        unsafe {
            // ── Create and initialise mpv ─────────────────────────────────────
            let handle = mpv_create();
            anyhow::ensure!(!handle.is_null(), "mpv_create returned null");

            // Must be set before mpv_initialize
            mpv_set_property_string(handle, b"vo\0".as_ptr() as _, b"libmpv\0".as_ptr() as _);
            mpv_set_property_string(handle, b"hwdec\0".as_ptr() as _, b"auto-safe\0".as_ptr() as _);
            mpv_set_property_string(handle, b"keep-open\0".as_ptr() as _, b"yes\0".as_ptr() as _);
            // Suppress terminal output
            mpv_set_property_string(handle, b"terminal\0".as_ptr() as _, b"no\0".as_ptr() as _);

            let rc = mpv_initialize(handle);
            if rc < 0 {
                mpv_terminate_destroy(handle);
                anyhow::bail!("mpv_initialize error {rc}");
            }

            // ── Create software render context ────────────────────────────────
            let api_ptr = MPV_RENDER_API_TYPE_SW.as_ptr() as *mut c_void;
            let mut adv: c_int = 1;
            let mut create_params = [
                MpvRenderParam { type_: MPV_RENDER_PARAM_API_TYPE, data: api_ptr },
                MpvRenderParam {
                    type_: MPV_RENDER_PARAM_ADVANCED_CONTROL,
                    data: &mut adv as *mut c_int as *mut c_void,
                },
                MpvRenderParam { type_: MPV_RENDER_PARAM_INVALID, data: null_mut() },
            ];

            let mut render_ctx: *mut MpvRenderCtx = null_mut();
            let rc = mpv_render_context_create(
                &mut render_ctx,
                handle,
                create_params.as_mut_ptr(),
            );
            if rc < 0 {
                mpv_terminate_destroy(handle);
                anyhow::bail!("mpv_render_context_create error {rc}");
            }

            // ── Wire update callback ──────────────────────────────────────────
            let cb_data = Box::into_raw(Box::new(CbData {
                flag: AtomicBool::new(false),
                ctx: egui_ctx,
            }));
            mpv_render_context_set_update_callback(
                render_ctx,
                Some(update_cb),
                cb_data as *mut c_void,
            );

            // ── Load file ─────────────────────────────────────────────────────
            let path_cstr = CString::new(path_str)?;
            let cmd: [*const i8; 3] = [
                b"loadfile\0".as_ptr() as _,
                path_cstr.as_ptr(),
                null_mut(),
            ];
            mpv_command(handle, cmd.as_ptr());

            Ok(Self {
                handle,
                render_ctx,
                cb_data,
                render_buf: Vec::new(),
                width: 0,
                height: 0,
                last_render: None,
            })
        }
    }

    // ── Frame polling ─────────────────────────────────────────────────────────

    /// Must be called from the same thread that created the VideoContext.
    /// Returns `Some(image)` if a new frame was rendered into it.
    pub fn poll_frame(&mut self) -> Option<egui::ColorImage> {
        let cb = unsafe { &*self.cb_data };
        if !cb.flag.swap(false, Ordering::Relaxed) {
            return None;
        }

        let w = self.get_i64("width") as u32;
        let h = self.get_i64("height") as u32;
        if w == 0 || h == 0 {
            return None;
        }

        let bytes = (w * h * 4) as usize;
        self.render_buf.resize(bytes, 0);

        let mut size = [w as c_int, h as c_int];
        let fmt = b"rgba\0";
        let stride: usize = w as usize * 4;

        let mut params = [
            MpvRenderParam {
                type_: MPV_RENDER_PARAM_SW_SIZE,
                data: size.as_mut_ptr() as *mut c_void,
            },
            MpvRenderParam {
                type_: MPV_RENDER_PARAM_SW_FORMAT,
                data: fmt.as_ptr() as *mut c_void,
            },
            MpvRenderParam {
                type_: MPV_RENDER_PARAM_SW_STRIDE,
                data: &stride as *const usize as *mut c_void,
            },
            MpvRenderParam {
                type_: MPV_RENDER_PARAM_SW_POINTER,
                data: self.render_buf.as_mut_ptr() as *mut c_void,
            },
            MpvRenderParam { type_: MPV_RENDER_PARAM_INVALID, data: null_mut() },
        ];

        let rc = unsafe { mpv_render_context_render(self.render_ctx, params.as_mut_ptr()) };
        if rc < 0 {
            return None;
        }

        self.width = w;
        self.height = h;
        self.last_render = Some(Instant::now());

        Some(egui::ColorImage::from_rgba_unmultiplied(
            [w as usize, h as usize],
            &self.render_buf,
        ))
    }

    // ── Playback controls ─────────────────────────────────────────────────────

    pub fn play_pause(&self) {
        unsafe {
            let cmd: [*const i8; 3] = [
                b"cycle\0".as_ptr() as _,
                b"pause\0".as_ptr() as _,
                null_mut(),
            ];
            mpv_command(self.handle, cmd.as_ptr());
        }
    }

    /// Seek to an absolute position in seconds.
    pub fn seek_absolute(&self, secs: f64) {
        let s = format!("{secs:.3}");
        let Ok(cs) = CString::new(s) else { return };
        unsafe {
            let cmd: [*const i8; 4] = [
                b"seek\0".as_ptr() as _,
                cs.as_ptr(),
                b"absolute\0".as_ptr() as _,
                null_mut(),
            ];
            mpv_command(self.handle, cmd.as_ptr());
        }
    }

    /// Seek relative to current position (can be negative).
    pub fn seek_relative(&self, secs: f64) {
        let s = format!("{secs:.3}");
        let Ok(cs) = CString::new(s) else { return };
        unsafe {
            let cmd: [*const i8; 4] = [
                b"seek\0".as_ptr() as _,
                cs.as_ptr(),
                b"relative\0".as_ptr() as _,
                null_mut(),
            ];
            mpv_command(self.handle, cmd.as_ptr());
        }
    }

    /// Volume 0.0–1.0 (1.0 = 100%, 2.0 = 200%).
    pub fn set_volume(&self, vol: f64) {
        let mut v = (vol * 100.0).clamp(0.0, 200.0);
        unsafe {
            mpv_set_property(
                self.handle,
                b"volume\0".as_ptr() as _,
                MPV_FORMAT_DOUBLE,
                &mut v as *mut f64 as *mut c_void,
            );
        }
    }

    pub fn get_position(&self) -> f64 {
        self.get_f64("time-pos")
    }

    pub fn get_duration(&self) -> f64 {
        self.get_f64("duration")
    }

    pub fn is_paused(&self) -> bool {
        self.get_i64("pause") != 0
    }

    pub fn get_volume(&self) -> f64 {
        self.get_f64("volume") / 100.0
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn get_f64(&self, name: &str) -> f64 {
        let Ok(cname) = CString::new(name) else { return 0.0 };
        let mut val: f64 = 0.0;
        unsafe {
            mpv_get_property(
                self.handle,
                cname.as_ptr(),
                MPV_FORMAT_DOUBLE,
                &mut val as *mut f64 as *mut c_void,
            );
        }
        val
    }

    fn get_i64(&self, name: &str) -> i64 {
        let Ok(cname) = CString::new(name) else { return 0 };
        let mut val: i64 = 0;
        unsafe {
            mpv_get_property(
                self.handle,
                cname.as_ptr(),
                MPV_FORMAT_INT64,
                &mut val as *mut i64 as *mut c_void,
            );
        }
        val
    }
}

impl Drop for VideoContext {
    fn drop(&mut self) {
        unsafe {
            // Destroy render context first (stops the update callback),
            // then tear down mpv, then free the callback data.
            mpv_render_context_free(self.render_ctx);
            mpv_terminate_destroy(self.handle);
            drop(Box::from_raw(self.cb_data));
        }
    }
}
