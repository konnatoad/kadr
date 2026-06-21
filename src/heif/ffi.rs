#![allow(non_camel_case_types, dead_code)]

use std::ffi::{c_int, c_void};

pub type heif_context = c_void;
pub type heif_image_handle = c_void;
pub type heif_image = c_void;

pub const HEIF_COLORSPACE_RGB: c_int = 1;
pub const HEIF_CHROMA_INTERLEAVED_RGBA: c_int = 10;
pub const HEIF_CHANNEL_INTERLEAVED: c_int = 10;

#[repr(C)]
pub struct heif_error {
    pub code: c_int,
    pub subcode: c_int,
    pub message: *const std::ffi::c_char,
}

impl heif_error {
    pub fn ok(&self) -> bool {
        self.code == 0
    }
}

#[link(name = "heif")]
unsafe extern "C" {
    pub fn heif_context_alloc() -> *mut heif_context;
    pub fn heif_context_free(ctx: *mut heif_context);
    pub fn heif_context_read_from_memory_without_copy(
        ctx: *mut heif_context,
        mem: *const c_void,
        size: usize,
        options: *const c_void,
    ) -> heif_error;
    pub fn heif_context_get_primary_image_handle(
        ctx: *mut heif_context,
        out: *mut *mut heif_image_handle,
    ) -> heif_error;
    pub fn heif_image_handle_release(handle: *mut heif_image_handle);
    pub fn heif_image_handle_get_width(handle: *const heif_image_handle) -> c_int;
    pub fn heif_image_handle_get_height(handle: *const heif_image_handle) -> c_int;
    pub fn heif_decode_image(
        handle: *mut heif_image_handle,
        out_img: *mut *mut heif_image,
        colorspace: c_int,
        chroma: c_int,
        options: *const c_void,
    ) -> heif_error;
    pub fn heif_image_release(img: *mut heif_image);
    pub fn heif_image_get_plane_readonly(
        img: *const heif_image,
        channel: c_int,
        out_stride: *mut c_int,
    ) -> *const u8;
}
