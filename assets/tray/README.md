# Tray artwork

The menu bar (macOS) and system tray (Windows, Linux) icon: SkillBinder's mark from `assets/icon.png`, a page beside its binder spine, redrawn as a one-color glyph with whole-pixel edges at each size (spec rule 14, ADR 0021).

- `tray-template.png`: 44 px, black on transparent. macOS draws it as a template image at 22 pt, recolored for the menu bar.
- `tray-black-<size>.png` and `tray-white-<size>.png`, 16, 20, 24, and 32 px: the glyph for light and for dark panels on Windows and Linux, picked by the system appearance. Each carries a one-pixel halo of the other color, which vanishes on the panel it is picked for and keeps it visible on one whose shade differs, such as GNOME's top bar, which stays dark under a light theme. Windows shows the 32 px file, scaled to the notification area; Linux offers every size to the StatusNotifier host.
