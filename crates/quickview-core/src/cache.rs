use std::path::{Path, PathBuf};

use directories::ProjectDirs;

/// Return the XDG cache directory for QuickView.
///
/// On Linux this is typically: `~/.cache/quickview/`.
pub fn cache_dir() -> Option<PathBuf> {
    // qualifier, org, app
    let proj = ProjectDirs::from("com", "example", "QuickView")?;
    Some(proj.cache_dir().to_path_buf())
}

pub fn ocr_cache_path(file: &Path, lang: &str) -> Option<PathBuf> {
    let dir = cache_dir()?;

    // Include file metadata to avoid stale caches.
    let meta = std::fs::metadata(file).ok();
    let mtime = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);

    let mut hasher = blake3::Hasher::new();
    hasher.update(file.as_os_str().as_encoded_bytes());
    hasher.update(b"\0");
    hasher.update(lang.as_bytes());
    hasher.update(b"\0");
    hasher.update(&mtime.to_le_bytes());
    hasher.update(&size.to_le_bytes());
    let key = hasher.finalize().to_hex().to_string();

    Some(dir.join("ocr").join(format!("{key}.json")))
}
