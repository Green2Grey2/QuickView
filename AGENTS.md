# Agent Onboarding (QuickView)

## What This Repo Is

QuickView is a Wayland-first image viewer for Linux with:
- Quick Look-style "Quick Preview" (borderless overlay window).
- OCR-backed text selection on top of images (Tesseract TSV).
- A full viewer mode with basic directory navigation.

Primary target is Arch Linux + Wayland (wlroots compositors like Sway/Hyprland/niri). Layer-shell is used when supported; GNOME/Mutter does not support layer-shell so Quick Preview falls back to a normal undecorated window.

## Repo Layout

- `crates/quickview/`: CLI entrypoint (`quickview` binary).
- `crates/quickview-core/`: non-GTK core (OCR parsing, geometry, selection logic, cache helpers).
- `crates/quickview-ui/`: GTK4/libadwaita UI (full viewer + quick preview windows, overlay widget).
- `docs/`: phased plan, architecture, decisions, development notes.
- `adrs/`: deeper architecture decisions.
- `assets/`: icons, `.desktop`, AppStream metainfo.
- `packaging/`: Arch/Flatpak/AppImage stubs.
- `templates/`: desktop entry and keybind examples.

## Quickstart (Arch)

1. Install system deps:
```bash
./scripts/bootstrap_arch.sh
```

Rust/toolchain notes:
- Minimum Rust is `1.83` (see workspace `Cargo.toml`).
- `rust-toolchain.toml` tracks `stable` and expects `rustfmt` and `clippy`.

2. Build:
```bash
cargo build
```

3. Run full viewer:
```bash
cargo run --bin quickview -- path/to/image.png
```

4. Run quick preview:
```bash
cargo run --bin quickview -- --quick-preview path/to/image.png
```

5. Provide a path via stdin (file argument omitted or `-`):
```bash
printf '%s\n' path/to/image.png | cargo run --bin quickview -- --quick-preview -
```

## Common Dev Commands

Using `just` (optional):
```bash
just fmt
just clippy
just test
just run-full path/to/image.png
just run-quick path/to/image.png
```

Without `just`:
```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

CLI help:
```bash
cargo run --bin quickview -- --help
```

## CI

GitHub Actions runs in an `archlinux:latest` container and installs system packages via `pacman` before running `fmt`, `clippy`, `test`, and `build` (see `.github/workflows/ci.yml`).

## Key Implementation Pointers

- Quick Preview window: `crates/quickview-ui/src/windows/quick_preview.rs`
- Full viewer window: `crates/quickview-ui/src/windows/full_viewer.rs`
- Viewer controller (loads images, kicks OCR, ignores late results): `crates/quickview-ui/src/windows/shared.rs`
- Overlay + drag selection rendering: `crates/quickview-ui/src/widgets/image_overlay.rs`
- Tesseract invocation + TSV parsing: `crates/quickview-core/src/ocr/`

## Project Invariants (Don't Break These)

- Keep the GTK main thread responsive: OCR and other heavy work must stay off-thread.
- "Render first, enrich later": show image before OCR finishes.
- When switching files or closing Quick Preview, never show stale OCR results for the previous file.

## Docs To Read First

- `docs/PHASED_PLAN.md`
- `docs/ARCHITECTURE.md`
- `docs/DEVELOPMENT.md`
- `docs/DECISIONS.md`

## What Not To Commit

- Build artifacts: `target/` (should already be ignored).
- Secrets or local env files: `.env`, `.env.*` (should already be ignored).
- Large binary test fixtures unless explicitly intended (keep sample images small).
