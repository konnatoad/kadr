use crate::media::{
    formats::{is_image, is_video},
    MediaEntry,
};
use std::path::Path;
use walkdir::WalkDir;

pub struct ScanOptions {
    pub include_images: bool,
    pub include_videos: bool,
    pub recursive: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            include_images: true,
            include_videos: true,
            recursive: true,
        }
    }
}

pub fn scan_folder(folder: &Path, opts: &ScanOptions) -> Vec<MediaEntry> {
    let walker = if opts.recursive {
        WalkDir::new(folder).follow_links(true)
    } else {
        WalkDir::new(folder).max_depth(1).follow_links(true)
    };

    walker
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let path = e.path();
            let img = opts.include_images && is_image(path);
            let vid = opts.include_videos && is_video(path);
            img || vid
        })
        .filter_map(|e| MediaEntry::from_path(e.into_path()))
        .collect()
}

pub fn scan_for_image(path: &Path, opts: &ScanOptions) -> Vec<MediaEntry> {
    if path.is_file() {
        let folder = path.parent().unwrap_or(path);
        scan_folder(folder, opts)
    } else {
        scan_folder(path, opts)
    }
}
