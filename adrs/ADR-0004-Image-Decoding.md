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

