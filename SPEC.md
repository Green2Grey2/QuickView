# QuickView — Quick Look with OCR for Linux

## Overview

A lightweight image previewer for Wayland with built-in OCR text selection. Inspired by macOS Quick Look + Preview. Two modes: quick preview (spacebar) and full viewer (double-click).

## Target Environment

- Arch Linux (and Arch-based distros)
- Wayland compositors (niri, Hyprland, Sway, etc.)
- wlroots-compatible
- GTK4 / libadwaita for native look and feel

## Two Modes

### 1. Quick Preview (spacebar from file manager)

- Opens a borderless floating window centered on screen
- Shows the image immediately, fit-to-window
- OCR runs in the background — text becomes selectable once ready
- Spacebar again (or Escape) dismisses it
- No window chrome, no titlebar — just the image
- Feels instant

### 2. Full Viewer (double-click / "Open With")

- Opens as a proper windowed application
- Image display with zoom and pan
- OCR text overlay still available
- Navigate between images in the same directory (arrow keys)
- Basic info (filename, dimensions, file size)

## Core Feature: OCR Text Selection

- Text recognized by OCR is selectable directly on the image
- Click and drag to select words, like selecting text in a PDF
- Selected text is visually highlighted
- Ctrl+C or right-click to copy
- OCR happens async — image shows first, text layer appears when ready
- Small indicator while OCR is processing

## Out of Scope (v1)

- Image editing (crop, rotate, adjust)
- Video or non-image file preview
- Thumbnail generation
- Plugin system
- PDF support

## Integration

- Registers as an image handler via `.desktop` file
- File managers (Nautilus, Thunar, PCManFM) can use it as default viewer
- Compositor keybind integration (e.g. niri `spawn` for quick preview mode)
- Reads from stdin or file path argument
- `wl-copy` for clipboard (or native Wayland clipboard)

## Dependencies

- Tesseract (OCR engine, packaged on Arch)
- GTK4 + libadwaita
- Wayland (no X11 fallback needed)
