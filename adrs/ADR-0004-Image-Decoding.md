# ADR-0004: Image Decoding Strategy (prefer glycin)

**Status:** Accepted  
**Date:** 2026-03-02

## Context

QuickView opens arbitrary image files from disk. Image decoders have historically been a source of security bugs.

We need:
- broad format support
- good performance
- a safety-conscious approach

## Decision

Prefer **`glycin`** for decoding where available, with a fallback to GTK/GDK default decoding.

## Rationale

Glycin decodes via sandboxed modular loaders, improving safety when handling untrusted images.

## Implementation notes (2026-07-05)

- The `glycin` client crate is **always compiled**; no cargo feature. Which
  backend is used is decided **at runtime** by probing the installed loader
  configs once per session (`quickview-ui/src/decode.rs`). The client links
  libseccomp, lcms2, and fontconfig — ubiquitous on any GTK-capable system,
  and required at build time regardless of which backend runs (see
  `docs/DEPENDENCIES.md`).
- The GDK fallback is strictly **session-wide, never per-file**: a file glycin
  rejects is a failed load. Retrying it with the unsandboxed GDK decoder would
  let a crafted image reach the unsandboxed path simply by making the sandboxed
  loader fail.
- Both backends run off the GTK main thread (glycin in its sandboxed loader
  process, GDK on a worker thread); stale decodes are dropped via a monotonic
  job ID, the same pattern used for OCR (ADR-0010).

## Consequences

### Positive
- Better security posture by default.
- Clean output path to `GdkTexture`.

### Negative
- Extra dependency.
- Some distros/environments may not ship glycin; must keep a fallback.

## Alternatives considered

- GdkPixbuf-only decoding
- libvips (excellent for huge images, but bigger dependency footprint)

