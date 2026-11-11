# Scoped-pages pixel spec

This folder freezes the four prototypes the user approved for the redesign in [ADR 0007](../../adr/0007-pages-list-active-skills.md) (#98–#102) as a pixel spec. On 2026-11-09 the user asked: "Make sure the implemented UI is identical to the proposed prototype. Pixel perfect!" The implemented surfaces must therefore match the reference PNGs here pixel for pixel, in light and dark. Behavior stays in ADR 0007 and the issues; this folder fixes the look.

Paths below are relative to this folder.

## Scope

Compared: the surfaces the redesign changes.

- Library filter row, rows, placement area, grid and placement popover (#99).
- The workspace page (#101), and the agent and Universal pages (#102), headers included.
- The Add skill picker (#100).
- The blocking progress strip, its wash and its end toast (#98, with `Adding…` from #100).

Never compared:

- The app's 232 px sidebar. It stays as it is. In the references it is the prototypes' mock sidebar, widened to 232.
- The Library header. The prototype mocks this existing surface; it is masked by default (`spec/library-2-crops.md`).
- The content under the progress wash, and the page seen through the pill's ring or past a crop's rounded corners (`pixdiff --page`, `spec/progress-4-parity.md`). A formula checks the wash instead.
- The window frame (its rounded corners and 2 device px edge hairline), which pixdiff and `parity/progress-checks.py` leave out, and the macOS traffic lights. AppKit draws both.

## Chosen variants

| Area | Issue | Prototype | Variant the user picked (render id) | Spec | States |
|---|---|---|---|---|---|
| Library | #99 | `prototypes/library.html` | A "Popover", filter style "Segmented" (`popover`, `--filters segmented`) | `spec/library-1-states.md` … `-7-conflicts.md` | 18 |
| Workspace, agent and Universal pages | #101, #102 | `prototypes/scoped-pages.html` | C "Agents menu" (`menu`) | `spec/scoped-pages-00-index.md` … `-07-gpui-gaps.md` | 14 |
| Add skill picker | #100 | `prototypes/picker.html` | C "Two panes" (`c`) | `spec/picker-1-states.md` … `-6-gpui.md`, `spec/picker-parity.json` | 11 |
| Progress | #98 | `prototypes/progress.html` | D "Inline" strip (`inline`) | `spec/progress-1-states.md` … `-5-gpui.md` | 10 + the base `00-idle` |

Each state has `references/<area>/<NN>-<state>-<light|dark>.png`: 2360 × 1456 px, the 1180 × 728 logical window at 2x, which is what the gpui MCP `screenshot` returns. The colors are the app's tokens; `parity/reference.js` maps them onto the kit's roles in its `TOKENS` table.

## Frosted wash instead of blur

The approved progress variant dimmed and blurred the window behind the strip. GPUI has no backdrop or element blur, and AGENTS.md forbids patching dependencies to add one. The prototype was therefore changed to a frosted wash before the references were rendered: the page `background` at 70% opacity over the whole window, sidebar included. The pill sits above the wash, and a transparent blocker sits below it. No `blur()` or `backdrop-filter` is left in any bundle. Natively the wash is one window-sized quad of `background` at alpha 0.7. `parity/progress-checks.py wash` checks it on the native capture itself: `strip = round(0.7 · bg + 0.3 · base)` within 1 level. The traffic lights and the window frame cannot be washed and are left out. Issue #98's text said 55%; the user settled it at 70% (C1 in `spec/progress-1-states.md`).

## Approved additions

The user's follow-up answers added or confirmed these, and the references include them:

- **Picker (#100): an All pill** first in each workspace card's pill row. It is on for every new pick, with the tooltip "Includes agents added later". Every state with a skill selected in a workspace shows it.
- **Workspace and agent pages (#101, #102): a search field**, 220 × 28, above the card. It narrows rows by slug or description and is hidden when the place holds nothing. With no match, the page shows `No skills match '<query>'` (states 08 and 12). The pages are list-only: no folder tabs and no grid.
- **Library (#99):** turning All agents off keeps the agents held now as an explicit list (state 15). The prototype already did this.
- **Progress (#98):** none. The "Quick 0.6s" speed is a demo control that freezes the 100% frame; it is hidden in the references.

## Folder layout

- `source/`: the prototypes' sources, the pages and the kit (`kit.css`, `icons.js`, `data.js`, `shell.js`), the inliner `kit/inline.py`, and the icon generator `kit/gen_icons.py`. A file over the repository's size limit is kept as a `<name>.parts/` folder whose parts join in numeric order; the inliner reads it as one file. `kit/reference.js` links to `parity/reference.js`.
- `prototypes/`: the four self-contained bundles, built from `source/` by the inliner; never edit them. Open one in a browser to use the prototype; its variant bar switches variants. They draw only retained Hugeicons from `assets/hugeicons` and agent logos from `assets/agent-logos`, under those assets' licenses.
- `references/<area>/`: the frozen PNGs.
- `spec/`: element geometry, type and colors; comparison crops and masks; GPUI building blocks and gaps; the settled conflicts.
- `parity/`: the renderer, the scenarios, the pixel diff, the progress checks and the native fixture.

## Regenerate the references

Needs Google Chrome (set `CHROME` for another path), node 18+ and python3; no packages.

```sh
parity/render-ref.sh <area> <variant> <light|dark> <scenario> out.png [--filters segmented] [--spec sel.json out.json]
parity/render-all.sh [area …]           # every state, light and dark, into references/
parity/render-all.sh --check [area …]   # re-render into a temporary folder and compare bytes
```

- A render is deterministic. Headless Chrome runs over the DevTools pipe with an exact 1180 × 728 viewport at scale 2 in sRGB. Animations, transitions and the caret are off, and page timers run on a manual clock that only scenarios tick. A virtual pointer drives `:hover`. When frozen, `render-all.sh --check` matched all 108 PNGs byte for byte.
- `render-ref.sh` stages the bundle in a temporary folder, swaps its inlined reference mode for `parity/reference.js`, and puts `parity/scenarios/<area>.js` beside it. A page error paints a red banner and exits 1.
- `--spec` writes the computed styles of the selectors in `parity/selectors/` (rects in logical window px). `parity/spec-rows.py` prints such a dump one element per line.
- To add a state, register a scenario in `parity/scenarios/<area>.js` (the helpers are listed at the top of `parity/reference.js`), add it to the list in `render-all.sh`, and add a row to the area's states table.
- To change a prototype after a user decision, edit its source in `source/`, rebuild its bundle with `python3 -I source/kit/inline.py source/<area>.html prototypes/<area>.html`, re-render the affected states, update the spec rows, and commit them together. After an asset change, regenerate the icons first with `python3 -I source/kit/gen_icons.py <repo root> source/kit/icons.js.parts/2-generated.js`. Keep each part under about 12 KB; split a growing one at a blank line into the next number.

## Parity procedure

1. **Fixture.** Build the native fixture once (`parity/fixture/README.md`). Before each run, restore it with `parity/fixture/restore.sh $FX`, or `restore.sh $FX problems` for the edited and foreign copies (picker and progress problem states, workspace state 06).
2. **Launch.** Run `cargo build -p skillbinder --features native-test --locked`, then `target/debug/skillbinder --mcp --data-dir $FX/root/data --home $FX/root/home`. The window opens at 1180 × 728. For the dark references, switch the theme with the sidebar toggle.
3. **Reach the state** through the gpui MCP. Use the steps in the area's states table. Let animations settle, and leave the pointer where the table says (parked when the reference has none).
4. **Screenshot** with the MCP `screenshot` (the full window, 2360 × 1456) and save it unchanged as a PNG. It must be in sRGB, as the references are. The screenshot holds the display's color codes, untagged, so first set the display's color profile to sRGB IEC61966-2.1 (System Settings › Displays). Check it on a capture with nothing over the sidebar: the interiors of the Frontend and Backend preset icons must read exactly `#3b82f6` and `#f59e0b`. On #104's P3 display they read (78, 128, 238) and (233, 162, 59): grays keep their values, every tinted pixel moves, and converting back to sRGB is not exact.
5. **Compare.** Run `parity/pixdiff references/<area>/<NN>-<state>-<theme>.png native.png diff.png --tolerance 8 --crop <crop> [--ignore <rect> …] --side side.png --json`. Rects are logical px; pixdiff scales them by 2 and leaves out the window frame: pixels that are not opaque, and those within 2 device px of them or of the window edge.

| Area | Default crop | Per-state masks and extra checks |
|---|---|---|
| Library | `232,44,948,684` (the page; header masked) | `spec/library-2-crops.md` |
| Workspace and agent pages | `232,0,948,728` (the main column, header included) | `spec/scoped-pages-00-index.md`: carets, focus ring, wash samples under dialogs |
| Picker | `56.5,45.5,1067,637` (the dialog), then tooltip crops | `spec/picker-1-states.md` and `spec/picker-parity.json`: carets, scrollbar, wash samples, shadow band |
| Progress | each state's pill or toast box | `spec/progress-4-parity.md`: the spinner is always masked; wash and halo through `parity/py parity/progress-checks.py` |

## Pass criteria

A state passes in a theme when all of these hold:

- **No geometry or color mismatch.** After pixdiff's antialiasing, snapping and icon filters (`aa_filtered`, red in `diff.png`), no pixel remains outside text glyphs. A red pixel on a box edge, fill, border, radius, shadow, icon or logo is a defect.
- **Pixel snapping stays within the state's budget.** GPUI puts icons, logos and box edges on whole device pixels, where Chrome antialiases them at fractional positions. pixdiff counts such a pixel as `snap` (green in `diff.png`) when a shift of at most 1 device px explains it both ways: in every channel, each image's value lies within the other's 3×3 neighbourhood (± tolerance). A missing or extra line, a wrong fill or a 2 device px shift fails one way and stays red. `snap.pixels + icon.pixels` must not exceed the state's budget in the area's crop table. The budget is what catches a whole region moved by 1 device px.
- **Icon residue stays small (G18).** An SVG icon or logo whose box (from the reference's `--spec` dump) starts at a fractional device position is drawn by GPUI on whole device pixels, so a detail 1 or 2 device px wide can land at another sub-pixel phase. pixdiff counts the red pixels left in such a box, widened by 1 device px, as `icon` (purple), but only while the box has at most 12 and each is within 40 levels of passing the snap test. Otherwise they all stay red, so a wrong, missing, recolored or moved icon still fails.
- **Text antialiasing is allowed and listed.** Chrome rasterizes with Skia and GPUI with CoreText, both with SF Pro, so glyph edges differ. Yellow pixels are allowed. Red pixels are allowed only inside a text run's box, and only when the native text bounds (MCP `get_element_bounds`, the layout box, not the ink) match the spec rect within 0.5 logical px. Layout does not change with the theme, so a run that passes in light passes in dark. Light-on-dark glyphs look up to 1 px bolder because CoreText dilates them (Library G13); that shows in the ink only. The PR lists each such run with its state and pixel count.
- **Only the spec's masks.** Required masks (spinner, caret, scrollbar, traffic lights) need no comment. An optional mask for a GPUI gap still open, such as the dash pattern, the checkbox mark or the progress bar's end at the pill's curve, is listed in the PR with its gap id.
- **The other checks pass.** The wash formula, the halo ratio and the wash samples pass as the area's spec states.

A budget is set once per state, from the native build of the area's parity PR: 1.2 × the larger of the light and dark `snap + icon` counts, rounded up to two significant figures. That PR shows that moving the whole native capture by 1 device px, in each direction, exceeds it. Text glyph edges make up most of the count, so budgets differ by state: one cap per area would let a moved page through in the sparser states. `--no-snap` turns the test off.

The evidence covers every reference state of the area, in light and dark. Attach the native screenshots, the `diff.png` and `side.png` files, and the pixdiff JSON to the PR, and record it apart from the native interaction check.

## GPUI gaps per area

Where GPUI or GPUI Kit cannot reach a CSS value as-is, the spec gives the closest equivalent and the expected delta.

- **All areas.** GPUI snaps quads, SVG icons and images to whole device pixels (`gpui-pre` `window.rs` `paint_quad`, `paint_svg`, `paint_image`); Chrome antialiases them at fractional positions. Up to 1 device px of this is snap, within the state's budget (Pass criteria). GPUI's shadow `blur_radius` is a sigma: use half the CSS blur. A spread shadow keeps the element's radius. Content masks are rectangles, so nothing clips to a rounded corner. GPUI's default line height is about 1.618, so set every line height explicitly. Kit Button, Avatar, Tag and Tooltip defaults differ in size, padding, text size or delay, so the app owns those controls (Decisions, 1); the Kit Input matches with overrides.
- **Library** (`spec/library-6-gpui.md`, G1–G19): GPUI draws 2/1 px dashes where Chrome draws about 3/2 px, so use a custom path or the dash masks. GPUI has no CSS `filter`; use image grayscale. Kit Switch and the segmented TabBar cannot take the 30 × 18 and 28 px geometry. Kit Popover clamps 4 px from the window edge, where the prototype clamps 8. GPUI has no container queries and no auto-fill grid.
- **Workspace and agent pages** (`spec/scoped-pages-07-gpui-gaps.md`, G1–G10): Kit Notification cannot draw the toast, so use an app toast. Kit Dialog's shadow is twice as soft. Kit tooltips prefer the top and wait 500 ms. Kit Checkbox draws the Hugeicons tick, not the CSS check; mask the box interior.
- **Picker** (`spec/picker-6-gpui.md`): dashed pills, tooltip placement and delay, and the check mark, as above. GPUI applies opacity per primitive, so use the flattened colors in `spec/picker-5-overlays.md`. Build the focus ring from two absolute children. The Kit scrollbar fades in on hover.
- **Progress** (`spec/progress-5-gpui.md`, G1–G13): the traffic lights and the window frame stay unwashed. GPUI's 4-sample shadow blur leaves the light halo up to 3/255 off, which the halo check allows. Kit Spinner turns at another speed, so use a custom rotation. Kit Notification cannot draw the toast. GPUI has no CSS transitions; motion falls outside still parity.

## Decisions (2026-11-09)

The user settled every open point on 2026-11-09. This spec is authoritative for every visual value; where an issue's text disagrees with it, the spec wins.

1. **App-owned controls where Kit cannot match.** Wherever GPUI Kit cannot hit the prototype pixel for pixel, the app builds an exact control of its own: the switch, the segmented control, the toast, the tooltip, the small button text, the popover placement with its 8 px margin, the avatar with its outside ring, the tag, the dialog surface and the dashed outline. Each lives in a small module under `crates/ui/src/native_view/` (a `controls/` folder) and is reused across slices. Kit stays where styling matches exactly, such as the Input.
2. **The update glyph is cyan** (`info`, `#06b6d4 | #22d3ee`) on every redesigned surface: Library, the workspace and agent pages, and the picker. The rest of the app, such as the skill page header, keeps warning yellow. The scoped-pages and picker prototypes were changed to match the Library.
3. **Prototype glitches are fixed in the prototype.**
   - The Library placement button's hover and open fill is the menu highlight, fg at 0.08, over the row's own fill (`#e2e2e2 | #373737` on a hovered row, `#ececec | #212121` over the card), so it shows on a hovered row. The avatar rings inside take the same composite (`spec/library-4-rows.md`).
   - Avatar rings inside the agents menu take the menu surface (`#ffffff | #262626`), not the page background (`spec/scoped-pages-03-menu.md`).
4. **Real data.** The kit's `data.js` matches the native fixture:
   - your agents are listed Universal first, then by name: Universal, Claude Code, Cursor, Windsurf, in the sidebar, menus and stacks;
   - Universal's members are the registry's `~/.agents/skills` agents: Cline, Codex, Dexto, fx, Kimi Code CLI, Loaf, Pi, Sarvam Code, Warp, Zed;
   - a project's `.claude/skills` is named "Claude Code, Cline, Cursor, fx, Warp", and a workspace with no agents offers the same 12 directories as the native app (`spec/library-5-popover.md`);
   - rows of many logo pills wrap onto further lines, aligned to the trailing edge, wherever pills appear.
5. **Earlier conflicts follow the prototype.** Among them: the 70% wash, the `loading` spinner glyph, the 2 px bar, the app toast, 18 px avatars with a 5 px overlap, the outlined Updates chip, one "No skills match" empty state with Clear filters, the prototype's spacing and gutters, the `robot` overlap glyph, the "Changed here" mark, no clear button in the search field, the picker at 92% × 90% of the window, and the prototype's review list and edited-copy prompt. The picker's scrim click does not close it, as #100 says.
6. **The checkbox tick** stays the retained Hugeicons `check` (AGENTS.md icon rule): a documented delta, masked in the parity check.

The settled items are recorded in `spec/library-7-conflicts.md`, `spec/scoped-pages-07-gpui-gaps.md` (A, D), `spec/picker-2-dialog.md`, `spec/picker-5-overlays.md` and `spec/progress-1-states.md`.
