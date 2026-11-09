# Native fixture for the parity check

This fixture gives a native SkillBinder window the same data as the prototypes: 18 skills with their descriptions, library folders, presets, workspaces, agents and update flags. A native screenshot of a state can then be compared with the reference PNG of that state.

It was first built on 2026-11-09 from `feat/skill-activation-redesign` at `18f5e92`, with `cargo build -p skillbinder --features native-test --locked`.

## Files

`$FX` is a fixture directory of your choice, absolute and outside the repository, for example `$TMPDIR/skillbinder-parity/fixture`. It holds `seed/` (the snapshot, never edited) and `root/` (the live copy the app runs on).

| File | What it does |
|---|---|
| `make-fixture.sh <root> [init\|finish\|problems]` | Builds the files. `init` creates the home, the workspaces, the Git source and the skills locks before the first launch. `finish` adds the 2 skills only Discovery lists, after the UI setup. `problems` makes the optional edited and foreign copies. |
| `restore.sh $FX [problems]` | Replaces `$FX/root` with a fresh copy of `$FX/seed` at the same absolute path. With `problems`, it also applies the edited and foreign copies. It refuses to run while the app is open on that root. |
| `verify.js <prototype.html> <root>` | Checks a root against the prototypes' data: counts, descriptions, folders, preset and workspace holdings, colors and sources. Pass any bundle from `../../prototypes/`; it prints `OK: …` for a good seed. Needs `node` and `sqlite3`. |
| `panel-pick.js` | Accessibility (JXA) helper that selects a folder in the native "Add workspace" panel. Only needed to build the seed. |

## Build the seed (once per fixture directory, about 15 minutes of UI driving)

The seed stores absolute paths, so it works only at the `$FX/root` it was built in.

1. `scripts=docs/design/scoped-pages/parity/fixture; mkdir -p $FX && $scripts/make-fixture.sh $FX/root`
2. Launch the app on it (see "Restore and launch").
3. Onboarding: Continue ×3, Create library, Import. This imports 12 skills into Default for the agents that held them.
4. Library: untick a11y-audit, copy-editor, data-viz, k8s-debug and prompt-tuning. Set the code-review agents menu to "All agents".
5. Presets: add "Frontend" and "Backend". For each, open the right-click menu › Color › Custom, set the hex field (`set_text`) to `#3B82F6` or `#F59E0B`, and press Enter.
6. Add the workspaces acme-web, api-server and docs-site, in that order, from `$FX/root/home/code` (see "Driving tips" for the folder panel).
7. Discovery: Select all (6) › Review import › Import.
8. On the acme-web page, set code-review and e2e-playwright to "All agents". On api-server, set sql-helper and docker-compose to "All agents".
9. Preset holdings:
   1. Activate Frontend. Tick a11y-audit, code-review, frontend-design and tailwind-helper. Set data-viz's menu to Cursor, and storybook's menu to Claude Code, then Cursor.
   2. Activate Backend. Tick code-review and sql-helper. Set api-docs to Universal, docker-compose to Claude Code then Universal, and k8s-debug to Claude Code.
   3. Activate Default.
10. Add the folders Web, Backend and Writing, and file the skills with the row menu › Move to folder as the table below lists. Dismiss the import result.
11. Quit the app, then:
    1. `$scripts/make-fixture.sh $FX/root finish`
    2. `sqlite3 $FX/root/data/skillbinder.db 'pragma wal_checkpoint(TRUNCATE)' && rm -rf $FX/root/data/logs/*`
    3. `rm -rf $FX/seed && cp -Rp $FX/root $FX/seed`
    4. `node $scripts/verify.js docs/design/scoped-pages/prototypes/library.html $FX/seed`

## Restore and launch (every run)

```sh
docs/design/scoped-pages/parity/fixture/restore.sh $FX     # or: … restore.sh $FX problems
cargo build -p skillbinder --features native-test --locked
target/debug/skillbinder --mcp --data-dir $FX/root/data --home $FX/root/home
```

- With `--home`, the engine sees an empty environment (`EngineConfig::isolated`), so `XDG_*` and `CLAUDE_CONFIG_DIR` in your shell never leak in.
- Quit with `pkill -TERM -f "skillbinder --mcp --data-dir $FX/root/data"`, then restore before the next run. Any click that changes something (activation, theme toggle, update) persists in `root/`.
- At launch the app scans, finds 2 skills for Discovery, and checks the sources after the scan. Exactly k8s-debug, pdf-tools and storybook then get the update glyph. Do not click "Update all": it applies the updates. Default is the active preset. The theme follows the system until the sidebar toggle switches it.
- Use the clean seed for the Library, workspace and agent pages. Use `problems` for the picker and progress problem states.

## Window size

The window opens at 1180 × 728 logical points (`apps/desktop/src/main.rs`, `Bounds::centered`). The gpui MCP `screenshot` returns 2360 × 1456 px (2x), the size of every reference PNG.

## Driving tips

