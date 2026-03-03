# QuickView Spec Sheet

**Document version:** 0.1 (planning)  
**Last updated:** 2026-03-02  
**Source:** user-provided high-level spec (`SPEC.md`)

---

## 1) Product summary

QuickView is a lightweight **Wayland-first image previewer** for Linux with built-in **OCR text selection**, inspired by macOS Quick Look + Preview.

Two launch modes:

1. **Quick Preview** (intended to be triggered via spacebar keybind / file manager integration)  
   - Borderless overlay window, centered, feels “instant”.

2. **Full Viewer** (intended to be the default image “Open With” handler)  
   - Windowed application with zoom/pan and directory navigation.

---

## 2) Target environment

- Distros: **Arch Linux** (and Arch-based)
- Display: **Wayland** (wlroots-compatible compositors: e.g., niri, Hyprland, Sway)
- UI: **GTK4 + libadwaita** (native GNOME/Adwaita look)
- OCR engine: **Tesseract** (system package)

**Explicit constraint:** No X11 fallback is required for v1.

---

## 3) Goals

### G1 — Instant-feeling preview
Show the image immediately in Quick Preview mode (no blocking on OCR).

### G2 — OCR text selection on images
Recognized text becomes selectable as an overlay layer once OCR completes.

### G3 — Practical “daily driver” viewer
Full Viewer mode supports zoom/pan + keyboard navigation + basic image metadata.

---

## 4) Non-goals (v1)

These are explicitly out of scope for the first deliverable unless re-prioritized:

- Image editing tools (crop/rotate/adjust)
- Video or non-image preview
- Thumbnail generation
- Plugin system
- PDF support

(They can be future phases.)

---

## 5) User stories

- As a user, I can press a key (spacebar) and instantly see a larger preview of the selected image, then dismiss it quickly.
- As a user, I can select text that appears in an image (screenshot, meme, scanned doc photo) and copy it.
- As a user, I can open an image normally and zoom/pan and move to next/previous image in the directory.
- As a user, I can see basic info: filename, dimensions, file size.

---

## 6) Functional requirements

### 6.1 Launch modes

**FR-001 — Quick Preview window**
- Launch parameter: `--quick-preview`.
- Borderless floating/overlay window.
- Centered on the active monitor.
- Image displayed **fit-to-window** by default.

**FR-002 — Quick Preview dismissal**
- Spacebar toggles (space closes if already open).
- Escape closes.
- Click outside closes (optional but strongly recommended).

**FR-003 — Full Viewer**
- Launch default: `quickview <file>` opens full viewer.
- Standard window chrome and title (or headerbar per libadwaita conventions).
- Zoom in/out (keyboard + gesture), pan (drag).
- Keyboard navigation: left/right arrows to previous/next image in same directory.
- Basic info panel: filename, dimensions, file size.

### 6.2 OCR & selection

**FR-004 — OCR pipeline is async**
- Image renders first.
- OCR starts after initial paint and runs asynchronously.
- While OCR is running, show a small indicator (“Recognizing text…”).

**FR-005 — OCR overlay + selection**
- Once OCR results arrive, create a selectable overlay layer aligned to the image.
- Drag-select highlights words/lines.
- Copy selection via `Ctrl+C` and a context menu “Copy”.

**FR-006 — Language control**
- Default OCR language is user-configurable (config file or env var).
- Command line overrides supported: `--lang eng` etc.

### 6.3 Integration

**FR-007 — Desktop integration**
- Provide `.desktop` file to register as an image handler.
- “Open With” works in Nautilus/Thunar/PCManFM/etc.

**FR-008 — CLI inputs**
- Accept a file path argument.
- Read a file path from stdin when the file argument is `-` or omitted (e.g., `echo photo.png | quickview -`).

**FR-009 — Clipboard**
- Copy uses native GTK clipboard (Wayland-safe).
- Optional fallback: invoke `wl-copy` only if needed and available.

---

## 7) Non-functional requirements

### NFR-001 — Responsiveness
- UI must remain responsive while OCR is running.
- No long-running work on the GTK main thread.

### NFR-002 — Security posture
- Treat images as untrusted input.
- Prefer sandboxed decoding if possible (see architecture).

### NFR-003 — Privacy
- OCR runs locally/offline. No cloud calls.

### NFR-004 — Reliability
- OCR failures must not break image viewing.
- Handle corrupted images gracefully (clear errors, no crash).

### NFR-005 — Accessibility
- Keyboard navigation supported for all major actions.
- High-contrast/high-DPI layouts should remain usable.

---

## 8) Acceptance criteria (v1)

- Quick Preview mode displays image without waiting for OCR.
- OCR overlay appears after processing and is aligned correctly under zoom/pan.
- Text selection copies the expected string to clipboard.
- Full Viewer navigation moves between images in directory.
- App is usable under at least one wlroots compositor (Sway/Hyprland/niri).

---

## 9) Resolved questions

- **”Spacebar from file manager” integration approach:**
  Compositor keybind is the primary path — `quickview --quick-preview <path>` invoked via a wlroots keybind. Optional GNOME Sushi-compatible DBus interface for Nautilus may follow later. (ADR-0008)

- **OCR scope:**
  Word-level bounding boxes only for v1. TSV output parsed per word with `order` field for reading sequence. Line/paragraph grouping deferred to a future phase. (ADR-0006)

- **Cache strategy:**
  In-memory cache for v1. Cache key includes file path, size, mtime, OCR language, and settings. Persistent on-disk cache designed for but not wired up yet. (ADR-0009)

