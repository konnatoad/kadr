//! Minimal raw FFI bindings to libmpv.
//!
//! Build requirements:
//!   - mpv.lib (import library) must be on the linker search path at compile time.
//!     Set MPV_LIB_DIR env var or place mpv.lib next to the binary / in a well-known SDK dir.
//!   - mpv-2.dll (or mpv.dll) must be next to the executable at runtime.
//!
//! Download the mpv Windows build + libmpv dev package from:
//!   https://sourceforge.net/projects/mpv-player-windows/files/libmpv/

#![allow(non_camel_case_types, dead_code)]

use std::ffi::{c_char, c_int, c_void};

// Opaque mpv types
pub type MpvHandle = c_void;
pub type MpvRenderCtx = c_void;

/// `mpv_render_param` — (type, *data) pair; array terminated by `type_ = 0`.
#[repr(C)]
pub struct MpvRenderParam {
    pub type_: c_int,
    pub data: *mut c_void,
}

// ── mpv_format ────────────────────────────────────────────────────────────────
pub const MPV_FORMAT_STRING: c_int = 1;
pub const MPV_FORMAT_FLAG: c_int = 3;   // *int (0/1)
pub const MPV_FORMAT_INT64: c_int = 4;  // *int64_t
pub const MPV_FORMAT_DOUBLE: c_int = 5; // *double

// ── mpv_render_param_type (render.h) ─────────────────────────────────────────
pub const MPV_RENDER_PARAM_INVALID: c_int = 0;
pub const MPV_RENDER_PARAM_API_TYPE: c_int = 1;
pub const MPV_RENDER_PARAM_ADVANCED_CONTROL: c_int = 10;
pub const MPV_RENDER_PARAM_SW_SIZE: c_int = 17;    // *int[2]
pub const MPV_RENDER_PARAM_SW_FORMAT: c_int = 18;  // *const char ("rgba")
pub const MPV_RENDER_PARAM_SW_STRIDE: c_int = 19;  // *size_t
pub const MPV_RENDER_PARAM_SW_POINTER: c_int = 20; // *void (pixel buffer)

/// Flag returned by `mpv_render_context_update` when a new frame is available.
pub const MPV_RENDER_UPDATE_FRAME: u64 = 1;

/// API type string for software (CPU) rendering.
pub const MPV_RENDER_API_TYPE_SW: &[u8] = b"sw\0";

// ── mpv functions ─────────────────────────────────────────────────────────────
#[cfg_attr(windows, link(name = "mpv"))]
#[cfg_attr(not(windows), link(name = "mpv"))]
unsafe extern "C" {
    pub fn mpv_create() -> *mut MpvHandle;
    pub fn mpv_initialize(ctx: *mut MpvHandle) -> c_int;
    pub fn mpv_terminate_destroy(ctx: *mut MpvHandle);

    pub fn mpv_set_property_string(
        ctx: *mut MpvHandle,
        name: *const c_char,
        data: *const c_char,
    ) -> c_int;
    pub fn mpv_set_property(
        ctx: *mut MpvHandle,
        name: *const c_char,
        format: c_int,
        data: *mut c_void,
    ) -> c_int;
    pub fn mpv_get_property(
        ctx: *mut MpvHandle,
        name: *const c_char,
        format: c_int,
        data: *mut c_void,
    ) -> c_int;
    pub fn mpv_command(ctx: *mut MpvHandle, args: *const *const c_char) -> c_int;

    pub fn mpv_render_context_create(
        res: *mut *mut MpvRenderCtx,
        mpv: *mut MpvHandle,
        params: *mut MpvRenderParam,
    ) -> c_int;
    pub fn mpv_render_context_free(ctx: *mut MpvRenderCtx);
    pub fn mpv_render_context_set_update_callback(
        ctx: *mut MpvRenderCtx,
        callback: Option<unsafe extern "C" fn(*mut c_void)>,
        callback_ctx: *mut c_void,
    );
    pub fn mpv_render_context_render(
        ctx: *mut MpvRenderCtx,
        params: *mut MpvRenderParam,
    ) -> c_int;
    pub fn mpv_render_context_update(ctx: *mut MpvRenderCtx) -> u64;
}