- "frame wait timed out" means the window is fully covered: GPUI draws no frame then. Move it to an uncovered spot without activating the app, for example `osascript -e 'tell application "System Events" to set position of window 1 of (first process whose unix id is <pid>) to {x, y}'`, and raise it with `perform action "AXRaise"` before a screenshot.
- `click_element` on a row below the fold reports "clicked" but misses. Scroll the row into view first.
- Park the pointer where each area's spec says before a capture, so no hover fill or tooltip shows.
- The "Add workspace" folder panel is out of reach of the MCP. Accessibility calls work without focus:
  1. Open the panel's "Where:" popup and pick an ancestor folder of `$FX`.
  2. `osascript -l JavaScript panel-pick.js <pid> <folder> … root home code acme-web` expands each named folder in turn and selects the last.
  3. `osascript -e 'tell application "System Events" to click button "Add workspace" of splitter group 1 of window "Open" of (first process whose unix id is <pid>)'`

## Prototype data → native fixture

Element ids `preset-default`, `agents-universal`, `agent-claude-code`, `agent-cursor` and `agent-windsurf` are fixed. Frontend, Backend, the workspaces and the folder tabs have ids with UUIDs made when the seed is built; read them with `get_ui_tree`.

| Prototype data | Native fixture |
|---|---|
| 18 skills (slug, description) | 18 library skills. Each `SKILL.md` has `name: <slug>` and `description: <description>` byte for byte, and the body is `# <slug>` and the description. |
| Folders Web 6, Backend 4, Writing 3, none 5 | Library folders Web, Backend, Writing in that tab order, with the same filing (`make-fixture.sh` lists each skill's folder). |
| Updates: k8s-debug, pdf-tools, storybook | A `file://…/root/sources/skills.git` source whose HEAD changes these three. They learn it at import from the global lock (k8s-debug, pdf-tools) and from acme-web's `skills-lock.json` (storybook). The other 15 have no source. |
| Your agents universal, claude, cursor, windsurf (the app's order); Universal's members Cline, Codex, Dexto, fx, Kimi Code CLI, Loaf, Pi, Sarvam Code, Warp, Zed | Universal, Claude Code, Cursor and Windsurf, detected at `~/.agents/skills`, `~/.claude/skills`, `~/.cursor/skills` and `~/.codeium/windsurf/skills` under `--home`. The registry's agents whose global root is `~/.agents/skills` are Universal's members; a preset that holds Universal stores their 10 ids. |
| Default (no color, on) | code-review for All agents; git-commit for Claude Code and Universal; pdf-tools and release-notes for Claude Code; test-writer and frontend-design for Claude Code and Cursor; changelog for Universal (7). |
| Frontend (blue) | Custom color `#3b82f6`. code-review, frontend-design, tailwind-helper and a11y-audit for all; storybook for Claude Code and Cursor; data-viz for Cursor (6). |
| Backend (amber) | Custom color `#f59e0b`. code-review and sql-helper for all; api-docs for Universal; docker-compose for Claude Code and Universal; k8s-debug for Claude Code (5). |
| acme-web `~/code/acme-web`, uses `.agents/skills` and `.claude/skills` | `$FX/root/home/code/acme-web`, shown as `~/code/acme-web` once home is shortened. code-review and e2e-playwright for all; storybook and tailwind-helper for `.claude/skills`; frontend-design for `.agents/skills` (5). |
| api-server, uses `.claude/skills` | sql-helper and docker-compose for all; api-docs for `.claude/skills` (3). |
| docs-site, uses none; offers the 12 directories of the detected agents | Holds nothing; the folder has only a README (0). Its agents menu offers `.agents/skills` (Universal), `.claude/skills` ("Claude Code, Cline, Cursor, fx, Warp"), `.cline/skills`, `.clinerules/skills`, `.codex/skills`, `.cursor/skills`, `.dexto/skills`, `.fx/skills`, `.kimi-code/skills`, `.pi/skills`, `.warp/skills` and `.windsurf/skills`. |
| Active 13, inactive 5 | Inactive: a11y-audit, copy-editor, data-viz, k8s-debug, prompt-tuning. |
| Sidebar counts | Library 18. Default 7, Frontend 6, Backend 5. acme-web 5, api-server 3, docs-site 0. Universal 3, Claude Code 6, Cursor 3, Windsurf 1. Discovery 2. |
| Discovery 2 | `figma-export` and `react-hooks` in `acme-web/.claude/skills`, not in the library. The prototypes give only the count, so the names are the fixture's. |
| Edited: release-notes in `~/.claude`, storybook in acme-web `.claude`; foreign: tailwind-helper in `~/.cursor` | Only after `restore.sh $FX problems`. |

## Known differences

The prototypes' agents, Universal members, directory names and order used to differ from native. On 2026-11-09 the user decided the prototype data must be the real data, so the kit's `data.js` now matches this fixture: Universal first, then by name; Universal's members are the registry's `~/.agents/skills` agents; `.claude/skills` is named after its detected readers; a workspace without agents offers the 12 directories above. What still differs:

1. **Paths.** The current app prints the full workspace path. The projects live under the fake home, so a page that shortens home to `~` shows exactly `~/code/acme-web`.
2. **Preset colors.** The app's palette has no amber, and its Blue is `#2563eb` / `#60a5fa`. The fixture uses custom colors equal to the prototype tints, which the app draws the same in both themes.
3. **Edited and foreign copies** are opt-in, and the app learns about them lazily: a copy is marked Edited only when the engine tries to rewrite or remove it. The foreign `~/.cursor/skills/tailwind-helper` raises the current app's Cursor count from 3 to 4, and switching to Frontend reports it as a conflict.
4. **Sync.** The library repository has no commits or remote, so the Sync row shows an attention dot. The sidebar is never compared.

None of these changes pixels on the redesigned surfaces.
