# QuickView Phased Implementation Plan

**Document version:** 0.2 (planning)  
**Last updated:** 2026-07-05

This plan is structured as incremental phases that each deliver a usable artifact.
If priorities change, you can reshuffle phases, but try to keep the “render first, OCR later” principle intact.

---

## Phase 0 — Repo + tooling foundation ✅

**Deliverables**
- Repository structure (`crates/`, `docs/`, `adrs/`)
- CI pipeline (build + lint + tests)
- Basic logging enabled
- Packaging skeletons (Arch PKGBUILD stub + Flatpak manifest stub)

**Definition of done**
- `build` succeeds on Arch in a clean environment ✅
- `run` launches an empty window without warnings ✅
- `docs/` renders in your preferred markdown viewer ✅

---

## Phase 1 — Full Viewer: open + display image ✅

**Core tasks**
- Implement CLI parsing:
  - `quickview <path>` ✅
  - `quickview --help` ✅
- Load and display image:
  - decode to a texture ✅
  - show in a viewer widget ✅
- Fit-to-window baseline ✅
- Keyboard shortcuts:
  - `Esc` closes window ✅
  - `+/-` zoom (or Ctrl+scroll) ✅
- Basic UI shell with libadwaita (headerbar, etc.) ✅

**Definition of done**
- Opening a PNG/JPG renders correctly ✅
- No UI freezes during decode (decode is async or sufficiently fast) ✅
- Zoom and pan: Ctrl+scroll, pinch-to-zoom, +/- keys, middle-drag pan ✅

---

## Phase 2 — Directory navigation + info panel ✅

**Core tasks**
- Identify “image set” as all supported images in the same directory ✅
- Maintain a sorted list and current index ✅
- Add navigation:
  - Left/Right arrows to prev/next ✅
- Add info panel:
  - filename ✅
  - dimensions ✅
  - file size ✅
  - (implemented as headerbar title/subtitle via `adw::WindowTitle` rather than
    a separate panel — most idiomatic libadwaita for three fields)

**Definition of done**
- Prev/next navigation is correct and stable ✅
- Info updates immediately when switching images ✅

---

## Phase 3 — Quick Preview mode (borderless overlay) ✅

**Core tasks**
- Add a `--quick-preview` mode:
  - borderless ✅
  - centered ✅
  - dismiss on Space/Esc ✅
- Implement “always-on-top” behavior ✅
- If available, integrate Layer Shell (wlroots-friendly overlay):
  - runtime detect if Layer Shell is supported ✅
  - use overlay layer with appropriate keyboard focus policy ✅

**Definition of done**
- `quickview --quick-preview <image>` shows a borderless preview and closes instantly on Space/Esc ✅
- Works on at least one wlroots compositor ✅

---

## Phase 4 — OCR pipeline integration (async) ✅

**Core tasks**
- Add OCR backend abstraction (interface/trait) ✅
- Implement default Tesseract backend:
  - run OCR asynchronously ✅
  - produce word-level boxes + text ✅
- Add a non-blocking “OCR in progress” indicator ✅
- Ensure cancellation / ignoring late results when user navigates away ✅

**Definition of done**
- OCR starts after image display ✅
- OCR completion adds internal OCR result state (even before selection UI exists) ✅
- App stays responsive during OCR ✅

---

## Phase 5 — OCR overlay + text selection UX ✅

**Core tasks**
- Render OCR overlay (invisible by default or lightly highlighted on hover) ✅
- Implement drag-selection:
  - compute selection rectangle in image coordinates ✅
  - highlight matched words ✅
- Implement copy:
  - Ctrl+C copies selected text ✅
  - context menu action “Copy” ✅ (plus “Copy All Text” for the whole OCR result)

**Definition of done**
- User can reliably select and copy text from an image ✅
- Selection stays aligned under zoom/pan ✅

---

## Phase 6 — Integration polish (partially done)

**Core tasks**
- `.desktop` integration:
  - default open handler for images ✅ (`assets/desktop/`, installed by PKGBUILD/Flatpak)
  - optional action for quick preview mode ✅ (`QuickPreview` desktop action)
