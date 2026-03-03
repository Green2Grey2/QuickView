# ADR-0006: OCR Output Format (TSV first)

**Status:** Accepted  
**Date:** 2026-03-02

## Context

Selectable OCR overlay requires bounding boxes per recognized text unit.
We need an output format that is:
- easy to parse
- stable across versions
- provides boxes and text

## Decision

Implement **TSV** output parsing first.

## Rationale

- TSV is straightforward to parse.
- Includes coordinates per recognized element.
- Good fit for word-based overlay selection.

## Consequences

### Positive
- Faster to implement correctly.
- Lower risk than HTML parsing.

### Negative
- Less expressive than hOCR for document structure.
- Might need extra heuristics for reading order.

## Alternatives considered

- hOCR (HTML-like, richer structure; more parsing complexity)
- ALTO XML (richer, but more complex and less needed for v1)

