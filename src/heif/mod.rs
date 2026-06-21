mod ffi;

use anyhow::{Result, anyhow};
use image::{DynamicImage, RgbaImage};

use ffi::*;

pub fn decode_heif(data: &[u8]) -> Result<DynamicImage> {
    unsafe {
        let ctx = heif_context_alloc();
        if ctx.is_null() {
            return Err(anyhow!("heif_context_alloc failed"));
        }
        scopeguard::defer! { heif_context_free(ctx); }

        let err = heif_context_read_from_memory_without_copy(
            ctx,
            data.as_ptr() as *const _,
            data.len(),
            std::ptr::null(),
        );
        if !err.ok() {
            return Err(anyhow!("libheif: failed to read image data"));
        }

        let mut handle: *mut heif_image_handle = std::ptr::null_mut();
        let err = heif_context_get_primary_image_handle(ctx, &mut handle);
        if !err.ok() || handle.is_null() {
            return Err(anyhow!("libheif: failed to get primary image handle"));
        }
        scopeguard::defer! { heif_image_handle_release(handle); }

        let width = heif_image_handle_get_width(handle) as u32;
        let height = heif_image_handle_get_height(handle) as u32;

        let mut img: *mut heif_image = std::ptr::null_mut();
        let err = heif_decode_image(
            handle,
            &mut img,
            HEIF_COLORSPACE_RGB,
            HEIF_CHROMA_INTERLEAVED_RGBA,
            std::ptr::null(),
        );
        if !err.ok() || img.is_null() {
            return Err(anyhow!("libheif: decode failed"));
        }
        scopeguard::defer! { heif_image_release(img); }

        let mut stride: std::ffi::c_int = 0;
        let plane = heif_image_get_plane_readonly(img, HEIF_CHANNEL_INTERLEAVED, &mut stride);
        if plane.is_null() {
            return Err(anyhow!("libheif: failed to get image plane"));
        }

        let stride = stride as usize;
        let row_bytes = width as usize * 4;
        let mut pixels = Vec::with_capacity(row_bytes * height as usize);
        for y in 0..height as usize {
            let row = std::slice::from_raw_parts(plane.add(y * stride), row_bytes);
            pixels.extend_from_slice(row);
        }

        let buf = RgbaImage::from_raw(width, height, pixels)
            .ok_or_else(|| anyhow!("libheif: buffer size mismatch"))?;
        Ok(DynamicImage::ImageRgba8(buf))
    }
}
