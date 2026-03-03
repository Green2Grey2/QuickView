# ADR-0002: UI Toolkit Choice (GTK4 + libadwaita)

**Status:** Accepted  
**Date:** 2026-03-02

## Context

The spec calls for a “native look and feel” on Wayland, targeting Arch and wlroots compositors.
We need:
- high-quality image rendering
- input/gesture handling (zoom/pan)
- Wayland-native behavior
- a modern Linux desktop UI style

## Decision

Use **GTK4 + libadwaita** as the primary UI toolkit.

## Rationale

- Matches the spec requirement for a native GNOME/Adwaita look and modern GTK4 rendering.
- Good accessibility and input handling primitives.

## Consequences

### Positive
- Clean modern UI; consistent GNOME experience.
- Mature Wayland support.

### Negative
- KDE users may prefer Qt aesthetics.
- libadwaita theming is intentionally constrained.

## Alternatives considered

- **Qt 6** (KDE-first look)
- **SDL/wgpu** (max control, but you rebuild everything: clipboard, accessibility, UI controls)
- **Flutter** (fast UI development, but heavier runtime; less native)

