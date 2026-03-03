#!/usr/bin/env bash
set -euo pipefail

sudo pacman -S --needed \
  base-devel rustup pkgconf \
  gtk4 libadwaita \
  tesseract tesseract-data-eng \
  gtk4-layer-shell

# Optional:
# sudo pacman -S --needed wl-clipboard
# sudo pacman -S --needed glycin glycin-gtk4

rustup default stable
rustup component add rustfmt clippy

echo "Done. Now run: cargo build"
