use anyhow::{Result, anyhow};
use image::{DynamicImage, ImageBuffer, Rgb};
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
    let img = image::open(path)?;
    Ok(img)
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
