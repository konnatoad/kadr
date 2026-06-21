#![allow(non_camel_case_types, dead_code)]

use std::ffi::{c_int, c_uint, c_ushort, c_void};

pub type libraw_data_t = c_void;

#[repr(C)]
pub struct libraw_processed_image_t {
    pub type_: c_int,
    pub height: c_ushort,
    pub width: c_ushort,
    pub colors: c_ushort,
    pub bits: c_ushort,
    pub data_size: c_uint,
    pub data: [u8; 1],
}

pub const LIBRAW_IMAGE_BITMAP: c_int = 1;

#[link(name = "raw_r")]
unsafe extern "C" {
    pub fn libraw_init(flags: c_uint) -> *mut libraw_data_t;
    pub fn libraw_close(lr: *mut libraw_data_t);
    pub fn libraw_open_buffer(
        lr: *mut libraw_data_t,
        buf: *const c_void,
        size: usize,
    ) -> c_int;
    pub fn libraw_unpack(lr: *mut libraw_data_t) -> c_int;
    pub fn libraw_dcraw_process(lr: *mut libraw_data_t) -> c_int;
    pub fn libraw_dcraw_make_mem_image(
        lr: *mut libraw_data_t,
        errc: *mut c_int,
    ) -> *mut libraw_processed_image_t;
    pub fn libraw_dcraw_clear_mem(img: *mut libraw_processed_image_t);
}
