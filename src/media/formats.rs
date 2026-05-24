use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaType {
    Image,
    RawImage,
    Video,
}

#[derive(Debug, Clone)]
pub struct MediaEntry {
    pub path: std::path::PathBuf,
    pub media_type: MediaType,
    pub file_name: String,
    pub file_size: u64,
    pub modified: Option<std::time::SystemTime>,
}

impl MediaEntry {
    pub fn from_path(path: std::path::PathBuf) -> Option<Self> {
        let media_type = media_type_for_path(&path)?;
        let file_name = path.file_name()?.to_string_lossy().into_owned();
        let meta = std::fs::metadata(&path).ok();
        let file_size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified = meta.and_then(|m| m.modified().ok());
        Some(Self { path, media_type, file_name, file_size, modified })
    }
}

pub fn media_type_for_path(path: &Path) -> Option<MediaType> {
    let ext = path.extension()?.to_ascii_lowercase();
    let ext = ext.to_str()?;
    match ext {
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "tif"
        | "avif" | "heic" | "heif" | "ico" | "pnm" | "pbm" | "pgm" | "ppm" => {
            Some(MediaType::Image)
        }
        "cr2" | "cr3" | "nef" | "nrw" | "arw" | "srf" | "sr2" | "dng"
        | "orf" | "rw2" | "raf" | "pef" | "ptx" | "srw" | "x3f" | "mrw"
        | "3fr" | "fff" | "iiq" | "cap" | "eip" | "rwl" | "rwz" | "kdc"
        | "dcr" | "raw" | "r3d" | "mef" | "mos" => {
            Some(MediaType::RawImage)
        }
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "webm" | "flv" | "m4v"
        | "mpg" | "mpeg" | "3gp" | "ts" | "mts" | "m2ts" => {
            Some(MediaType::Video)
        }
        _ => None,
    }
}

pub fn is_image(path: &Path) -> bool {
    matches!(media_type_for_path(path), Some(MediaType::Image | MediaType::RawImage))
}

pub fn is_video(path: &Path) -> bool {
    matches!(media_type_for_path(path), Some(MediaType::Video))
}
