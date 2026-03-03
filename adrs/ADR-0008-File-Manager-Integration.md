# ADR-0008: File Manager Integration Strategy (Spacebar Quick Preview)

**Status:** Accepted  
**Date:** 2026-03-02

## Context

The spec wants “spacebar quick preview from file manager”.
Linux file managers implement this differently:
- GNOME Files (Nautilus) uses Sushi (DBus previewer).
- Other file managers may not support spacebar preview at all.
- wlroots compositors can bind keys to arbitrary commands.

## Decision

Primary integration path (wlroots-friendly):
- Provide `quickview --quick-preview <path>` and document compositor keybind recipes.

Optional advanced integration:
- Implement a GNOME Sushi-compatible DBus interface so Nautilus can invoke QuickView as a previewer.

## Consequences

### Positive
- Works across many wlroots setups without writing plugins.
- Leaves room for desktop-specific integrations when needed.

### Negative
- “Spacebar in file manager” will not be universal unless desktop-specific work is done.

