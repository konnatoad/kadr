use anyhow::{Result, anyhow};
use image::{DynamicImage, ImageBuffer, Rgb};
use std::io::Read;
use std::path::Path;

use super::formats::MediaType;

pub struct LoadedImage {
    pub image: DynamicImage,
    pub preview: egui::ColorImage,
    pub width: u32,
    pub height: u32,
}

impl LoadedImage {
    pub fn load(path: &Path) -> Result<Self> {
        let media_type = super::formats::media_type_for_path(path)
            .ok_or_else(|| anyhow!("unsupported format"))?;

        let image = match media_type {
            MediaType::Image => load_standard(path)?,
            MediaType::RawImage => load_raw(path)?,
            MediaType::Video => return Err(anyhow!("video files cannot be loaded as images")),
        };

        const MAX_TEX_DIM: u32 = 2560;

        let width = image.width();
        let height = image.height();

        let preview_rgba = if width > MAX_TEX_DIM || height > MAX_TEX_DIM {
            image.thumbnail(MAX_TEX_DIM, MAX_TEX_DIM).to_rgba8()
        } else {
            image.to_rgba8()
        };

        let preview = egui::ColorImage::from_rgba_unmultiplied(
            [
                preview_rgba.width() as usize,
                preview_rgba.height() as usize,
            ],
            &preview_rgba,
        );

        Ok(Self {
            image,
            preview,
            width,
            height,
        })
    }

    pub fn to_egui_image(&self) -> &egui::ColorImage {
        &self.preview
    }
}

fn load_standard(path: &Path) -> Result<DynamicImage> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());

    if matches!(ext.as_deref(), Some("jpg") | Some("jpeg") | Some("jfif")) {
        let data = std::fs::read(path)?;
        match load_jpeg_turbo(&data) {
            Ok(img) => return Ok(img),
            Err(e) => log::warn!("turbojpeg failed ({e}), falling back to image::open"),
        }
    }

    Ok(image::open(path)?)
}

/// Decode a JPEG using mozjpeg (libjpeg-turbo fork, bundled — no system install needed).
/// 3-5× faster than the pure-Rust jpeg-decoder used by the `image` crate.
fn load_jpeg_turbo(data: &[u8]) -> Result<DynamicImage> {
    let decomp = mozjpeg::Decompress::new_mem(data)
        .map_err(|e| anyhow::anyhow!("mozjpeg init: {e}"))?;

    let width  = decomp.width()  as u32;
    let height = decomp.height() as u32;

    let mut rgb = decomp.rgb()
        .map_err(|e| anyhow::anyhow!("mozjpeg rgb: {e}"))?;

    let pixels: Vec<u8> = rgb.read_scanlines_flat()
        .map_err(|e| anyhow::anyhow!("mozjpeg: read_scanlines: {e}"))?;

    let buf = image::RgbImage::from_raw(width, height, pixels)
        .ok_or_else(|| anyhow::anyhow!("mozjpeg: buffer size mismatch"))?;
    Ok(DynamicImage::ImageRgb8(buf))
}

fn load_raw(path: &Path) -> Result<DynamicImage> {
    let raw = rawloader::decode_file(path).map_err(|e| anyhow!("RAW decode failed: {e}"))?;

    let (width, height) = (raw.width, raw.height);

    match raw.data {
        rawloader::RawImageData::Integer(data) => {
            let max_val = raw.whitelevels[0] as f32;
            let black = raw.blacklevels[0] as f32;

            let pixels: Vec<u8> = data
                .chunks(3)
                .flat_map(|chunk| {
                    let r = ((chunk[0] as f32 - black) / (max_val - black)).clamp(0.0, 1.0);
                    let g = ((chunk.get(1).copied().unwrap_or(chunk[0]) as f32 - black)
                        / (max_val - black))
                        .clamp(0.0, 1.0);
                    let b = ((chunk.get(2).copied().unwrap_or(chunk[0]) as f32 - black)
                        / (max_val - black))
                        .clamp(0.0, 1.0);
                    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
                })
                .collect();

            let buf =
                ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(width as u32, height as u32, pixels)
                    .ok_or_else(|| anyhow!("RAW buffer size mismatch"))?;
            Ok(DynamicImage::ImageRgb8(buf))
        }
        rawloader::RawImageData::Float(data) => {
            let pixels: Vec<u8> = data
                .chunks(3)
                .flat_map(|chunk| {
                    let r = chunk[0].clamp(0.0, 1.0);
                    let g = chunk.get(1).copied().unwrap_or(chunk[0]).clamp(0.0, 1.0);
                    let b = chunk.get(2).copied().unwrap_or(chunk[0]).clamp(0.0, 1.0);
                    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
                })
                .collect();

            let buf =
                ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(width as u32, height as u32, pixels)
                    .ok_or_else(|| anyhow!("RAW float buffer size mismatch"))?;
            Ok(DynamicImage::ImageRgb8(buf))
        }
    }
}

pub fn apply_rotation(img: DynamicImage, degrees: i32) -> DynamicImage {
    match degrees.rem_euclid(360) {
        90 => img.rotate90(),
        180 => img.rotate180(),
        270 => img.rotate270(),
        _ => img,
    }
}

pub fn apply_flip_horizontal(img: DynamicImage) -> DynamicImage {
    img.fliph()
}

pub fn apply_flip_vertical(img: DynamicImage) -> DynamicImage {
    img.flipv()
}

pub fn save_image(img: &DynamicImage, path: &Path) -> Result<()> {
    img.save(path)?;
    Ok(())
}

