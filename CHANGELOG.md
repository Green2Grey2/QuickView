# Changelog

## Unreleased

### Added
- **Zoom & pan** — Ctrl+scroll zoom (anchored at cursor), pinch-to-zoom,
  `+`/`-` keyboard zoom, `0`/`Home` reset to fit-to-window. Middle-click drag
  or Ctrl+left-drag to pan. Works in both Full Viewer and Quick Preview.
  Selection and OCR highlights stay aligned at all zoom levels.
- **Spatial index for OCR hit-testing** — `OcrWordIndex` uniform-grid index
  replaces linear scan during drag-select for faster word lookup.
- **ViewTransform hardening** — validated constructor rejects non-finite and
  non-positive scale values; fields are now private with getters.
- **CI GTK4 version check** — `pkg-config --atleast-version=4.10 gtk4` in CI.

- Initial scaffold.
