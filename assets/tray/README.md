# Tray artwork

The menu bar (macOS) and system tray (Windows, Linux) icon: SkillBinder's rune from `assets/icon.png`, a stave bound by a diamond with a hook at each end, drawn from its 18-unit template glyph `GLYPHS.bindrune` in `docs/design/brand/source/icons.js`, stroked 1.4 units wide with round caps and joins (spec rule 14, ADR 0021).

`docs/design/brand/source/tray_icons.py` draws every file: `python3 docs/design/brand/source/tray_icons.py assets/tray`.

- `tray-template.png`: 44 px, black on transparent, the glyph 36 px tall. macOS draws it as a template image at 22 pt, the rune at its own 18 pt, recolored for the menu bar.
- `tray-black-<size>.png` and `tray-white-<size>.png`, 16, 20, 24, and 32 px: the glyph for light and for dark panels on Windows and Linux, picked by the system appearance. Each carries a one-pixel halo of the other color, which vanishes on the panel it is picked for and keeps it visible on one whose shade differs, such as GNOME's top bar, which stays dark under a light theme. Windows shows the 32 px file, scaled to the notification area; Linux offers every size to the StatusNotifier host.
- `tray-template-dot.png`, `tray-black-dot-<size>.png`, and `tray-white-dot-<size>.png`: the same glyphs with a dot at the top right, in the glyph's color and cut out of the rune by a clear gap, shown while something waits for the user (spec D6).
