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

## Implementation notes (2026-07-05)

The implementation went **straight to on-disk**, revising the decision above:

- Quick Preview spawns a fresh process per invocation, so an in-memory cache
  buys the flagship workflow nothing; on-disk also covers in-session revisits
  (re-reading a few-KB JSON is effectively instant).
- `cache.rs` key derivation already existed: blake3 of
  `path + lang + mtime + size` → `<cache>/ocr/<key>.json` (Linux:
  `~/.cache/quickview/ocr/`). Staleness needs no invalidation logic — an
  edited file produces a new key and is simply a miss. The mtime is hashed at
  full nanosecond precision so a same-second rewrite with an unchanged byte
  length still misses, and the key is snapshotted *before* tesseract runs so
  a file edited mid-OCR stores its stale result under the old key (which the
  edited file then correctly misses) rather than the new one. The path is
  derived from the lowercased app name, so the app-ID rename (done:
  io.github.Green2Grey2.QuickView) did not move it on Linux.
- Tesseract is invoked with no psm/oem flags, so the OCR settings in the key
  are `lang`, the optional `tessdata_dir` (presence marker keeps `None` and
  empty distinct), and the **effective** downscale target (`full` or
  `WxH`) — the target rather than the configured `max_dimension` threshold,
  so below-threshold images keep their entries across threshold edits. **The
  rule stands: any newly configurable OCR setting (psm/oem) must join the
  key.**
- Writes are atomic (temp file + rename in the same directory): concurrent
  QuickView processes are a designed use case.
- Entries are created `0600` in `0700` directories — they hold recognized
  text from the user's images, so they must not rely on the home directory
  for privacy. Modes apply at creation only: entries written by builds
  predating this (or a pre-existing lax cache dir) keep their old modes;
  clearing `~/.cache/quickview/ocr` resets everything.
- Empty results are cached (text-free images shouldn't re-run tesseract);
  failures are not cached, so transient errors retry on the next open. A
  failed cache write is a warning, never a failed OCR.
- **No eviction in v1.** Entries are a few KB; the directory grows unbounded
  but slowly. Manual cleanup: `rm -rf ~/.cache/quickview/ocr`. Real eviction
  arrives with the Phase 8 persistent SQLite cache that replaces this scheme.

