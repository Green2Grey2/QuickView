# QuickView Architecture

**Document version:** 0.1 (planning)  
**Last updated:** 2026-03-02

This architecture aims to satisfy the spec goals:
- “instant” image display (Quick Preview) even before OCR completes
- selectable OCR text overlay aligned under zoom/pan
- Wayland-first behavior, especially on wlroots compositors

---

## 1) High-level component diagram

```mermaid
flowchart LR
  subgraph UI[GTK4 / libadwaita UI Process]
    A[App entry\nCLI + .desktop] --> B{Mode?}
    B -->|--quick-preview| Q[Quick Preview Window\n(borderless overlay)]
    B -->|default| F[Full Viewer Window]
    Q --> R[Renderer\n(texture + transforms)]
    F --> R
    R --> O[OCR Overlay Layer\n(hit-testing + selection)]
  end

  subgraph IMG[Image Pipeline]
    I1[Open file / stdin] --> I2[Decode image]
    I2 --> I3[GdkTexture]
  end

  subgraph OCR[OCR Pipeline]
    T1[Prepare bitmap\n(optional preprocess)] --> T2[OCR engine]
    T2 --> T3[Layout output\n(TSV/HOCR)]
    T3 --> T4[Parsed boxes\nwords/lines + confidence]
  end

  I1 --> I2 --> I3 --> R
  I3 --> T1 --> T2 --> T3 --> T4 --> O

  subgraph Cache[Cache]
    C1[(In-memory cache)]:::cache
    C2[(Optional persistent cache)]:::cache
  end

  T4 --> C1
  C1 --> O
  T4 --> C2

  classDef cache fill:#f2f2f2,stroke:#bbb,color:#111;
```

---

## 2) Key architecture principles

### P1 — Never block the GTK main thread
- All image decoding (if not already async) and all OCR runs off-thread or out-of-process.
- UI thread handles input, layout, rendering, and drawing only.

### P2 — Render first, enrich later
- “First pixels” policy: decode + show image as soon as possible.
- OCR is treated as enrichment that may arrive later.

### P3 — Data model is in *image coordinates*
- OCR engines return bounding boxes relative to the input bitmap.
- Store OCR boxes in **image pixel coordinates** and transform to widget space at render/hit-test time.

### P4 — Graceful degradation
- If OCR engine is missing, disable OCR UI with a clear hint (viewer still works).
- If Layer Shell is unavailable, fall back to a normal undecorated “always on top” window for Quick Preview.

---

## 3) Process model and concurrency

### 3.1 Threads / tasks
- **Main/UI thread**: GTK main loop, event handling, view transforms, overlay rendering.
- **Worker thread pool**:
  - OCR jobs (CPU-heavy)
  - optional preprocessing (resize/threshold)
  - directory scanning (if needed)

Communication is message-based:
- worker emits `OcrResultReady(image_id, boxes)` to main thread via a thread-safe channel.
- main thread updates overlay state and queues redraw.

### 3.2 Cancellation
Quick Preview is ephemeral. If user closes the preview:
- cancel any pending OCR job for that image if possible
- ignore late OCR results if they arrive after dismissal (check `image_id` is still active)

---

## 4) Windowing / display architecture

### 4.1 Two window modes
1) **Quick Preview window**
- undecorated
- centered
- optionally “layer shell overlay” (best on wlroots)

2) **Full Viewer window**
- normal application window
- navigation + info panel

### 4.2 Layer Shell support (best practice)
For wlroots compositors, the recommended approach for a borderless overlay-like window is Wayland **Layer Shell**.
- Use `gtk4-layer-shell` if available to implement this cleanly in GTK4.
- If the compositor does not support Layer Shell, fall back.

> Note: Layer Shell is commonly supported on wlroots compositors but is not supported on GNOME Shell Wayland. That is a known ecosystem constraint and should be handled by a runtime capability check.

---

## 5) Image pipeline

### 5.1 Decode strategy
Decoding should:
- support common formats (png, jpg, webp, etc.)
- avoid unsafe/unbounded decoders where possible
- produce GPU-friendly textures for GTK4 rendering

**Recommended**: use `glycin` if available (sandboxed loader model), otherwise fall back to GDK/GTK default decoding.

### 5.2 Large images
For very large images:
- decode at a reasonable size for “fit to window” first (fast preview)
- optionally decode full-res on-demand when user zooms beyond 1:1

---

## 6) OCR pipeline

### 6.1 OCR backend abstraction
Define an internal interface such as:

- `recognize(image_bitmap, lang, settings) -> OcrLayout`

Then implement:
- `TesseractBackend` (default)
- `PaddleBackend` / `EasyOCRBackend` (optional future, heavy deps)

This keeps the UI/selection code independent from the OCR engine.

### 6.2 Output format choice
To enable selectable text overlay, you need:
- recognized text
- word or line bounding boxes
- confidence (optional)

**Recommended default format**: **TSV** (simple to parse, includes box coordinates per element).  
Alternative: **hOCR** (HTML-like, standard-ish, richer structure).

---

## 7) Selection overlay design

### 7.1 Data structures
Store OCR results as:

- `OcrWord { text, bbox: Rect, confidence, order }`
- `OcrResult { words: Vec<OcrWord> }`
- optional: paragraph/block grouping for better selection behavior (future)

### 7.2 Hit testing
Selection requires mapping pointer coordinates → OCR boxes. Best practice:
- build a spatial index (e.g., grid index or R-tree) over word bounding boxes in image coordinates
- at drag-select, query overlapping boxes, then order them by reading order (line then x)

### 7.3 Transform math
Maintain a view transform `T`:
- scale (zoom)
- translation (pan)
- fit-to-window baseline transform

Convert bounding boxes for render:
- `bbox_widget = T(bbox_image)`

Hit-testing does the inverse:
- `p_image = T^-1(p_widget)`

### 7.4 Copy semantics
When copying selection:
- join words with spaces
- preserve line breaks by grouping lines and inserting `\n` between them
- optional: smarter whitespace based on bounding box gaps

---

## 8) Caching strategy

### 8.1 In-memory cache (v1)
Cache OCR results per image in memory while the app stays open.

Key fields:
- file path
- file size + mtime (or hash)
- OCR language
- OCR settings (psm/oem)

### 8.2 Optional persistent cache (future / phase)
Persist OCR results to disk (SQLite or key-value store).  
This dramatically improves repeated opens for large images.

---

## 9) Error handling and observability

- Detect missing OCR engine and show a friendly “OCR unavailable” message.
- OCR failures must not crash the app.
- Log:
  - OCR start/end/error
  - decode errors
  - timing metrics (for profiling, not user-facing)

---

## 10) Security & privacy considerations

- Images are untrusted input; prefer sandboxed loaders if feasible.
- OCR is offline/local.
- Avoid loading remote URIs by default unless explicitly supported.
- Consider running OCR in a separate process with resource limits (nice-to-have hardening).

