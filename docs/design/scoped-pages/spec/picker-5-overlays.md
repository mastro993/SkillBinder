# Add skill picker: frozen visual spec (5/6) — tooltips, copy, z-order, timings

Colors are `light | dark`.

## Tooltip box (every tooltip in the picker)

| property | value |
|---|---|
| box | padding `4 8`, border 1, radius 6, max-width 260, multi-line on `\n` (`white-space: pre-line`) |
| text | 12 / 400, line height 16.2 (1.35); one line → box height 26.2, two lines → 42.4 |
| colors | bg `#ffffff \| #0a0a0a` (app `popover` token); border `#e5e5e5 \| #262626`; text `#0a0a0a \| #fafafa` |
| shadow | `0 4px 6px -1px #0000001a, 0 2px 4px -2px #0000001a` (shadow-md), both themes |
| placement | centered on the **trigger element** (not the pointer), 6 px below its bottom edge; above it (6 px) when it would pass `window height − 8`; clamped 8 px inside the window |
| timing | appears 350 ms after the pointer enters the trigger (0 ms if another tooltip hid < 400 ms ago); fades in 100 ms; hides on pointer leave, mouse down, scroll |

Placements measured in the references:

| state | trigger (rect) | text | tooltip rect |
|---|---|---|---|
| 04 | Add wrapper `793.48,632.59 304.12x28` | Pick an agent for changelog | `857.53,666.59 175.84x26.2` (below; hangs over the dialog's bottom edge onto the wash) |
| 06 | row code-review `74.4,292.57 626.59x52.39` | Already here | `343.19,350.96 89.42x26.2` |
| 11 | Windsurf pill `761.98,189.9 26x24` (second pill line) | Windsurf / .windsurf/skills · not used here yet | `669.98,219.9 210.4x42.39` (covers the next card and the list's edge) |

## Copy strings (exact)

| where | string |
|---|---|
| title | `Add skill` · `to` · place chip (`acme-web`, `docs-site`, `Claude Code`) · preset chip (`Default`, agent places only) |
| place chip tooltip | workspace path: `~/code/acme-web`, `~/code/docs-site`; agent folder: `~/.claude/skills` (issue #100: Universal adds "Used by <names>") |
| preset chip tooltip | `Saved to Default` |
| close button | tooltip `Close` (aria "Close") |
| search placeholder | `Search library` |
| folder chips | `All 18`, `Web 6`, `Backend 4`, `Writing 3` (label + count) |
| list row tooltip, already here | `Already here` |
| update glyph tooltip | `Update available` |
| no match | `No skills match` · button `Clear search` |
| no library skills (issue) | `No skills in your library` |
| pane header | `Selected` + count (`3`) |
| pane empty | `Nothing selected` |
| deselect | tooltip `Deselect`; aria `Deselect <slug>` |
| All pill | `All`; tooltip `Includes agents added later` |
| folder pill tooltips (acme-web) | the row name, the path, ` · not used here yet` for a folder the workspace does not use, and Universal's readers: `Universal\n.agents/skills\nCline, Codex, Cursor, Dexto, fx, Kimi Code CLI, Loaf, Pi, Sarvam Code, Warp, Zed` · `Claude Code, Cline, Cursor, fx, Warp\n.claude/skills` · `Windsurf\n.windsurf/skills · not used here yet` |
| folder pill tooltips (docs-site) | as above, each path followed by ` · not used here yet` |
| card warning | `Pick an agent` |
| footer | `Cancel` · `Add` / `Add N` |
| Add disabled tooltip | `Select skills` (nothing picked) · `Pick an agent for <slug>` |

## Z-order (back to front)

1. Window content: sidebar, header, workspace or agent page (unchanged, not compared).
2. Scrim/wash over the whole window, sidebar included (kit z 70). Per issue #100 a click on it
   does nothing (decided: #100 wins over the prototype, which closed the picker there); ×, Esc and Cancel close it.
3. Dialog box and its shadow; inside it the list and the Selected pane.
4. Tooltips (kit z 90), above the dialog, may extend past it onto the wash (state 04).
(Toasts, kit z 85, are not part of the picker; the add's progress belongs to #98.)

## Timings and motion

| what | prototype |
|---|---|
| open (Add skill button only) | scrim fades in 150 ms ease-out; dialog `translateY(-8px)`→0 and opacity 0→1 in 180 ms ease-out |
| close | immediate |
| tooltip | 350 ms delay, 100 ms fade |
| hover fills | buttons, pills 120 ms; checkbox fill 100 ms; rows and chips instant |
| selection, pills, filters | instant redraw; list and card-list scroll positions kept |
| search | filters on every keystroke; focused when the picker opens |

## Flattened colors for opacity (use these instead of alpha on overlapping layers)

| element | opacity | light | dark |
|---|---|---|---|
| Add disabled: fill / label (on pane `#fafafa \| #161616`) | 0.45 | `#939393` / `#fafafa` | `#7d7d7d` / `#161616` |
| checkbox on + disabled: fill / mark (row `#ffffff \| #0f0f0f`) | 0.45 | `#959595` / `#fdfdfd` | `#797979` / `#131313` |
| same on a hovered row (`#f5f5f5 \| #262626`) | 0.45 | `#909090` | `#868686` |
| already-here slug / description | 0.5 | `#848484` / `#b9b9b9` | `#848484` / `#595959` |
| chosen folder chip count | 0.7 | `#b6b6b6` on `#0a0a0a` | `#525252` on `#fafafa` |
| off pill logo (grayscale + 0.5) | 0.5 | Claude Code `#c4c4c4`, `layers` `#b9b9b9` | `#4c4c4c`, `#595959` |
