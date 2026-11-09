# Add skill picker: frozen visual spec (1/6) — states and parity regions

Issue #100, prototype `prototypes/picker.html`, variant **c "Two panes"**. Reference = the prototype in
reference mode (`parity/reference.js`): 1180x728 logical window at 2x, app tokens, light and dark.
All numbers are logical px from the window's top-left unless marked "phys" (x2).

Files of this spec:
1. `picker-1-states.md` — states, scenarios, PNGs, comparison crops and ignores (this file)
2. `picker-2-dialog.md` — scrim, dialog box, title row, search, folder chips
3. `picker-3-list.md` — list card, rows, checkbox, update glyph, "No skills match"
4. `picker-4-selected.md` — Selected pane, cards, pills, warning, footer
5. `picker-5-overlays.md` — tooltips, copy strings, z-order, timings, flattened opacities
6. `picker-6-gpui.md` — GPUI/Kit building blocks and GPUI gaps
Machine-readable: `picker-parity.json` (crops, ignores, wash samples per state), built from raw
`__spec` dumps taken with `parity/selectors/picker.json` (not committed; see the end of this file).

## Approved addition (follow-up answer, issue #100)

**All pill** — the prototype lacked it. Added to `prototypes/picker.html` in the kit's pill style:
`<button class="sb-pill pk-all">All</button>` first in every workspace card's pill row; CSS
`.sb-pill.pk-all { padding: 0 9px }` (otherwise `.sb-pill`: h 24, radius 999, 12/500, dashed when
off, solid fg border on `--sb-bg` when on). Tooltip "Includes agents added later". Behavior (issue
#100 Design): on when a skill is picked, in every workspace (also one with no agents); unticking a
folder the workspace uses turns it off; turning it off keeps the ticked pills; turning it on ticks
every folder the workspace uses; ticking a folder the workspace does not use yet leaves it as is.
Add stores All-on skills as all agents (`WorkspaceTargets::AllAgents`). No other addition.

## Reference-mode decision: search focus

Headless Chrome has no window focus, so the autofocused search never matched `:focus`. The
scenarios copy the `:focus`/`:focus-visible` rules onto `.__ref-focus` (parity/scenarios/picker.js
`focused()`), giving what a focused browser shows: border `ring`, a 3 px `ring`@25% shadow and a
2 px `ring` outline at 1 px offset. A click on a row, chip or pill moves focus off the search (in a
browser and in GPUI, whose dialog root takes focus), so those states show it unfocused.

## States (variant c; render: `parity/render-ref.sh picker c <theme> <scenario> <png>`)

PNGs: `references/picker/<NN>-<state>-<light|dark>.png` (2360x1456). Scenarios: `parity/scenarios/picker.js`.

| NN | state | scenario | from | what it shows | search |
|---|---|---|---|---|---|
| 01 | empty | `empty` | acme-web | picker as opened; 3 already-here rows ticked + greyed; "Nothing selected"; `Add` disabled | focused |
| 02 | selected-3 | `selected-3` | acme-web | a11y-audit, api-docs, changelog picked; cards with All on, Universal + `.claude/skills` on, the other 10 folders dashed, wrapping onto a second line; `Add 3` | — |
| 03 | all-off | `all-off` | acme-web | 02, then api-docs' Universal unticked: All off, `.claude/skills` only | — |
| 04 | no-agent | `no-agent` | acme-web | 02, then changelog's Universal + `.claude/skills` unticked: All off, all dashed, "Pick an agent"; `Add 3` disabled with tooltip "Pick an agent for changelog" | — |
| 05 | no-match | `no-match` | acme-web | search "zzz": chips All 0 / Web 0 / Backend 0 / Writing 0, "No skills match" + Clear search | focused, text |
| 06 | already-here | `already-here` | acme-web | pointer on code-review: row hover + tooltip "Already here" | focused |
| 07 | folder-chip | `folder-chip` | acme-web | Backend chip chosen: 4 rows, k8s-debug update glyph | — |
| 08 | docs-site | `docs-site` | docs-site | 2 picks, nothing ticked, All on, "Pick an agent" x2, `Add 2` disabled | — |
| 09 | docs-site-carry | `docs-site-carry` | docs-site | `.claude/skills` ticked on a11y-audit; api-docs starts with it; `Add 2` | — |
| 10 | agent | `agent` | Claude Code | title chip Claude Code + preset chip Default; slug-only cards; `Add 3` | — |
| 11 | pill-tooltip | `pill-tooltip` | acme-web | 02 + pointer on a11y-audit's Windsurf pill, the last, on the second line: hover fill + 2-line tooltip | — |

Fixture: the kit's `data.js`, with the native fixture's agents and folders in the app's order (18 library skills; acme-web uses `.agents/skills` and `.claude/skills`,
holds code-review, e2e-playwright, frontend-design, storybook, tailwind-helper; docs-site uses none;
Claude Code's active preset Default holds code-review, frontend-design, git-commit, pdf-tools,
release-notes, test-writer). The native check must use the same fixture data.

