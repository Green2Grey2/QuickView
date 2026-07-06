# Dependencies

This file is a quick reference for system and Rust dependencies.

## System dependencies (Arch)

Required:
- gtk4 (>= 4.10 — required for `append_scaled_texture` used by the zoom/pan renderer)
- libadwaita
- tesseract
- tesseract language pack(s) (at least English: `tesseract-data-eng`)
- gtk4-layer-shell
- libseccomp, lcms2, fontconfig — linked by the glycin client crate (sandbox
  seccomp filter, ICC color management, font paths). Needed at build and link
  time even when the runtime falls back to GDK decoding. On Debian/Ubuntu the
  build needs `libseccomp-dev`, `liblcms2-dev`, and `libfontconfig-dev`.

Optional:
- wl-clipboard
- glycin (sandboxed image decoding loaders; detected at runtime — without them
  QuickView falls back to in-process GDK decoding. On Debian/Ubuntu the package
  is `glycin-loaders`.)

## Rust crates (workspace)

- `gtk4` (GTK4 bindings, `v4_10` feature enabled)
- `libadwaita` (Adwaita widgets)
- `glycin` (sandboxed image decoding client; always compiled — links
  libseccomp, lcms2, and fontconfig)
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

