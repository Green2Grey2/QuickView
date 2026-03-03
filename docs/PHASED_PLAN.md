# QuickView Phased Implementation Plan

**Document version:** 0.1 (planning)  
**Last updated:** 2026-03-02

This plan is structured as incremental phases that each deliver a usable artifact.
If priorities change, you can reshuffle phases, but try to keep the “render first, OCR later” principle intact.

---

## Phase 0 — Repo + tooling foundation

**Deliverables**
- Repository structure (`crates/`, `docs/`, `adrs/`)
- CI pipeline (build + lint + tests)
- Basic logging enabled
- Packaging skeletons (Arch PKGBUILD stub + Flatpak manifest stub)

**Definition of done**
- `build` succeeds on Arch in a clean environment
- `run` launches an empty window without warnings
- `docs/` renders in your preferred markdown viewer

---

## Phase 1 — Full Viewer: open + display image

**Core tasks**
- Implement CLI parsing:
  - `quickview <path>`
  - `quickview --help`
- Load and display image:
  - decode to a texture
  - show in a viewer widget
- Fit-to-window baseline
- Keyboard shortcuts:
  - `Esc` closes window
  - `+/-` zoom (or Ctrl+scroll)
- Basic UI shell with libadwaita (headerbar, etc.)

**Definition of done**
- Opening a PNG/JPG renders correctly
- No UI freezes during decode (decode is async or sufficiently fast)
- *(Zoom and pan deferred — see Phase 5 or later)*

---

## Phase 2 — Directory navigation + info panel

**Core tasks**
- Identify “image set” as all supported images in the same directory
- Maintain a sorted list and current index
- Add navigation:
  - Left/Right arrows to prev/next
- Add info panel:
  - filename
  - dimensions
  - file size

**Definition of done**
- Prev/next navigation is correct and stable
- Info updates immediately when switching images

---

## Phase 3 — Quick Preview mode (borderless overlay)

**Core tasks**
- Add a `--quick-preview` mode:
  - borderless
  - centered
  - dismiss on Space/Esc
- Implement “always-on-top” behavior
- If available, integrate Layer Shell (wlroots-friendly overlay):
  - runtime detect if Layer Shell is supported
  - use overlay layer with appropriate keyboard focus policy

**Definition of done**
- `quickview --quick-preview <image>` shows a borderless preview and closes instantly on Space/Esc
- Works on at least one wlroots compositor

---

## Phase 4 — OCR pipeline integration (async)

**Core tasks**
- Add OCR backend abstraction (interface/trait)
- Implement default Tesseract backend:
  - run OCR asynchronously
  - produce word-level boxes + text
- Add a non-blocking “OCR in progress” indicator
- Ensure cancellation / ignoring late results when user navigates away

**Definition of done**
- OCR starts after image display
- OCR completion adds internal OCR result state (even before selection UI exists)
- App stays responsive during OCR

---

## Phase 5 — OCR overlay + text selection UX

**Core tasks**
- Render OCR overlay (invisible by default or lightly highlighted on hover)
- Implement drag-selection:
  - compute selection rectangle in image coordinates
  - highlight matched words
- Implement copy:
  - Ctrl+C copies selected text
  - context menu action “Copy”

**Definition of done**
- User can reliably select and copy text from an image
- Selection stays aligned under zoom/pan

---

## Phase 6 — Integration polish

**Core tasks**
- `.desktop` integration:
  - default open handler for images
  - optional action for quick preview mode
- Compositor keybind recipes (niri, Hyprland, Sway) documented
- Optional GNOME Sushi-compatible DBus interface (advanced / optional)

**Definition of done**
- App can be set as default image viewer in common file managers
- Quick Preview can be triggered by a keybind in at least one compositor

---

## Phase 7 — Hardening + performance

**Core tasks**
- Add cache (in-memory first)
- Add basic benchmarking hooks (decode + OCR timing)
- Improve OCR accuracy options:
  - language selection
  - selectable `tessdata_fast` vs `tessdata_best`
- Add guardrails:
  - maximum image dimensions for OCR (downscale)
  - memory usage limits (where feasible)

**Definition of done**
- Re-opening the same image is faster due to caching
- OCR is stable across a variety of real-world screenshots

---

## Phase 8 — Nice-to-haves / future roadmap

- Persistent OCR cache (SQLite)
- Better layout/reading-order reconstruction
- Optional “search within OCR text”
- PDF support (if requested later)
- Image editing tools (crop/rotate)
- Multi-file drag/drop
- Plugin system

