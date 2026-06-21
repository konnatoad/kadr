mod ffi;

use anyhow::{Result, anyhow};
use image::{DynamicImage, RgbImage};

use ffi::*;

pub fn decode_raw(data: &[u8]) -> Result<DynamicImage> {
    unsafe {
        let lr = libraw_init(0);
        if lr.is_null() {
            return Err(anyhow!("libraw_init failed"));
        }
        scopeguard::defer! { libraw_close(lr); }

        let rc = libraw_open_buffer(lr, data.as_ptr() as *const _, data.len());
        if rc != 0 {
            return Err(anyhow!("libraw: open_buffer error {rc}"));
        }

        let rc = libraw_unpack(lr);
        if rc != 0 {
            return Err(anyhow!("libraw: unpack error {rc}"));
        }

        let rc = libraw_dcraw_process(lr);
        if rc != 0 {
            return Err(anyhow!("libraw: dcraw_process error {rc}"));
        }

        let mut errc: std::ffi::c_int = 0;
        let img = libraw_dcraw_make_mem_image(lr, &mut errc);
        if img.is_null() || errc != 0 {
            return Err(anyhow!("libraw: make_mem_image error {errc}"));
        }
        scopeguard::defer! { libraw_dcraw_clear_mem(img); }

        let width = (*img).width as u32;
        let height = (*img).height as u32;
        let colors = (*img).colors as usize;
        let bits = (*img).bits as usize;
        let data_size = (*img).data_size as usize;
        let pixels_ptr = (*img).data.as_ptr();
        let pixels_slice = std::slice::from_raw_parts(pixels_ptr, data_size);

        let rgb_pixels: Vec<u8> = if bits == 16 && colors == 3 {
            // 16-bit → 8-bit
            pixels_slice
                .chunks_exact(2)
                .map(|c| u16::from_ne_bytes([c[0], c[1]])
                    .wrapping_shr(8) as u8)
                .collect()
        } else if bits == 8 && colors == 3 {
            pixels_slice.to_vec()
        } else {
            return Err(anyhow!("libraw: unsupported output format ({colors} colors, {bits} bits)"));
        };

        let buf = RgbImage::from_raw(width, height, rgb_pixels)
            .ok_or_else(|| anyhow!("libraw: buffer size mismatch"))?;
        Ok(DynamicImage::ImageRgb8(buf))
    }
}
