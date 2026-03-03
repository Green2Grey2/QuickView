# Integration Guide

## CLI usage

- Full viewer (double-click / “Open With”):

```bash
quickview /path/to/image.png
```

- Quick preview (spacebar-style):

```bash
quickview --quick-preview /path/to/image.png
```

- Read file from stdin (use `-`):

```bash
printf '%s\n' /path/to/image.png | quickview --quick-preview -
```

## Compositor keybind examples

See: `templates/keybind-examples.md`.

## Nautilus (GNOME Files) “Sushi-style” preview

GNOME’s Sushi previewer is DBus-activated and expects applications to call the `ShowFile` method on the `org.gnome.NautilusPreviewer2` interface.

If you want **deep Nautilus integration**, one long-term option is to provide an optional DBus service that implements that interface and forwards to QuickView’s Quick Preview window.

This repo only includes a stub plan (see `docs/PHASED_PLAN.md`).

