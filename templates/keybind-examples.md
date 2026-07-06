# Keybind / Launcher Examples (Quick Preview)

Quick Preview is designed to be launched as:

    quickview --quick-preview /path/to/image.png

How you obtain `/path/to/image.png` from a file manager varies by desktop environment and file manager.

## Practical approaches

### A) File-manager “Custom Action” (recommended)
Many file managers allow custom context-menu actions which pass the selected file path to a command.
Configure that action to call:

    quickview --quick-preview %f

(Where `%f` is whatever placeholder your file manager uses.)

### B) Desktop-file action
If your file manager supports desktop actions, the provided `quickview.desktop` includes:

- “Quick Preview (borderless)” → `quickview --quick-preview %f`

### C) Compositor keybind (advanced)
Compositors can bind keys to commands, but they generally do *not* know which file is selected inside your file manager.
This approach works best if you have a helper that can query the selected file (desktop-specific).

The recipes below sidestep that limitation with the most common use case:
previewing the **newest screenshot**. Adjust the glob to your screenshot
directory.

QuickView is single-instance: pressing the bind while a preview is already
open **closes it** (toggle), so one bind is enough for both open and dismiss.

> Rule-matching cheat sheet: **layer rules** match the layer-surface
> namespace `quickview` (the normal path on wlroots compositors), while
> **window rules** match the app ID `io.github.Green2Grey2.QuickView` — those
> only apply on the fallback path when gtk4-layer-shell is unavailable.
> They are different matchers; a window rule will not catch the layer surface.

#### niri

```kdl
binds {
    Mod+Shift+Space {
        spawn "sh" "-c" "quickview --quick-preview \"$(ls -t ~/Pictures/Screenshots/* | head -n1)\"";
    }
}
```

Optional cosmetics for the overlay (matches the layer-surface namespace):

```kdl
layer-rule {
    match namespace="quickview"
    // e.g. shadow { on; }
}
```

#### Hyprland

```conf
bind = SUPER SHIFT, SPACE, exec, quickview --quick-preview "$(ls -t ~/Pictures/Screenshots/* | head -n1)"

# Optional cosmetics (layer surface, normal path):
layerrule = blur, quickview

# Fallback path only (no layer shell): keep the borderless window floating.
windowrulev2 = float, class:^(io\.github\.Green2Grey2\.QuickView)$
```

#### Sway

```conf
bindsym $mod+Shift+space exec quickview --quick-preview "$(ls -t ~/Pictures/Screenshots/* | head -n1)"

# Fallback path only (no layer shell): keep the borderless window floating.
for_window [app_id="io.github.Green2Grey2.QuickView"] floating enable
```

