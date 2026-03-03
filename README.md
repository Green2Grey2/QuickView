<p align="center">
  <img src="assets/icons/hicolor/scalable/apps/com.example.QuickView.svg" width="128" height="128" alt="QuickView icon">
</p>

<h1 align="center">QuickView</h1>

<p align="center">
  A Wayland-native image viewer for Linux with <strong>Quick Look-style preview</strong> and <strong>OCR text selection</strong>.
</p>

<p align="center">
  <a href="docs/PHASED_PLAN.md">Roadmap</a> &middot;
  <a href="docs/ARCHITECTURE.md">Architecture</a> &middot;
  <a href="docs/SPEC_SHEET.md">Spec Sheet</a> &middot;
  <a href="docs/DECISIONS.md">Decisions</a> &middot;
  <a href="adrs/">ADRs</a> &middot;
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

---

Inspired by macOS Quick Look + Preview — open an image, select text right off it, copy to your clipboard. Built with GTK4 and libadwaita, targeting wlroots compositors (niri, Hyprland, Sway).

> **Status:** Early development. Core scaffold is functional — image display, OCR pipeline, drag-select overlay, and quick preview mode all work. See the [Phased Plan](docs/PHASED_PLAN.md) for what's done and what's next.

## Features

**Quick Preview** — borderless overlay window, dismiss with `Space` or `Esc`
```
quickview --quick-preview photo.png
```

**Full Viewer** — standard app window with headerbar, arrow key navigation between images in the same directory
```
quickview photo.png
```

**OCR Text Selection** — Tesseract runs asynchronously after the image loads. Drag to select recognized words, `Ctrl+C` to copy.

## Requirements

Arch Linux (primary target):

```bash
sudo pacman -S --needed \
  base-devel rustup pkgconf \
  gtk4 libadwaita \
  tesseract tesseract-data-eng \
  gtk4-layer-shell
```

Optional:
- `wl-clipboard` — CLI clipboard fallback
- `glycin` — sandboxed image decoding (future)

> `gtk4-layer-shell` provides true overlay behavior on wlroots compositors. On GNOME/Mutter, Quick Preview falls back to an undecorated window.

## Build and Run

```bash
cargo build

# Full viewer
cargo run -- path/to/image.png

# Quick preview
cargo run -- --quick-preview path/to/image.png

# Pipe a path via stdin
echo path/to/image.png | cargo run -- --quick-preview -

# OCR in a different language
cargo run -- --lang deu path/to/image.png
```

## Project Structure

```
crates/
  quickview/          CLI entrypoint
  quickview-core/     OCR parsing, geometry, selection logic (no GTK dep)
  quickview-ui/       GTK4/libadwaita windows, widgets, layer-shell
docs/                 Architecture, spec sheet, phased plan, decisions
adrs/                 Architecture Decision Records (10)
packaging/            Arch PKGBUILD, Flatpak manifest, AppImage stub
assets/               .desktop file, icons, AppStream metainfo
```

## License

[MIT](LICENSE)
