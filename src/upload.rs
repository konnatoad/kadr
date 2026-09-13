use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

const BASE_URL: &str = "https://bomzh.fm";

pub fn upload_file(path: &Path, api_key: &str, folder_id: &str) -> Result<String> {
    let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("upload")
        .to_string();
    let mime = guess_mime(path);

    let boundary = format!("----kadrBoundary{}", std::process::id());
    let mut body = Vec::with_capacity(data.len() + 256);
    body.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"image\"; filename=\"{filename}\"\r\nContent-Type: {mime}\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(&data);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let mut req = ureq::post(&format!("{BASE_URL}/upload"))
        .header("X-Api-Key", api_key)
        .header(
            "Content-Type",
            &format!("multipart/form-data; boundary={boundary}"),
        );
    if !folder_id.is_empty() {
        req = req.query("folder_id", folder_id);
    }

    match req.send(&body[..]) {
        Ok(mut resp) => {
            let text = resp.body_mut().read_to_string()?;
            Ok(text.trim().to_string())
        }
        Err(ureq::Error::StatusCode(code)) => bail!("HTTP {code}"),
        Err(e) => bail!("{e}"),
    }
}

pub fn expand_to_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in paths {
        if p.is_dir() {
            for entry in walkdir::WalkDir::new(p).follow_links(true) {
                if let Ok(entry) = entry
                    && entry.file_type().is_file()
                {
                    out.push(entry.path().to_path_buf());
                }
            }
        } else if p.is_file() {
            out.push(p.clone());
        }
    }
    out
}

fn guess_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" | "jfif" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",
        "ico" => "image/x-icon",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "mkv" => "video/x-matroska",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}
