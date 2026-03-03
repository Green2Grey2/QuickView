# ADR-0005: OCR Engine Choice (Tesseract default)

**Status:** Accepted  
**Date:** 2026-03-02

## Context

QuickView requires offline OCR with bounding boxes to enable selection.
Constraints:
- must run locally (privacy)
- must be packageable on Arch
- must support multiple languages

## Decision

Use **Tesseract** as the default OCR engine.

## Rationale

- Widely packaged.
- Works offline.
- Offers structured output (TSV/hOCR) suitable for text overlay selection.

## Consequences

### Positive
- Lightweight relative to deep-learning OCR stacks.
- Integrates via CLI or `libtesseract`.

### Negative
- For complex “scene text”, DL-based engines can be more accurate.
- Accuracy depends on model choice and preprocessing.

## Alternatives considered

- PaddleOCR (high accuracy, heavier, typically Python stack)
- EasyOCR (easy to use, heavier Python/ML dependencies)
- Cloud OCR APIs (not recommended by default due to privacy and network)

