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

