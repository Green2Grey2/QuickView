# QuickView

A Wayland-native image viewer for Linux with Quick Look-style preview and OCR text selection. Built with Rust, GTK4, and libadwaita.

## What This Project Does

QuickView opens images in two modes:
- **Quick Preview** (`--quick-preview`) — borderless floating overlay, dismiss with Space/Esc
- **Full Viewer** (default) — standard windowed app with headerbar, arrow key navigation between images

After displaying an image, Tesseract OCR runs asynchronously in the background. Once complete, users can drag-select recognized words directly on the image and copy them with Ctrl+C.

## Target Environment

- Arch Linux (primary), Wayland compositors (niri, Hyprland, Sway)
- GTK4 + libadwaita for UI
- gtk4-layer-shell for true overlay behavior on wlroots compositors
- Tesseract for OCR (called as CLI, output parsed as TSV)

## Project Structure

```
crates/
  quickview/          CLI entrypoint (clap)
  quickview-core/     OCR, geometry, selection, caching — no GTK dependency
  quickview-ui/       GTK4/libadwaita windows, widgets, layer-shell
docs/                 Architecture, spec, phased plan, decisions
adrs/                 10 Architecture Decision Records
packaging/            Arch PKGBUILD, Flatpak manifest
assets/               .desktop file, icons, AppStream metainfo
```

## Key Architecture Decisions

- **Rust + Cargo** workspace with three crates (ADR-0001)
- **GTK4 + libadwaita** for Wayland-native UI (ADR-0002)
- **gtk4-layer-shell** for overlay window, runtime detection with fallback (ADR-0003)
- **Tesseract CLI with TSV output** for OCR — simpler than C API bindings (ADR-0005, ADR-0006)
- **async-channel + glib::MainContext::spawn_local** for thread-to-UI communication (ADR-0010)
- **No X11 fallback** — Wayland only

## Current State

The scaffold is functional with image display, async OCR pipeline, drag-select overlay, quick preview mode, and directory navigation. See `docs/PHASED_PLAN.md` for what's done and what's next.

### What works:
- Async, sandboxed image loading (glycin loader process when installed,
  runtime-probed once per session; in-process GDK decoding on a worker thread
  otherwise — fallback is session-wide only, never per-file)
- Quick Preview window (borderless, layer-shell, Space/Esc dismiss,
  click-outside-to-close via transparent backdrop; focus-loss close on the
  no-layer-shell fallback path)
- Single-instance app (`HANDLES_COMMAND_LINE` + canonical synthetic argv):
  repeating a `--quick-preview` invocation toggles the preview closed, a
  different file replaces its content, full-viewer invocations open new
  windows in the primary instance
- Full Viewer window (headerbar, arrow key navigation)
- File info in the headerbar (filename, dimensions, file size)
- Async OCR (Tesseract TSV → word bounding boxes)
- OCR settings via `~/.config/quickview/config.toml` (`quickview-core`
  `config.rs`): lang (precedence `--lang` > `QUICKVIEW_LANG` > config >
  `eng`) and `tessdata_dir` (`--tessdata-dir` > config); both live in
  `OcrOptions` and join the cache key
- On-disk OCR cache (`~/.cache/quickview/ocr/`, keyed by path+lang+mtime+size;
  no eviction in v1 — see ADR-0009 implementation notes)
- Drag-select overlay with word highlighting
- Ctrl+C clipboard copy and right-click context menu (Copy / Copy All Text)
- Stale decode and OCR result cancellation via monotonic job IDs
- Zoom & pan (Ctrl+scroll, pinch, +/- keys, middle-drag pan) via custom `ZoomableCanvas` widget

### What's not implemented yet:
- Performance benchmarks

## Development

```bash
# Install deps (Arch)
sudo pacman -S --needed gtk4 libadwaita tesseract tesseract-data-eng gtk4-layer-shell

# Build and run
cargo build
cargo run -- path/to/image.png
cargo run -- --quick-preview path/to/image.png

# Lint and test
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## Important Conventions

- Never block the GTK main thread — all OCR and I/O runs on background threads
- Use async-channel to send results back to the UI thread
- quickview-core must have zero GTK dependencies (keeps it testable without a display server)
- Coordinate transforms go through `ViewTransform::from_center()` and related methods in `geometry.rs` — image coords vs widget coords. Fields are private; use `.scale()`, `.offset_x()`, `.offset_y()` getters. `contain()` returns `ContainResult`.
- OCR results use image-space coordinates; convert to widget-space only for rendering
- OCR hit-testing uses `OcrWordIndex` spatial index (`ocr/index.rs`) for efficient drag-select queries