- Compositor keybind recipes (niri, Hyprland, Sway) documented
  (`templates/keybind-examples.md` exists but is generic — add per-compositor snippets)
- Rename placeholder app ID to `io.github.Green2Grey2.QuickView` ✅
  (app ID in `quickview-ui`, `.desktop`, metainfo, icon filename, Flatpak
  manifest, PKGBUILD, and the `ProjectDirs` qualifier in `cache.rs`; Linux
  cache/config paths derive from the lowercased app name, so
  `~/.cache/quickview/` did not move and existing OCR cache entries stay valid)
- Quick Preview dismissal completeness (FR-002):
  - click outside closes ✅ — layer-shell path: surface anchored to all edges,
    transparent backdrop (`gtk::Overlay` sibling of the centered panel) closes
    on click; fallback path closes on focus loss only (a `GestureClick` on the
    window, as originally sketched, would also fire for clicks *inside* the
    preview and break drag-select, FR-005)
  - single-instance toggle ✅ — `GApplication` uniqueness with
    `HANDLES_COMMAND_LINE`; the invoking process resolves everything (clap,
    stdin, canonicalization) and forwards a canonical synthetic argv, so a
    second `--quick-preview` invocation toggles the open preview (same file)
    or replaces its content (different file). This retired the
    `run_with_args(&[])` workaround: GLib now receives a sanitized argv we
    build ourselves.
- Optional GNOME Sushi-compatible DBus interface (advanced / optional)

**Definition of done**
- App can be set as default image viewer in common file managers
- Quick Preview can be triggered by a keybind in at least one compositor
- Pressing the keybind while a preview is already open closes it (toggle)

---

## Phase 7 — Hardening + performance (in progress)

**Core tasks**
- Async + sandboxed image loading ✅ — implements ADR-0004, fixes NFR-001/NFR-002.
  - synchronous `Texture::from_file` in `load_file` replaced with glycin's
    async loader (always compiled, chosen by a runtime loader probe — no cargo
    feature), falling back to GDK decoding on a background thread via the
    existing async-channel pattern ✅
  - fallback is session-wide only, never per-file: a file glycin rejects is a
    failed load and is never re-fed to the unsandboxed GDK decoder ✅
  - previous image stays visible (under the busy spinner) until decode completes ✅
  - stale-result guard (monotonic job ID, same pattern as OCR) so fast
    arrow-key navigation cannot race decodes ✅
- Add cache ✅ — on-disk OCR cache (revising ADR-0009's "in-memory first"; see
  its implementation notes): blake3 key of path+lang+mtime+size → JSON under
  `~/.cache/quickview/ocr/`, checked/written on the OCR worker thread, atomic
  writes, no eviction in v1. Survives restarts, so Quick Preview's
  process-per-invocation benefits too.
- Add basic benchmarking hooks (decode + OCR timing)
- Improve OCR accuracy options:
  - language selection via config file / env var ✅ — precedence
    `--lang` > `QUICKVIEW_LANG` > `~/.config/quickview/config.toml` > `eng`
    (`quickview-core/src/config.rs`; see `templates/config.example.toml`)
  - selectable `tessdata_fast` vs `tessdata_best` ✅ — as a plain
    `tessdata_dir` path (config or `--tessdata-dir`) rather than an enum:
    the fast/best sets have no conventional install location, users clone
    them anywhere. Joins the OCR cache key per ADR-0009.
- Add guardrails:
  - maximum image dimensions for OCR (downscale)
  - memory usage limits (where feasible)

**Definition of done**
- Opening a large image does not freeze or hitch the UI
- Re-opening the same image is faster due to caching
- OCR is stable across a variety of real-world screenshots

---

## Phase 8 — Nice-to-haves / future roadmap (not started)

- Persistent OCR cache (SQLite)
- Better layout/reading-order reconstruction
- Optional “search within OCR text”
- PDF support (if requested later)
- Image editing tools (crop/rotate)
- Multi-file drag/drop
- Plugin system

