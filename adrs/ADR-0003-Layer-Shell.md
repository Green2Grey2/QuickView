# ADR-0003: Quick Preview Overlay Window via Wayland Layer Shell

**Status:** Accepted  
**Date:** 2026-03-02

## Context

Quick Preview mode must feel like “Quick Look”:
- borderless
- above other windows
- centered
- easy to dismiss
- works well on wlroots compositors (Sway, Hyprland, niri)

Standard toplevel windows do not always behave like overlays across compositors.

## Decision

Use the **Wayland Layer Shell protocol** for Quick Preview where supported, implemented via **`gtk4-layer-shell`**.

## Rationale

Layer Shell is designed for desktop-shell-style surfaces:
- explicit layering (overlay/top/bottom/background)
- anchoring and input semantics

GTK4 does not expose Layer Shell directly; `gtk4-layer-shell` provides a supported integration.

## Consequences

### Positive
- Predictable overlay behavior on wlroots compositors.
- Better “instant preview” UX.

### Negative
- Not supported on GNOME Shell Wayland (requires fallback to normal window).
- Requires a runtime detection and dual path implementation.

## Alternatives considered

- Undecorated always-on-top toplevel window only (simpler, less consistent).
- Desktop-specific preview integrations (GNOME Sushi DBus, KDE-specific solutions).

