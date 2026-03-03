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
- Image loading and display (GdkTexture)
- Quick Preview window (borderless, layer-shell, Space/Esc dismiss)
- Full Viewer window (headerbar, arrow key navigation)
- Async OCR (Tesseract TSV → word bounding boxes)
- Drag-select overlay with word highlighting
- Ctrl+C clipboard copy
- Stale OCR result cancellation via monotonic job IDs

### What's not implemented yet:
- Zoom and pan
- Info panel (filename, dimensions, file size)
- Context menu (right-click copy)
- OCR caching (cache module exists but is not wired up)
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
- Coordinate transforms go through `compute_contain_transform()` — image coords vs widget coords
- OCR results use image-space coordinates; convert to widget-space only for rendering
