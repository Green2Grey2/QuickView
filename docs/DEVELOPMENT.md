# Development Guide

## Fast start

```bash
# build
cargo build

# run full viewer
cargo run --bin quickview -- path/to/image.png

# run quick preview
cargo run --bin quickview -- --quick-preview path/to/image.png
```

## Logging

QuickView uses `tracing`.

```bash
RUST_LOG=info cargo run --bin quickview -- --quick-preview path/to/image.png
RUST_LOG=quickview=debug,quickview_ui=debug cargo run --bin quickview -- path/to/image.png
```

## OCR setup

- Install Tesseract and at least one language pack.
- Default language is `eng`.

Examples:

```bash
sudo pacman -S --needed tesseract tesseract-data-eng
```

## UI architecture hints

- Keep all OCR and I/O off the GTK main thread.
- Use `async-channel` + `glib::MainContext::spawn_local()` to send results back to the UI.
- Prefer small widgets with clear responsibilities:
  - `ImageOverlayWidget` wraps the overlay + spinner; delegates to `ZoomableCanvas`
  - `ZoomableCanvas` (custom `gtk::Widget` subclass) handles image rendering via GSK/Snapshot, zoom/pan state, selection gestures, and OCR highlight overlay
  - `ViewerController` manages OCR dispatch and yields `OcrResult`

## Useful tasks

```bash
# formatting
cargo fmt

# linting
cargo clippy --all-targets --all-features -- -D warnings

# tests
cargo test --all
```

