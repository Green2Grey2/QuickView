# ADR-0007: Clipboard & Copy Implementation

**Status:** Accepted  
**Date:** 2026-03-02

## Context

QuickView must copy selected OCR text reliably on Wayland.
Wayland clipboard access is mediated by the compositor; shelling out to tools like `wl-copy` works but adds an external dependency.

## Decision

Use **GTK’s native clipboard API** for copy by default.  
Optionally support `wl-copy` as a fallback when running in minimal environments.

## Consequences

### Positive
- Fewer external dependencies.
- Most reliable inside a GTK app.

### Negative
- Edge cases may exist in very minimal compositors; fallback helps.

## Alternatives considered

- **wl-copy only** — simpler but adds a hard runtime dependency on wl-clipboard
- **D-Bus clipboard protocol** — lower-level, no advantage over GTK's built-in API

