#!/usr/bin/env bash
set -euo pipefail

sudo pacman -S --needed \
  base-devel rustup pkgconf \
  gtk4 libadwaita \
  tesseract tesseract-data-eng \
  gtk4-layer-shell

# Enforce minimum GTK version required by the UI code (Snapshot/GSK APIs).
if ! pkg-config --atleast-version=4.10 gtk4; then
  echo "Error: GTK4 >= 4.10 is required. Found: $(pkg-config --modversion gtk4)" >&2
  exit 1
fi

# Optional:
# sudo pacman -S --needed wl-clipboard
# sudo pacman -S --needed glycin glycin-gtk4

rustup default stable
rustup component add rustfmt clippy

echo "Done. Now run: cargo build"
