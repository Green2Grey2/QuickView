# ADR-0010: Threading, Cancellation, and Result Delivery

**Status:** Accepted  
**Date:** 2026-03-02

## Context

GTK UI must remain responsive. OCR is CPU-heavy. Quick Preview may be dismissed quickly.
We need:
- background OCR execution
- safe result delivery to the UI thread
- cancellation or “ignore late results”

## Decision

- Run OCR in worker threads (or a dedicated worker process).
- Communicate results via message passing to the UI thread.
- Implement cancellation as:
  - cancel if supported by backend, else
  - ignore results not matching the active `image_id`.

## Consequences

### Positive
- Responsive UI at all times.
- Prevents stale OCR overlays when navigating quickly.

### Negative
- Some OCR work may be wasted if user dismisses quickly (acceptable tradeoff for responsiveness).

