# Library visual spec (issue #99) — 1/7: sources and frozen states

The approved Library prototype, frozen as the pixel reference for #99.
- Prototype: `prototypes/library.html` (paths are relative to `docs/design/scoped-pages/`), variant id `popover` ("A Popover"), filter style `segmented`.
- Scenarios: `parity/scenarios/library.js`.
- Raw computed styles: `page-<state>-<theme>.json` (states 01–09, 16, 18) and `pop-<state>-<theme>.json` (10–15, 17), from `__spec` through `parity/render-ref.sh --spec` with `parity/selectors/library-page.json` or `library-pop.json`. They are not committed; render them again when needed.

Files in this spec:
1. States (this file).
2. Comparison crops and masks.
3. Page frame, toolbar, empty state.
4. Rows, placement area, grid.
5. Popover.
6. GPUI/Kit mapping and gaps.
7. Conflicts, settled.

## Conventions

- Units are logical px in the 1180×728 window, origin at the window's top-left. Reference PNGs are 2360×1456 (2x).
- Colors are written light/dark as `#rrggbb[aa]`. "fg" is #0a0a0a/#fafafa and "muted-fg" is #737373/#a3a3a3. The other tokens are in 3/7.
- The 232 px sidebar is the prototype's mock and is never compared.
- Rendering: `parity/render-ref.sh library popover <light|dark> <scenario> <out.png> --filters segmented`. Two renders of the same state are byte-identical (checked on 10 and 14).
- Data is the kit's `data.js` INITIAL, inlined in the bundle: Default active, 18 skills, 3 workspaces. Its agents and directories are the native fixture's, in the app's order (decided 2026-11-09): your agents Universal, Claude Code, Cursor, Windsurf; Universal's members are the registry's `~/.agents/skills` agents; a workspace with no agent offers the 12 directories native offers (5/7). Native: `parity/fixture/restore.sh $FX` (clean seed), then the steps below.
- Pointer:
  - Hover and popover states keep the pointer where the table says.
  - Every other state has no pointer. On native, park the pointer in the sidebar (`pointer_move {x:120,y:600}`) before the capture.

## Approved additions

None.
- Every state in the list exists in `library.html` unchanged.
- The one follow-up answer that concerns the Library is "All agents switch off keeps current agents". The prototype already does this: the switch's off click calls `D.setAgents(slug, holds)`, so the explicit list keeps every agent held now. State 15 freezes it.
- The other follow-up answers concern the picker and the workspace and agent pages.

## States

Each state was rendered in light and dark at `references/library/<NN>-<state>-<theme>.png`. The scenario name equals `<state>`.

| NN | State | Shows | Pointer at the capture | Native steps (after restore) |
|---|---|---|---|---|
| 01 | `all` | All skills tab; All 18 · Active 13 · Inactive 5; Updates 3 off; list; rows a11y-audit…k8s-debug (11th row clipped by the window) | none | open Library |
| 02 | `active` | Active segment on; 13 rows from api-docs; Updates count 2 | none | click "Active" |
| 03 | `inactive` | Inactive on; 5 dimmed rows: a11y-audit, copy-editor, data-viz, k8s-debug (update glyph), prompt-tuning; Updates 1 | none | click "Inactive" |
| 04 | `updates` | Updates chip on (filled); All 3 · Active 2 · Inactive 1; k8s-debug (dim), pdf-tools, storybook | none | click the Updates chip |
| 05 | `folder-search` | Backend tab + search "debug": docker-compose (api-server chip) and k8s-debug (dim, update glyph); All 2 · Active 1 · Inactive 1 · Updates 1; folder tags hidden | none (the field is not focused, see 2/7) | click tab Backend, type `debug` |
| 06 | `no-match` | Search "zzz": empty state "No skills match" + "Clear filters"; all counts 0 | none | type `zzz` |
| 07 | `hover-active` | code-review row hover fill | (605.2, 305.8), the row body | move the pointer there |
| 08 | `hover-inactive` | a11y-audit row hover fill + dashed "+" circle visible | (687, 128.5) | move the pointer there |
| 09 | `hover-place` | pointer on code-review's placement button: row hover, and the button's highlight fill over it (4/7) | (1054.2, 305.8) | move there; take the shot before the 350 ms tooltip |
| 10 | `pop-agents-2ws` | code-review popover: All agents on, 4 pills on; acme-web both pills on, api-server on, docs-site "No agents yet" + 12 off pills on 3 lines; the row shows a "2" count chip | (1080.8, 305.8), on the trigger | delta: api-server holds code-review in `.claude/skills` (popover › api-server pill › Esc); then click the trigger |
| 11 | `pop-inactive` | a11y-audit popover (first row): switch off, every pill off; the trigger is the dashed circle with the open fill | (1136, 128.55) | click a11y-audit's trigger (hover the row first so it shows) |
| 12 | `pop-docs-site` | git-commit popover opened above the row; pointer on docs-site's `.claude/skills` pill, the second (hover fill) | (1034, 563.4) | click git-commit's trigger at (1119.5, 660.4), then move to the pill |
| 13 | `pop-docs-site-on` | as 12, then that pill clicked: docs-site uses `.claude/skills` and holds git-commit; its line loses "No agents yet" and its 12 pills, and the popover shrinks from 285 to 233 high, still anchored above; the row gains a docs-site chip | none | as 12, click the pill, park the pointer. Restore the fixture afterwards |
| 14 | `pop-bottom` | list scrolled to the end (card bottom at y 712); test-writer popover above the last row | (1119.5, 681.6) | scroll the list to the end, click test-writer's trigger |
| 15 | `all-agents-off` | code-review popover after switching All agents off: switch off, all 4 agent pills stay on (explicit list) | (943.7, 378.8), on the switch | click code-review's trigger, then the switch |
| 16 | `grid` | grid view, 4 columns, tiles a11y-audit…test-writer (5th row clipped) | none | click Grid |
| 17 | `grid-pop` | grid; code-review tile popover aligned to the placement's leading edge, clamped 8 px from the window's right edge | (1050.3, 192.6) | Grid, click code-review's tile trigger |
| 18 | `grid-hover-inactive` | grid; a11y-audit tile hover fill + dashed "+" in its footer | (358.75, 152.1) | Grid, move the pointer there |

The task's "preset switch handoff" state belongs to the progress area and was skipped.

## States not frozen (the prototype has no data for them)

- A workspace whose folder is missing (line at 0.4 opacity).
- The unsettled-copy warning glyph before the placement area.
- `+N` avatar overflow: a skill is held in at most 4 of your agents' directories.
- Narrow (<440 px) count chips.
- Tooltips: the kit tooltip is a 12 px mock (see 7/7).
- The edited-copy confirm.

Their look is given in 4/7 and 5/7 from the CSS, without a reference PNG.