## Comparison regions (all states)

Dialog box: `56.4, 45.4, 1067.2 x 637.2` (border included). The scrim covers the whole window,
sidebar included, so the sidebar is **not** compared as content; only the overlay's own pixels and
the wash color are.

| region | rect (x,y,w,h) | expected |
|---|---|---|
| C dialog crop | `56.5,45.5,1067,637` | equal to the reference (skips the dialog's partial edge columns) |
| W1 wash, main gutter | `1160,400,16,280` | uniform `#f2f2f2` light / `#080808` dark (`overlay` over page bg) |
| W2 wash, under header | `1150,47,26,6` | same as W1 |
| W3 wash, sidebar | `0,300,6,100` | uniform `#ededed` / `#111111` (`overlay` over sidebar `#fafafa`/`#161616`) |
| S shadow band | `232,683,892,40` | equal to the reference (dialog shadow over a plain page); skip if the native page has content there |

Per-state extras (also in `picker-parity.json`):

| state | extra crop | ignore | why |
|---|---|---|---|
| 01 empty | — | `100.9,100.4,4,20` | text caret before the placeholder (blinks natively; hidden in the reference) |
| 04 no-agent | `852.53,666.59,185.84,36.2` | — | the tooltip and its shadow hang below the dialog |
| 05 no-match | — | `121.7,100.4,4,20` | caret after "zzz" (text ends at x 123.2) |
| 06 already-here | — | `100.9,100.4,4,20`; `687.99,135.4,13,533.2` | caret; Kit scrollbar that fades in while the pointer is over the list |
| others | — | — | — |

Optional ignores, only while a GPUI gap below is unresolved (`picker-6-gpui.md`): each off (dashed)
pill's rect (`optional_ignore.dashed_pill`, dash pattern), and the 12x12 inside each checked
checkbox (`optional_ignore.check_glyph`, Tick02 glyph vs the CSS check).

Command (example, state 04 light):

```
R=docs/design/scoped-pages; $R/parity/pixdiff $R/references/picker/04-no-agent-light.png native.png diff.png \
  --crop 56.5,45.5,1067,637 --tolerance 8 --side side.png        # then the tooltip crop:
$R/parity/pixdiff $R/references/picker/04-no-agent-light.png native.png diff-tip.png --crop 852.53,666.59,185.84,36.2 --tolerance 8
```

Native capture rules: same fixture; window 1180x728 at 2x (sidebar 232); the picker settled (Kit
dialog animation 250 ms done); pointer moved onto the scrim at `1150,700` before every non-tooltip
capture (no row hover, no scrollbar); tooltip states wait for the tooltip delay plus its enter
animation. Reference renders are deterministic (decoded pixels identical across runs).

Regenerate everything: `parity/render-all.sh picker` renders the PNGs. For the dumps, run
`parity/render-ref.sh picker c light <state> <png> --spec parity/selectors/picker.json <dumps>/NN-<state>-light.json`
for each state (dark the same way), then `python3 -I parity/picker-parity.py <dumps> spec/picker-parity.json`.
`python3 -I parity/spec-rows.py <json> [keys]` prints a dump compactly.
