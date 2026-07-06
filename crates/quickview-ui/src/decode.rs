//! Async image decoding off the GTK main thread.
//!
//! Two backends, chosen once per session (ADR-0004):
//! - **glycin** — decodes in a sandboxed loader process. Used whenever the
//!   system has glycin loaders installed.
//! - **GDK** — in-process `gdk::Texture` decoding on a worker thread. Used
//!   only when the loader probe finds no glycin loaders for this session.
//!
//! The fallback is strictly session-wide, never per-file: a file glycin
//! rejects is a failed load. Re-feeding it to the unsandboxed GDK decoder
//! would let a crafted image reach the unsandboxed path simply by making the
//! sandboxed loader fail (NFR-002).

use std::{cell::Cell, path::Path};

use gtk4 as gtk;

use gtk::{gdk, gio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Glycin,
    Gdk,
}

/// The session-wide decode backend, probed on first use.
///
/// Main-thread only, like the rest of the UI crate. Concurrent first calls
/// may both run the probe; that is harmless (same result, cached once).
pub async fn backend() -> Backend {
    thread_local! {
        static BACKEND: Cell<Option<Backend>> = const { Cell::new(None) };
    }

    if let Some(backend) = BACKEND.get() {
        return backend;
    }

    let backend = if glycin::Loader::supported_mime_types().await.is_empty() {
        tracing::warn!(
            "image decoding: no glycin loaders installed; \
             falling back to in-process GDK decoding (unsandboxed)"
        );
        Backend::Gdk
    } else {
        tracing::info!("image decoding: glycin (sandboxed loader process)");
        Backend::Glycin
    };
    BACKEND.set(Some(backend));
    backend
}

/// Decode `path` into a texture without blocking the UI.
///
/// Await this from `glib::MainContext::spawn_local`; the actual decode work
/// happens in a sandboxed glycin process or on a worker thread.
pub async fn decode_texture(path: &Path) -> anyhow::Result<gdk::Texture> {
    match backend().await {
        Backend::Glycin => decode_glycin(path).await,
        Backend::Gdk => decode_gdk(path).await,
    }
}

async fn decode_glycin(path: &Path) -> anyhow::Result<gdk::Texture> {
    let file = gio::File::for_path(path);
    let image = glycin::Loader::new(file)
        .load()
        .await
        .map_err(|err| anyhow::anyhow!("glycin load failed: {err}"))?;
    let frame = image
        .next_frame()
        .await
        .map_err(|err| anyhow::anyhow!("glycin frame decode failed: {err}"))?;
    Ok(frame.texture())
}

async fn decode_gdk(path: &Path) -> anyhow::Result<gdk::Texture> {
    let (sender, receiver) = async_channel::bounded::<Result<gdk::Texture, glib::Error>>(1);

    let path = path.to_path_buf();
    // GdkTexture is immutable and thread-safe, so it can be created on a
    // worker thread and sent back to the main thread.
    std::thread::spawn(move || {
        let result = gdk::Texture::from_filename(&path);
        let _ = sender.send_blocking(result);
    });

    let result = receiver
        .recv()
        .await
        .map_err(|_| anyhow::anyhow!("GDK decode thread terminated unexpectedly"))?;
    Ok(result?)
}
