//! Producing downscaled pixels for the OCR max-dimension guardrail.
//!
//! Tesseract reads a file path, so downscaling means materializing a smaller
//! temporary image. The pixels come from the already-decoded `gdk::Texture`
//! — never from re-decoding the untrusted file in-process, which would
//! bypass the glycin sandbox (NFR-002) and miss formats gdk-pixbuf has no
//! loader for. Split across threads:
//!
//! - [`download_rgba`] runs on the **main thread**: GL/dmabuf-backed texture
//!   downloads are not reliably thread-safe before GTK 4.12. It is a bounded
//!   memcpy, paid only for oversized images.
//! - [`write_downscaled_png`] runs on the OCR worker thread: scales the
//!   (trusted, self-produced) RGBA buffer with gdk-pixbuf and encodes it to
//!   a private temp PNG for tesseract.

use std::path::PathBuf;

use anyhow::{Context, Result};
use gtk4 as gtk;

use gtk::prelude::*;
use gtk::{gdk, gdk_pixbuf};

use quickview_core::ocr::downscale::DownscalePlan;

/// RGBA pixels downloaded from a texture, ready to cross to a worker thread
/// (`glib::Bytes` is `Send + Sync`; no GObject travels with it).
pub struct RgbaPixels {
    bytes: glib::Bytes,
    width: i32,
    height: i32,
    stride: i32,
}

/// Download `texture` as unpremultiplied RGBA. Main thread only.
pub fn download_rgba(texture: &gdk::Texture) -> Result<RgbaPixels> {
    let width = texture.width();
    let height = texture.height();
    let stride = usize::try_from(width)
        .ok()
        .and_then(|w| w.checked_mul(4))
        .context("image width overflows RGBA stride")?;
    let len = usize::try_from(height)
        .ok()
        .and_then(|h| h.checked_mul(stride))
        .context("image size overflows RGBA buffer")?;

    // Checked allocation first: `download_bytes()` would g_malloc the same
    // buffer and abort the whole process on OOM. A failed allocation must
    // instead surface as an Err so the caller degrades to full-resolution
    // OCR.
    let mut buf: Vec<u8> = Vec::new();
    buf.try_reserve_exact(len)
        .with_context(|| format!("cannot allocate {len} bytes for the RGBA copy"))?;
    buf.resize(len, 0);

    let mut downloader = gdk::TextureDownloader::new(texture);
    downloader.set_format(gdk::MemoryFormat::R8g8b8a8);
    // SAFETY: gdk4-rs 0.10 does not bind download_into (only the aborting
    // download_bytes). The buffer is exactly `stride * height` bytes with
    // `stride == width * 4`, which is what an R8g8b8a8 download writes, and
    // the downloader stays alive for the duration of the call.
    unsafe {
        let ptr: *const gdk::ffi::GdkTextureDownloader = glib::translate::ToGlibPtr::<
            *const gdk::ffi::GdkTextureDownloader,
        >::to_glib_none(&downloader)
        .0;
        gdk::ffi::gdk_texture_downloader_download_into(ptr, buf.as_mut_ptr(), stride);
    }

    Ok(RgbaPixels {
        bytes: glib::Bytes::from_owned(buf),
        width,
        height,
        stride: i32::try_from(stride).context("texture stride exceeds i32")?,
    })
}

/// The downscaled temp image tesseract will read.
///
/// The temp file is deleted when this guard drops (including on every error
/// path), so keep it alive across the tesseract run.
pub struct DownscaledImage {
    file: tempfile::NamedTempFile,
    /// Per-axis `original / actual` factors for mapping OCR bboxes back to
    /// original image space, computed from the scaled image's real
    /// dimensions (per-axis rounding makes one uniform factor drift).
    pub factor_x: f64,
    pub factor_y: f64,
    /// Actual dimensions fed to tesseract (equals the plan's target).
    pub target: (u32, u32),
}

impl DownscaledImage {
    pub fn path(&self) -> &std::path::Path {
        self.file.path()
    }
}

/// Scale `pixels` to `plan`'s target and write a private temp PNG.
/// Worker-thread safe: the pixbuf is created and dropped here.
pub fn write_downscaled_png(pixels: &RgbaPixels, plan: &DownscalePlan) -> Result<DownscaledImage> {
    let src = gdk_pixbuf::Pixbuf::from_bytes(
        &pixels.bytes,
        gdk_pixbuf::Colorspace::Rgb,
        true, // has_alpha (R8g8b8a8, unpremultiplied)
        8,
        pixels.width,
        pixels.height,
        pixels.stride,
    );
    let target_w = i32::try_from(plan.target_w).context("target width exceeds i32")?;
    let target_h = i32::try_from(plan.target_h).context("target height exceeds i32")?;
    let scaled = src
        .scale_simple(target_w, target_h, gdk_pixbuf::InterpType::Bilinear)
        .context("pixbuf scaling failed (out of memory?)")?;
    drop(src);

    // NamedTempFile is created 0600, matching the OCR cache's posture: the
    // downscaled copy holds the same sensitive content as the screenshot.
    let file = tempfile::Builder::new()
        .prefix("quickview-ocr-")
        .suffix(".png")
        .tempfile_in(temp_dir())
        .context("failed to create OCR temp file")?;
    scaled
        .savev(file.path(), "png", &[])
        .context("failed to encode downscaled PNG")?;

    Ok(DownscaledImage {
        file,
        factor_x: f64::from(pixels.width) / f64::from(scaled.width()),
        factor_y: f64::from(pixels.height) / f64::from(scaled.height()),
        target: (scaled.width() as u32, scaled.height() as u32),
    })
}

/// Where the temp image lives: `$XDG_RUNTIME_DIR` first (0700 tmpfs — same
/// privacy expectations as the OCR cache), then the cache dir, then the
/// system temp dir.
fn temp_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from) {
        if dir.is_dir() {
            return dir;
        }
    }
    if let Some(dir) = quickview_core::cache::cache_dir() {
        if std::fs::create_dir_all(&dir).is_ok() {
            return dir;
        }
    }
    std::env::temp_dir()
}
