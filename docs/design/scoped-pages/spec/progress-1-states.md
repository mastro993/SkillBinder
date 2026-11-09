# Progress strip: frozen visual spec, part 1 of 5 (states, references, conflicts)

Area: the blocking progress for a change that writes many copies. Issue #98 implements it (#100 adds `Adding…`).
The user picked variant D, "Inline", id `inline`, in `prototypes/progress.html`.
Pixel parity covers only the redesigned surfaces:

- the wash over the window;
- the progress pill, while running and when done;
- the end toast.

The Library page behind the wash belongs to #99. The Review list is today's Skill copies dialog (see C5).

Paths below are relative to `docs/design/scoped-pages/`.

| File | What it is |
|---|---|
| `prototypes/progress.html` | the self-contained prototype |
| `parity/scenarios/progress.js` | scenarios |
| `references/progress/NN-<state>-<theme>.png` | references: 2360x1456 = 1180x728 logical at 2x |
| `parity/selectors/progress.json` | selectors of the raw `__spec` dumps the tables come from (`render-ref.sh --spec`; not committed) |
| `parity/progress-checks.py` | wash-formula and shadow-halo checks (part 4); `parity/spec-rows.py` prints a dump as rows |

Render any state again with `parity/render-ref.sh progress inline <light|dark> <scenario> <out.png>`. Two renders of the same state are byte-identical: each spec run produced the same PNG as the reference.

## States

Fixture: Default → Frontend is 20 pairs (13 adds, 7 removals). At Normal speed (3 s) each pair takes 150 ms. "Clock" is the page's manual clock when the shot is taken. Before every shot the pointer is removed, so nothing is hovered and no tooltip shows.

| NN | Scenario | Clock | Shows |
|---|---|---|---|
| 00 | `default` | 0 | Library before any action. It is the base for the wash formula; not a parity target. |
| 01 | `appear-10` | 300 ms | First frame after the reveal: wash, then the pill `Switching…` `10%` (2 of 20) |
| 02 | `strip-45` | 1350 ms | `Switching…` `45%` (9 of 20). The sidebar already shows Frontend as active (optimistic). |
| 03 | `adding-50` | 1500 ms | Action "Add 4 skills" (Claude Code + Universal, 8 pairs): `Adding…` `50%` |
| 04 | `importing-50` | 1500 ms | Action "Import 6" (6 pairs): `Importing…` `50%` |
| 05 | `finishing-100` | 600 ms | Speed Quick (0.6 s): work done at 600 ms, the pill holds `100%` with a full bar until 800 ms (reveal + 500) |
| 06 | `done-2` | 3000 ms | Problems = 2: `Done · 2 need attention` + Review + Close. The wash stays; the Library behind has redrawn. |
| 07 | `review` | 3000 ms | Review clicked: the pill and the wash are gone, and the review list `2 need attention` opens over the scrim |
| 08 | `toast-switched` | 3000 ms | No problems: the strip closed by itself; the toast `Switched to Frontend` sits bottom center |
| 09 | `deleting-45` | 1350 ms | Delete Frontend (it is on) → Delete in the confirmation → `Switching…` `45%`. Frontend is already gone from the sidebar. |
| 10 | `toast-deleted` | 3000 ms | The delete ends cleanly: toast `Deleted Frontend · Switched to Default` |

Every state has a light PNG and a dark PNG, `references/progress/NN-<scenario>-{light,dark}.png`: 22 files including the two 00 files (00 is `idle`, the `default` scenario). Every PNG was checked by eye. Each shows exactly the intended state, and nothing is clipped. In every state, the content under the wash is the Library page.

## Addition to the prototype

- **Speed "Quick 0.6s"**, a demo control only. It is hidden in reference mode and changes no surface.
  - Why it is needed: no existing speed ends inside the 500 ms minimum. Fast (0.2 s) ends before the reveal, and Normal (3 s) ends long after it. The prototype therefore never held its `100%` frame, which the states list and #98 ("holds at 100%") require.
  - The change is one entry in the Speed segment, `{id:'600', label:'Quick', count:'0.6s'}`.
  - The rebuilt bundle loads with a clean console. No blur or backdrop remains.
- The follow-up answers (picker All pill, list-only pages, Library All agents, parity scope) require nothing more in this area, so there is no other approved addition.

## Conflicts between the prototype (frozen here) and issue #98: settle before implementing

| # | Item | Prototype (what the refs show) | Issue #98 text |
|---|---|---|---|
| C1 | Wash | page `background` at **70%** over the whole window (sidebar included), no blur | "`background` at 55% opacity" |
| C2 | Spinner glyph | `loading` = `Loading03Icon` (retained but not mapped in `assets.rs`), 14 px, foreground | `refresh-03` as `sidebar_row::spinner` builds it (muted, 12 px) |
| C3 | Progress line | 2 px bar in `primary`, **no track**, clipped by the pill's rounded shape | Kit `Progress` xsmall (4 px, with a track at 20%) |
| C4 | End toast | inverted chip at bottom center (fg fill, bg text), `check-circle` 15 px in the text color, 36 px high, radius 8, 4 s | Kit `Notification` with the `check` glyph in `success` (top-right card, 382 wide, border, 14 px text, 5 s) |
| C5 | Review | the kit `reviewDialog`: title `2 need attention`, md 420 wide, rows in a card with Replace/Keep or Remove/Keep, footer Done | "opens the Skill copies dialog… each copy keeps today's row" (title `Skill copies`, Kit Dialog 448 wide) |
| C6 | Percentage | `Math.round(done/total*100)` | "rounded down" (floor). They differ at 1/8 (13 vs 12) and 1/6 (17 vs 16). The refs use exact values (10, 45, 50, 100). |
| C7 | Running tooltip | the pill has `data-tip` = "<plan title>\n<current step>", for example "Switching to Frontend\nCopying tailwind-helper → Cursor" | "No tooltip" |
| C8 | Import toast | `Imported 6 skills` | "An import shows no toast" (the result banner reports it) |
| C9 | Pill button text | 12 px, weight 500 | (unspecified). A Kit small `Button` label is 14 px, weight 400. |

The pixel references follow the prototype. When a conflict is settled in favour of the issue, its reference must be re-rendered after the prototype changes.

Other parts: 2 = wash and pill, 3 = toast and review list, 4 = parity crops and masks, 5 = GPUI mapping and gaps.
