# Palette playground

`prototypes/palette.html` is where the window's colors are explored and chosen. The current palette (a blue `#0077B6` accent, cool slate neutrals, and a flat neutral sidebar selection) was picked in it on 2026-11-20. Open the bundle in a browser. It draws the split-columns shell (`DESIGN.md` § Layout) with the Library, a workspace, Explore, and the Import skills dialog, colored entirely from the palette being edited.

- **Start from** loads a starting point: Current (today's `DESIGN.md`), A Colored glyphs, B Tiles, C Tinted canvas, D Item color.
- **Accent** and **Accent goes on** pick the accent and the roles that use it; a role turned off uses graphite (the foreground color) or a neutral fill instead.
- **Neutrals** and **Sidebar** generate the grays (OKLCH hue and tint) and the sidebar canvas, selection style, icon style, label ink, and per-item icon colors.
- **Tokens** lists every generated token for the shown theme; picking a color pins it until a generator control changes. The contrast list flags pairs under 4.5:1 for text and 3:1 for glyphs.
- **Hold to compare with Current** shows today's palette while held.
- **Export** produces a paste-ready block: the direction, the accent roles, and every color token in `DESIGN.md` names with `-dark` siblings, marking new tokens and changed values against today's.

The playground keeps its state in the browser's local storage.

## Changing the palette

1. Open `prototypes/palette.html`, start from Current, adjust, and copy the export.
2. Put the colors in `DESIGN.md`, run `cargo run -p xtask -- design-tokens`, and lint with `npx -y @google/design.md@0.4.0 lint DESIGN.md`. Tokens the export marks `# new` (such as `nav-*`) need code that uses them; add one only when something draws it. A changed selection style or accent role is a code change too.
3. Make Current match: update `CURRENT` and the `cur` variant in `source/palette-data.js`, then rebuild the bundle.

## Sources

`source/` holds the playground's sources; the bundle is generated from them and never edited by hand:

- `palette.html`: the page.
- `palette.css`: the control panel and the mock window's styles, every mock color a CSS variable.
- `color.js`: OKLCH and sRGB conversion, mixing, and WCAG contrast.
- `palette-data.js`: today's palette, the choices, the starting points, the saved state, and the generator.
- `mock.js`: the mock window.
- `controls.js`: the control panel, the export, and the events.

Rebuild the bundle with the shared inliner:

```sh
python3 -I docs/design/scoped-pages/source/kit/inline.py docs/design/palette/source/palette.html docs/design/palette/prototypes/palette.html
```
