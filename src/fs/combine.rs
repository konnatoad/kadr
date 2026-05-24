use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::media::formats::is_image;
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct CombineResult {
    pub copied: usize,
    pub renamed: usize,
    pub errors: Vec<(PathBuf, String)>,
}

pub fn combine_folders(source: &Path, dest: &Path) -> Result<CombineResult> {
    std::fs::create_dir_all(dest)?;

    let mut result = CombineResult::default();
    let mut name_counts: HashMap<String, u32> = HashMap::new();

    for entry in WalkDir::new(source).follow_links(true) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                result.errors.push((PathBuf::new(), e.to_string()));
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let src_path = entry.path();
        if !is_image(src_path) {
            continue;
        }

        let dest_path = resolve_dest_path(src_path, source, dest, &mut name_counts);
        let was_renamed = dest_path.file_name() != src_path.file_name();

        match std::fs::copy(src_path, &dest_path) {
            Ok(_) => {
                result.copied += 1;
                if was_renamed {
                    result.renamed += 1;
                }
            }
            Err(e) => {
                result.errors.push((src_path.to_path_buf(), e.to_string()));
            }
        }
    }

    Ok(result)
}

fn resolve_dest_path(
    src: &Path,
    source_root: &Path,
    dest: &Path,
    name_counts: &mut HashMap<String, u32>,
) -> PathBuf {
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("");

    let base_name = format!("{}.{}", stem, ext);
    let dest_plain = dest.join(&base_name);

    if !dest_plain.exists() && !name_counts.contains_key(&base_name) {
        name_counts.insert(base_name.clone(), 0);
        return dest_plain;
    }

    let parent_folder = src
        .parent()
        .and_then(|p| {
            p.strip_prefix(source_root).ok()
                .and_then(|rel| rel.components().next_back())
                .and_then(|c| c.as_os_str().to_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "file".to_string());

    let folder_name = format!("{}_{}.{}", stem, parent_folder, ext);
    let dest_with_folder = dest.join(&folder_name);

    if !dest_with_folder.exists() && !name_counts.contains_key(&folder_name) {
        name_counts.insert(folder_name.clone(), 0);
        return dest_with_folder;
    }

    let counter = name_counts.entry(folder_name.clone()).or_insert(0);
    loop {
        *counter += 1;
        let numbered = format!("{}_{}.{}", folder_name.trim_end_matches(&format!(".{}", ext)), counter, ext);
        let candidate = dest.join(&numbered);
        if !candidate.exists() {
            return candidate;
        }
    }
}
