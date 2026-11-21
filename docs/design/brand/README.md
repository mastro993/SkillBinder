# Brand playground

`prototypes/brand.html` is where SkillBinder's brand was explored and chosen: the app icon, the logo, and the slogan, in a rune and arcane direction.

## The brand

The user chose it on 2026-11-21; the playground opens on it.

- **Icon**: A Rune on Stone. The rune glows blue (`#1E84C4` glow, `#E4F2FE` core) on a dark slate tile with a fine grain, cut to Apple's continuous-corner shape so macOS 26 shows it unframed. At 32 px and below its stroke is thicker.
- **Logo**: the icon beside the Carved caps wordmark, SKILLBINDER in spaced Optima capitals. Optima is a macOS system font: before the wordmark is published as artwork, set it in a typeface licensed for logos or draw it as custom lettering.
- **Slogan**: "One library to bind them all."

The first round, literal binders (rings, a card fan, a B monogram, index tabs, a key ring), was rejected: the name means a collection of skills like magic. The rune copies no historical rune on purpose: an S and B bindrune on one stave reads as the Bluetooth logo, itself the bindrune of Hagall and Berkano; several Elder Futhark runes carry hate-symbol associations; and a hexagram reads as the Star of David.

## The playground

Open the bundle in a browser. Every color comes from `DESIGN.md`: the `#0077B6` accent, its soft and deep steps, and the slate neutrals.

- **Icon concept** frames one original rune, a stave bound by a diamond with a hook at each end, five ways: A Rune (alone, glowing), B Sigil (in a summoning circle), C Rune gem (a cut gem in its socket), D Runestone (carved in stone), E Constellation (drawn in stars).
- **Icon background** picks the Default appearance's tile: Stone, Abyss, or Frost. The Dark appearance is always Stone and the Tinted one grayscale, as macOS 26 draws them.
- **Wordmark** and **Slogan** pick the logo's type treatment and line; "Your own…" takes a typed slogan.
- The board shows the concepts side by side, the chosen icon in each appearance and at 128 to 16 px, in the Dock and as a menu bar template glyph, the logo on window, stone, and abyss surfaces, and the onboarding welcome.
- **Copy choice** copies a summary of the picks.

The playground keeps its state in the browser's local storage.

## Sources

`source/` holds the sources; the bundle is generated from them and never edited by hand:

- `brand.html`: the page.
- `brand.css`: the control column and the board.
- `icons.js`: the rune, its five framings, their color schemes, and the menu bar glyphs, drawn as SVG.
- `brand.js`: the choices, the board, and the events.
- `export.html` and `export-icons.sh`: the app icon files. The script captures `export.html` with Google Chrome and writes `assets/icon.icns` (macOS, on Apple's 824-in-1024 icon grid), `assets/icon.ico` (Windows), and `assets/icon.png` (Linux, 256 px). Run it on macOS after changing the icon and commit the three files.

Rebuild the bundle with the shared inliner:

```sh
python3 -I docs/design/scoped-pages/source/kit/inline.py docs/design/brand/source/brand.html docs/design/brand/prototypes/brand.html
```