// ── EXIF thumbnail fast-path ──────────────────────────────────────────────────
//
// Camera JPEGs embed a small (~160 px) thumbnail in the EXIF APP1 header.
// That header sits in the very first bytes of the file, so we only need to
// read ≤256 KB regardless of the full image size (65 MB, etc.).
//
// No extra crate needed — the EXIF/TIFF structure is simple enough to parse
// with a handful of byte-level helpers.

/// Read the first 256 KB of a JPEG file, extract the embedded EXIF thumbnail,
/// decode it, and return it as a `ColorImage` ready to upload as a texture.
/// Returns `None` if the file has no EXIF thumbnail or on any parse error —
/// the caller falls back to the normal full-resolution decode.
pub fn try_load_exif_thumbnail(path: &Path) -> Option<egui::ColorImage> {
    // 256 KB covers every real-world EXIF section (typical: 20–80 KB).
    let mut buf = Vec::with_capacity(256 * 1024);
    std::fs::File::open(path)
        .ok()?
        .take(256 * 1024)
        .read_to_end(&mut buf)
        .ok()?;

    let thumb_bytes = find_exif_thumbnail_bytes(&buf)?;

    let img = image::load_from_memory(thumb_bytes).ok()?;
    let rgba = img.to_rgba8();
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        &rgba,
    ))
}

/// Scan the JPEG byte stream for an APP1/Exif segment and return a slice of
/// the embedded thumbnail JPEG within that buffer.
fn find_exif_thumbnail_bytes(data: &[u8]) -> Option<&[u8]> {
    // Must start with JPEG SOI marker FF D8
    if data.get(0..2) != Some(&[0xFF, 0xD8]) {
        return None;
    }

    let mut pos = 2usize;
    loop {
        if *data.get(pos)? != 0xFF {
            return None;
        }
        let marker = *data.get(pos + 1)?;

        // SOS (compressed image data starts) or EOI — stop
        if marker == 0xDA || marker == 0xD9 {
            return None;
        }
        // Padding byte — step past it
        if marker == 0xFF {
            pos += 1;
            continue;
        }

        // Segment length includes the two length bytes but not the FF+marker
        let seg_len = ((*data.get(pos + 2)? as usize) << 8) | (*data.get(pos + 3)? as usize);
        if seg_len < 2 {
            return None;
        }

        // APP1 = 0xE1
        if marker == 0xE1 {
            // data[pos+4 .. pos+10] should be "Exif\0\0"
            if data.get(pos + 4..pos + 10) == Some(b"Exif\x00\x00") {
                // TIFF block starts at pos+10 in the file buffer
                let tiff_abs = pos + 10;
                let tiff_end = pos + 2 + seg_len;
                let tiff_data = data.get(tiff_abs..tiff_end)?;
                if let Some(thumb) = find_thumbnail_in_tiff(tiff_data, tiff_abs, data) {
                    return Some(thumb);
                }
            }
        }

        pos = pos.checked_add(2 + seg_len)?;
    }
}

/// Walk IFD0 → IFD1 inside a TIFF block and return a slice of the JPEG
/// thumbnail bytes.  `tiff_abs` is the absolute offset of the TIFF header
/// within `full` (needed to resolve IFD1 offsets back into the file buffer).
fn find_thumbnail_in_tiff<'a>(tiff: &[u8], tiff_abs: usize, full: &'a [u8]) -> Option<&'a [u8]> {
    if tiff.len() < 8 {
        return None;
    }

    let le = match tiff.get(0..2)? {
        b"II" => true,
        b"MM" => false,
        _ => return None,
    };

    // TIFF magic number must be 42
    if read_tiff_u16(tiff, 2, le)? != 42 {
        return None;
    }

    // IFD0 offset, then IFD1 pointer sits right after IFD0's entries
    let ifd0_off = read_tiff_u32(tiff, 4, le)? as usize;
    let ifd0_count = read_tiff_u16(tiff, ifd0_off, le)? as usize;
    let ifd1_off = read_tiff_u32(tiff, ifd0_off + 2 + ifd0_count * 12, le)? as usize;
    if ifd1_off == 0 {
        return None;
    }

    // Scan IFD1 for thumbnail location tags
    let ifd1_count = read_tiff_u16(tiff, ifd1_off, le)? as usize;
    let mut thumb_off: Option<u32> = None;
    let mut thumb_len: Option<u32> = None;

    for i in 0..ifd1_count {
        let e = ifd1_off + 2 + i * 12;
        match read_tiff_u16(tiff, e, le)? {
            0x0201 => thumb_off = read_tiff_u32(tiff, e + 8, le), // JPEGInterchangeFormat
            0x0202 => thumb_len = read_tiff_u32(tiff, e + 8, le), // JPEGInterchangeFormatLength
            _ => {}
        }
    }

    let off = thumb_off? as usize;
    let len = thumb_len? as usize;
    if len < 4 {
        return None;
    }

    // The offset is relative to the TIFF header start in the file buffer
    let start = tiff_abs + off;
    let end = start + len;
    full.get(start..end)
}

fn read_tiff_u16(data: &[u8], off: usize, le: bool) -> Option<u16> {
    let b = data.get(off..off + 2)?;
    Some(if le {
        u16::from_le_bytes([b[0], b[1]])
    } else {
        u16::from_be_bytes([b[0], b[1]])
    })
}

fn read_tiff_u32(data: &[u8], off: usize, le: bool) -> Option<u32> {
    let b = data.get(off..off + 4)?;
    Some(if le {
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    } else {
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
    })
}
