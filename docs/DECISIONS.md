# QuickView Decisions & Alternatives

**Document version:** 0.2 (planning → scaffold)  
**Last updated:** 2026-03-02

This document records **best-practice technical decisions** for QuickView and lists viable alternatives.
For more detail, see the ADRs in `adrs/`.

---

## Decision summary (recommended defaults)

### UI + platform
- **UI toolkit:** GTK4 + libadwaita
- **Quick Preview overlay:** Layer Shell via **gtk4-layer-shell** when supported; graceful fallback otherwise
- **OS targets:** Wayland-only (no X11 fallback for v1)

### Image rendering/decoding
- **Decode library (preferred, optional):** **glycin** (sandboxed modular loaders)
- **Fallback decoder (default in this scaffold):** GTK/GDK `Texture::from_file` path

### OCR
- **Default OCR engine:** Tesseract (offline)
- **OCR output format:** TSV (word-level bounding boxes)
- **Model choice:** allow swapping language packs / model sets (fast vs best) later

### UX + data
- **Selection:** word boxes + drag selection; render highlight overlay
- **Clipboard:** GTK native clipboard API; optional `wl-copy` fallback
- **Caching:** start in-memory; add persistent cache when needed

---

## Detailed decisions and tradeoffs

### 1) UI toolkit: GTK4 + libadwaita (recommended)

**Why**
- Matches your spec’s “native look and feel” requirement for modern Wayland desktops.
- libadwaita provides GNOME/HIG-aligned widgets and styling.

**Tradeoffs**
- Some KDE users may prefer Qt styling.
- libadwaita intentionally limits theme variability.

**Alternatives**
- **Qt 6**: excellent on KDE; different integration story.
- **SDL/wgpu**: maximum control/perf; you build everything (accessibility, text selection, clipboard).

---

### 2) Quick Preview overlay: Layer Shell via gtk4-layer-shell

**Why**
- “Quick Look” needs a borderless overlay that can sit above normal app windows.
- The Layer Shell protocol provides explicit semantics for surfaces that are layered/anchored and have defined input behavior.

**Important best practice**
- **Prefer GTK4 + gtk4-layer-shell**. The older GTK3 wrapper (`gtk-layer-shell`) is explicitly marked *unmaintained* and recommends moving to gtk4-layer-shell.

**Tradeoffs**
- Not supported everywhere (notably GNOME Shell Wayland/Mutter). You must fall back to a normal undecorated window there.

**Alternatives**
- **Undecorated always-on-top toplevel**: broader compatibility but less “shell-like”.
- **Desktop-specific**:
  - GNOME: Sushi-style DBus previewer (Nautilus)

---

### 3) Image decoding: prefer glycin (sandboxed) where practical

**Why**
- Image files are untrusted input.
- glycin decodes via sandboxed modular loaders; reduces blast radius from decoder bugs.

**Tradeoffs**
- Requires glycin client libs + loader binaries to be installed.
- You still need a fallback path (and you may want to keep the code path simple during early prototyping).

**Alternatives**
- **GTK/GDK decoding only**: simplest; less hardening.
- **libvips**: very fast for huge images; heavier dependency.

---

### 4) OCR engine: Tesseract default

**Why**
- Available as a system package on Arch.
- Offline, scriptable, multiple structured outputs.

**Tradeoffs**
- Scene text in photos can be weaker than modern deep-learning OCR.
- Accuracy is sensitive to language and preprocessing.

**Alternatives**
- **PaddleOCR/EasyOCR**: strong accuracy; heavier runtime (often Python/ML).
- **Cloud OCR APIs**: adds privacy + network constraints.

Recommendation: keep Tesseract as default and keep the OCR layer behind an interface so alternative backends can be added.

---

### 5) OCR output format: TSV (default) vs hOCR (optional)

**Why TSV**
- Easy to parse.
- Provides word-level boxes.

**Why hOCR (optional)**
- Richer structure (lines/paragraph semantics).

Recommendation: implement TSV first; add hOCR later as debug/export.

---

### 6) Clipboard

**Recommended**
- Use GTK clipboard API first.

**Optional fallback**
- `wl-copy`/`wl-paste` tools can be used as a CLI fallback (useful in minimal environments).

---

### 7) Caching

**Recommended**
- In-memory cache keyed by `{path, mtime, size, lang, settings}`.

**Alternative**
- Persistent cache (SQLite) when repeated OCR is common.

---

### 8) Threading + cancellation

**Recommended**
- OCR runs in a worker thread (or separate process) and returns results over a channel.
- Cancellation is best-effort: cancel if possible; otherwise ignore late results.

