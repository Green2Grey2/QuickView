# ADR-0001: Implementation Language and Build System

**Status:** Accepted  
**Date:** 2026-03-02

## Context

QuickView is a GTK4/libadwaita Wayland application with:
- custom rendering (image + overlay)
- background OCR workloads
- potential FFI integration (OCR engine APIs)
- packaging targets (Arch + optional Flatpak)

We need a language/build choice that supports:
- safe concurrency (OCR on background threads)
- long-term maintainability
- good GTK4 ecosystem support

## Decision

**Default recommendation:** **Rust + Cargo** (GTK4 bindings) for the main application.

## Rationale

- Rust provides memory safety for complex UI + async background processing.
- Modern GNOME ecosystem increasingly supports Rust (and key recommended libraries like `glycin` are Rust-native).
- Cargo makes dependency/version management straightforward.

## Consequences

### Positive
- Strong safety guarantees and good concurrency primitives.
- Good fit for sandboxed image decoding (glycin) and structured parsing.

### Negative / risks
- FFI work may be needed for best-in-class Tesseract integration (if not shelling out to CLI).
- GTK Rust ecosystem evolves quickly; maintainers must keep pace.

## Alternatives considered

1) **C + Meson**
- Pros: canonical GNOME stack; easiest access to GTK internals and C libraries.
- Cons: higher memory-safety burden; more careful threading required.

2) **Vala + Meson**
- Pros: GNOME-friendly, fast iteration, good GTK bindings.
- Cons: smaller ecosystem than Rust; fewer modern libraries.

3) **Python + PyGObject**
- Pros: fastest iteration; easiest to prototype OCR parsing and selection.
- Cons: performance overhead; packaging and distribution complexity; harder to guarantee “instant” feel under load.

## Notes

The architecture is designed so language choice is not coupled to core product behavior. If the team is already strong in C or Vala, those options are viable.

