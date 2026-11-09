# Library visual spec (issue #99) — 7/7: conflicts to settle before implementing

The references show the prototype exactly as approved. Each item below is a place where the prototype disagrees with #99's text, the current app, or the native fixture. Under pixel parity the prototype wins unless the user decides otherwise. When a decision changes the prototype, edit `prototypes/library.html` and re-run `parity/render-all.sh library`: the scenarios need no change.

| # | Conflict | Prototype (refs) | #99 / app / fixture | States affected | Suggested resolution |
|---|---|---|---|---|---|
| C1 | Update glyph color | `.lib-upd` uses info: cyan #06b6d4/#22d3ee | the app paints `upload-circle` in warning #eab308/#facc15; scoped-pages and picker use warning | every state that shows k8s-debug, pdf-tools or storybook (01–05, 07–18) | switch the prototype to `--sb-warning` and re-render; keep the app color |
| C2 | Placement hover/open fill | `--sb-muted`, which equals the row hover fill (#f5f5f5/#262626), so it is invisible on a hovered row | #99: the menu highlight fg@0.08 over the row fill | 09, 10, 11, 12, 13, 14, 15, 17 | take #99: the prototype uses `--sb-highlight` (#0a0a0a14/#fafafa14) and re-renders |
| C3 | Logos in pills and avatars | pill on: the logo in its own colors (Claude Code orange); off: grayscale at .5. Row avatars: color logos on white | #99: monochrome logo, fg when on and muted when off | 10–15, 17 | keep the prototype (it was approved) and fix #99's text |
| C4 | Agent order and fixture targets | targets claude, cursor, universal, windsurf; workspace directories claude, agents, windsurf; Universal = Codex, Amp, OpenCode, Gemini CLI, GitHub Copilot | native and #99: Universal first, then Claude Code, Cursor, Windsurf. Native `.claude/skills` reads "Claude Code, Cline, Cursor, fx, Warp"; docs-site offers about 12 directories; native Universal = Cline, Codex, Dexto, fx, Kimi Code CLI, Loaf, Pi, Sarvam Code, Warp, Zed (`parity/fixture/README.md`, "Known differences" 1–3) | every state with a stack or a popover (01, 02, 04, 05, 07, 09–18) | blocking for parity. Either reorder the kit's `data.js` targets (inlined in all four bundles: all four areas re-render), or adapt the fixture. Also cut the native detected agents so docs-site offers the same 3 directories, or widen `wsDirs` |
| C5 | Avatar metrics | 18 px, ring 2 px outside, overlap 5, logo 14.04 (color) / 12.6 (mono) | #99 and the app: 20 px, overlap 6, ring inside the box | rows in every list and grid state | prototype; fix #99's text |
| C6 | Empty-state copy | one copy, "No skills match" + "Clear filters", for a search, a segment or Updates; "No skills in <folder>" | #99: "No skills match these filters" + Clear filters (filters); "No skills match '<query>'" (search only); "No skills in <folder>" | 06 | decide. If #99 wins, change `drawList` and re-render 06 |
| C7 | Accessible names | "Change where it is on", "Turn on" | "Change where <slug> is on", "Turn on <slug>" | none (no pixels) | take #99 |
| C8 | Tooltip text size | kit tooltip 12 px | app Kit `Tooltip` 14 px (`GC/tooltip.rs`) | none frozen | use the app's Tooltip; the kit tooltip is a mock |
| C9 | Page and row geometry | gutter 16, top 14, gap 12; toolbar 28 high; flexible search (338.63); List/Grid 72×28; rows 59.09 with the folder as a tag; grid 4 × 221.5, radius 10, padding 12, gap 10. Header mock: gutter 16, 28 px icon buttons | app: gutter 24, top 16, gap 16; Input 32; search 260 fixed (#99: "keeps 260, may shrink to 160"); List/Grid 70×32; rows 90.5 with a folder line; grid 3 × ~290, radius 12, padding 16 | all | parity means the Library adopts the prototype frame. Align the header gutter to 16 too, so the tabs start at x 248, or keep the header masked (2/7) |
| C10 | Updates chip | outlined pill chip 28 high; on = solid fg fill with the text in the page color | #99: a small ghost button with the chosen fill of a folder tab (muted) | 01–06 | prototype; fix #99's text |
| C11 | Workspace pill wrapping | `.lib-wspills` never wraps (flex none) | #99: pills wrap onto further lines, aligned to the trailing edge | none frozen (3 pills at most), but C4's native fixture would overflow | decide with C4 |
| C12 | Count weight | segment counts inherit weight 500 | #99: "muted 11-point tabular figures" | 01–06 | prototype (500) |
| C13 | Kit components named in #99 | the segments, List/Grid and switch need 28/24 px and 30×18 geometry | Kit TabBar segmented and Switch cannot produce it (6/7 G5, G6) | 01–06, 10–15, 17 | app-owned segmented and switch controls, or accept the documented deltas |

## Not conflicts (verified equal)

- Menu surface #ffffff/#262626.
- Line fg@10%.
- Card surface #ffffff/#0f0f0f.
- Radii: card 10, menu 12.
- Divider inset 14.
- Dot 8.
- Hover #f5f5f5/#262626.
- Switch tokens #d4d4d4/#404040 and #171717/#fafafa.
- Segmented track #f5f5f5/#171717.
- Popover offset 4.
- Window margin 8, which is `theme::menu::WINDOW_MARGIN`.
