# ADR-0009: OCR Caching Strategy

**Status:** Accepted  
**Date:** 2026-03-02

## Context

OCR is expensive. Users often preview the same screenshots repeatedly.
We want repeat opens to be faster without complicating v1.

## Decision

Implement **in-memory cache** first.
Design interfaces so an optional persistent cache can be added later.

## Details

Cache key should include:
- file path
- file size + modification time (or hash)
- OCR language
- OCR settings (psm/oem etc.)

## Consequences

### Positive
- Immediate speed-up within a session.
- Minimal implementation complexity.

### Negative
- No persistence across app restarts unless a later persistent cache is added.

## Alternatives considered

- **Persistent on-disk cache from day one** — more complex, needs eviction policy and cleanup; deferred to a later phase
- **No cache** — simplest but OCR re-runs on every open, noticeably slow for repeat views

