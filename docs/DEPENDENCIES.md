# Dependencies

This file is a quick reference for system and Rust dependencies.

## System dependencies (Arch)

Required:
- gtk4 (>= 4.10 — required for `append_scaled_texture` used by the zoom/pan renderer)
- libadwaita
- tesseract
- tesseract language pack(s) (at least English: `tesseract-data-eng`)
- gtk4-layer-shell

Optional:
- wl-clipboard
- glycin + glycin-gtk4

## Rust crates (workspace)

- `gtk4` (GTK4 bindings, `v4_10` feature enabled)
- `libadwaita` (Adwaita widgets)
- `gtk4-layer-shell` (Layer Shell integration)
- `clap` (CLI)
- `tracing` + `tracing-subscriber` (logging)
- `async-channel` (thread-to-UI communication)
- `csv` (TSV parsing)
- `blake3` (cache key hashing)
- `serde` (OCR model serialization)
- `directories` (platform cache/config paths)

## Updating crates

```bash
cargo update
cargo tree -d
cargo audit
```

