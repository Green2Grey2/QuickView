use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Very small, conservative list.
///
/// This is intentionally file-extension based for the scaffold.
/// In later phases we can sniff magic bytes or use an image library.
pub const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "gif", "tif", "tiff", "bmp", "svg",
];

pub fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn list_images_in_dir(dir: &Path) -> Result<Vec<PathBuf>, FsError> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && is_image_path(&path) {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}
