//! On-disk OCR result cache.
//!
//! Entries are JSON files under `<cache_root>/ocr/`, keyed by a blake3 hash of
//! the image path, OCR language, and file mtime+size — so an edited file is
//! simply a cache miss (no invalidation logic needed). There is no eviction in
//! v1: entries are a few KB each, and users can clear the directory manually.
//! Phase 8's persistent SQLite cache is the planned successor (ADR-0009).

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::ocr::models::OcrResult;

/// Return the XDG cache directory for QuickView.
///
/// On Linux this is typically: `~/.cache/quickview/`.
pub fn cache_dir() -> Option<PathBuf> {
    // qualifier, org, app
    let proj = ProjectDirs::from("com", "example", "QuickView")?;
    Some(proj.cache_dir().to_path_buf())
}

pub fn ocr_cache_path(cache_root: &Path, file: &Path, lang: &str) -> PathBuf {
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

    cache_root.join("ocr").join(format!("{key}.json"))
}

/// Load a cached OCR result for `file`, or `None` on a miss.
///
/// Any failure (entry absent, unreadable, corrupt, or written by an
/// incompatible older schema) is treated as a miss: OCR re-runs and the entry
/// is overwritten.
pub fn load_ocr(cache_root: &Path, file: &Path, lang: &str) -> Option<OcrResult> {
    let path = ocr_cache_path(cache_root, file, lang);
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Store an OCR result for `file`.
///
/// The write is atomic (temp file + rename in the same directory): concurrent
/// QuickView processes are a designed use case (Quick Preview spawns one per
/// invocation), so a torn write must never be readable.
pub fn store_ocr(
    cache_root: &Path,
    file: &Path,
    lang: &str,
    result: &OcrResult,
) -> anyhow::Result<()> {
    let path = ocr_cache_path(cache_root, file, lang);
    let dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("cache path has no parent: {}", path.display()))?;
    std::fs::create_dir_all(dir)?;

    let json = serde_json::to_vec(result)?;
    let tmp = path.with_extension(format!("json.tmp-{}", std::process::id()));
    std::fs::write(&tmp, &json)?;
    if let Err(err) = std::fs::rename(&tmp, &path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(err.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::ocr::models::OcrWord;

    fn sample_result() -> OcrResult {
        OcrResult {
            words: vec![OcrWord {
                text: "hello".into(),
                confidence: 96.5,
                bbox: Rect {
                    x: 10.0,
                    y: 20.0,
                    w: 30.0,
                    h: 40.0,
                },
                order: 0,
            }],
        }
    }

    #[test]
    fn key_changes_with_lang_path_and_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        let img = dir.path().join("a.png");
        std::fs::write(&img, b"xx").unwrap();
        let base = ocr_cache_path(root, &img, "eng");

        // Same inputs -> same key.
        assert_eq!(base, ocr_cache_path(root, &img, "eng"));

        // Different language -> different key.
        assert_ne!(base, ocr_cache_path(root, &img, "deu"));

        // Different path -> different key.
        let img2 = dir.path().join("b.png");
        std::fs::write(&img2, b"xx").unwrap();
        assert_ne!(base, ocr_cache_path(root, &img2, "eng"));

        // Different size -> different key.
        std::fs::write(&img, b"xxxx").unwrap();
        assert_ne!(base, ocr_cache_path(root, &img, "eng"));

        // Different mtime (same size) -> different key.
        std::fs::write(&img, b"xx").unwrap();
        let before = ocr_cache_path(root, &img, "eng");
        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        std::fs::File::open(&img)
            .unwrap()
            .set_modified(old)
            .unwrap();
        assert_ne!(before, ocr_cache_path(root, &img, "eng"));
    }

    #[test]
    fn store_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let img = dir.path().join("a.png");
        std::fs::write(&img, b"xx").unwrap();

        let result = sample_result();
        store_ocr(dir.path(), &img, "eng", &result).unwrap();

        let loaded = load_ocr(dir.path(), &img, "eng").expect("cache hit");
        assert_eq!(loaded.words.len(), 1);
        assert_eq!(loaded.words[0].text, "hello");
        assert_eq!(loaded.words[0].bbox, result.words[0].bbox);
        assert_eq!(loaded.words[0].order, 0);

        // No stray temp file left behind.
        let ocr_dir = dir.path().join("ocr");
        let leftovers: Vec<_> = std::fs::read_dir(&ocr_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_none_or(|ext| ext != "json"))
            .collect();
        assert!(leftovers.is_empty(), "temp files left: {leftovers:?}");
    }

    #[test]
    fn absent_entry_is_a_miss() {
        let dir = tempfile::tempdir().unwrap();
        let img = dir.path().join("a.png");
        std::fs::write(&img, b"xx").unwrap();

        assert!(load_ocr(dir.path(), &img, "eng").is_none());
    }

    #[test]
    fn corrupt_entry_is_a_miss() {
        let dir = tempfile::tempdir().unwrap();
        let img = dir.path().join("a.png");
        std::fs::write(&img, b"xx").unwrap();

        let path = ocr_cache_path(dir.path(), &img, "eng");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{not json").unwrap();

        assert!(load_ocr(dir.path(), &img, "eng").is_none());
    }
}
