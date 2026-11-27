# Plugins: design reference

This folder holds the prototypes for listing and managing the coding agents' plugins in SkillBinder. They follow the decisions agreed with the owner on 2026-11-25 ([decisions.md](decisions.md): P1–P8, A1–A7), and each variant names the reference it draws on ([references.md](references.md)). Build the same layout, copy, states, and behavior with GPUI Kit and the design system's components first; small visual differences from Kit are fine. Where a prototype and decisions.md disagree, decisions.md holds.

## Chosen variants

| Area | Question | Prototype | Variant the user picked (2026-11-25) | Screenshot |
|---|---|---|---|---|
| Library: plugin skills and the Plugins tab | U1, L1–L5 | `prototypes/library.html` | A "Mixed + Plugins tab" (2026-11-27; replaces the 2026-11-25 list A "Nav row → list" and the Library mark) | ![Library's All skills: library and plugin skills in one alphabetical list, plugin rows with a filled plugin chip and read-only agents, the library caveman marked Also in caveman](screenshots/library.png) ![Library's Plugins tab listing plugins with source, description, skill count, and agents](screenshots/library-plugins-tab.png) |
| Agent page | L4 | `prototypes/library.html`, state Agent page | A (one list, as Library) | ![Claude Code's page listing its library skills and the plugin skills it loads, each plugin skill with its chip](screenshots/agent-page.png) |
| Plugin page | U2 | `prototypes/plugin-page.html` | C "Agents first, stacked" | ![The fstack plugin page: header, Agents card with Install, Update, and Uninstall, then Skills](screenshots/plugin-page.png) |
| Plugin skill view | U3 | `prototypes/plugin-skill.html` | A "Own page" | ![The feature-design plugin skill page with a From plugin fstack chip, front matter, and SKILL.md](screenshots/plugin-skill.png) |
| Install dialog | P11 | `prototypes/plugin-page.html`, Install on Droid | (one design) | ![Install fstack in Droid? naming its source, 10 skills, the marketplace step, and both commands](screenshots/install-dialog.png) |

Round 4 (after researching all 79 agents) changed rules the prototypes predate, and decisions.md holds: Codex now has Install and Uninstall rather than a hint, app-only agents such as Cursor are not in the first version, the marketplace line reads as the plugin's source, and an agent that can't use a plugin's format shows "Not installed · <agent> can't use this plugin" (P9), a state the prototypes don't draw.

On 2026-11-27 the owner moved plugins into Library (ADR 0041): `plugins-list.html` and the Library mark in `plugin-skill.html` are superseded by `library.html`, and their crumbs and sidebar row predate it; decisions.md holds. Each bundle opens on the picked variant. The variant bar still switches to the rejected ones: list B "Nav row → table" and C "Sidebar section", page A "Tabs" and B "Body + side column", skill view B "Dialog".

Open a bundle in a browser. Its variant bar switches variants in place, and the segmented control beside it switches states. Clicks work: Install, Update, and Uninstall run against an in-memory store, so a change made on one page shows on every other.

## library.html

Plugin skills in Library and plugins managed from its Plugins tab (ADR 0041, decisions L1–L5).

- **A "Mixed + Plugins tab"** (picked): plugin skills sit among library skills under All skills, alphabetically, each with a filled chip naming its plugin and, in place of the placement area, the agents that have the plugin and a chevron, read-only; the chip and the agents open the plugin. A library skill sharing a slug adds the outline mark "Also in caveman". A Plugins tab after All skills lists the plugins; a plugin opens under Library › Plugins. Like the Installed for you tag in Wix Automations.
- **B "Grouped by plugin"**: library skills first, then one collapsible group per plugin with its source, agents, and Manage. Like the tool groups of Claude's Figma connector.
- **C "Plugin tabs"**: mixed as A, plus a tab per plugin after the folders, opening with the plugin's agents above its skills. Like Wix's Created by you | Installed for you tabs.

States: Library, Agent page (Claude Code's page: its library skills and the plugin skills it loads, in A's one list or B's groups), and Can't read (GitHub Copilot's records unreadable). Off `frontend-design` shows its skill dimmed under Inactive. The URL hash `#tab=plugins` or `#route=plugin&pid=fstack` opens a view directly, for screenshots.

## plugins-list.html

Superseded by `library.html` (2026-11-27).

Where Plugins sits in the sidebar, and how the plugin list reads.

- **A "Nav row → list"**: a Plugins row under Library, counting plugins, opens a list of rows: name, marketplace, one-line description, skill count, the agents that have the plugin as a logo stack, and a chevron. Like Cursor Plugins.
- **B "Nav row → table"**: the same row opens a dense table with one column per chosen plugin-capable agent, each cell showing that agent's version, Off, or —; a version behind the newest shows the update glyph. Like Supabase Database Extensions and Stripe Apps.
- **C "Sidebar section"**: a Plugins section after Agents lists each plugin with its skill count; clicking one opens its page, and there is no list page.

States: Normal (8 plugins), Empty ("No plugins"), Many (17 plugins, including `enterprise-observability-and-incident-response-toolkit`), and Loading (skeleton rows in A and B, a spinner in C's section). Search filters by plugin name (A5). A row opens the plugin's page in its simple stacked layout; in C, Empty and Loading show Library, since there is no plugin to open.

## plugin-page.html

The page of one plugin, `fstack`: header (name, description, marketplace, repository link, newest version), README, Skills, and one Agents row per chosen plugin-capable agent (A7).

- **A "Tabs"**: README | Skills | Agents as tabs under the header. The header's agent stack jumps to Agents. Like Zoho CRM extension and VS Code extension detail.
- **B "Body + side column"**: README then Skills in the main column; Agents (compact rows) and Details (marketplace, repository, version) in a 206-wide side column. Like VS Code's Installation column and Vercel's fact column.
- **C "Agents first, stacked"**: one scroll with the Agents card, then Skills, then README. Like Linear's Slack Connections and Cursor plugin detail. plugins-list.html and plugin-skill.html also use this layout for any plugin page they open.

Agent rows: Claude Code "Installed · 0.9.0" with Uninstall; GitHub Copilot "Installed · 0.8.0" with Update and Uninstall (P8); Droid "Not installed" with Install, which opens P11's "Install fstack in Droid?" naming its source, "10 skills", the marketplace step, and `droid plugin marketplace add mastro993/fstack` and `droid plugin install fstack@fedemas --scope user`; Codex and Cursor show a hint, with no button (P2). Uninstall asks "Uninstall fstack from Claude Code?" (A1). While a command runs, the row's button shows a spinner (A3).

States: Normal, Off (Claude Code "Installed · 0.9.0 · Off", P7), Install fails (Droid's row shows the agent's error output, and Install fails again), and No marketplace (`motion`, loaded in Cursor from a folder: Install is disabled with "No source to install from", A2).

## plugin-skill.html

The read-only view of a plugin skill (A4), and the Library mark (P3).

- **A "Own page"**: a page like a library skill's: name, description, a "From plugin fstack" chip back to the plugin, "Also in Library" when the library has the same slug, the front matter table, and SKILL.md. There are no activation controls.
- **B "Dialog"**: a large dialog over the plugin page, like Explore's skill detail dialog: description and SKILL.md on the left; About (plugin, marketplace, repository, version, Also in Library) and Details (front matter other than name and description) on the right.

States: feature-design (`fstack/feature-design`), Also in Library (`caveman/caveman`, whose slug the library also has), and Library mark (Library, where `caveman`, `caveman-commit`, and `rust-best-practices` show an "Also in plugin <name>" chip that opens the plugin). "Also in Library" opens Library with that row selected.

## Not in the prototypes

- Plugins scoped to a project (P4: never listed).
- Turning a plugin on or off (P7: read-only).
- An agent that is not chosen (A6: never shown).
- Antigravity CLI, which P2 lists as plugin-capable, is not among the sample's chosen agents.
- Codex and Cursor rows when the plugin is installed there show status only, with no Uninstall and no hint; decisions.md does not say how to remove a plugin from an agent without a command.

## Icons

- Plugins use the Hugeicons puzzle glyph (`assets/hugeicons/PuzzleIcon.svg`) and Sync its refresh glyph (`RefreshIcon.svg`), both retained artwork the prototype kit's generated block lacks, so `source/plugins-data.js` adds them with the same normalization as `gen_icons.py`. The puzzle glyph is retained today only as a preset icon choice (`preset-glyphs.json`), so a preset may show the same glyph as Plugins.
- Marketplace uses the package glyph, the repository link the GitHub glyph, and a plugin without a marketplace the folder glyph.
- Droid has no bundled logo and shows the square terminal glyph (AGENTS.md).

## Sources

`source/` holds the editable pages and their shared files. They link the shared kit in `../scoped-pages/source/kit/`.

- `library.html`, `plugins-list.html`, `plugin-page.html`, `plugin-skill.html`: the pages.
- `plugins.css`: shared styles on the kit's tokens.
- `plugins-text.js`: README.md and SKILL.md sample texts.
- `plugins-data.js`: the sample data from decisions.md and an in-memory store (installs, updates, uninstalls, errors).
- `plugins-kit.js`: the sidebar, header, agent rows, skill rows, markdown, and Library rows the pages share.

Rebuild a bundle from the repository root after editing a source:

```sh
python3 -I docs/design/scoped-pages/source/kit/inline.py docs/design/plugins/source/library.html docs/design/plugins/prototypes/library.html
python3 -I docs/design/scoped-pages/source/kit/inline.py docs/design/plugins/source/plugins-list.html docs/design/plugins/prototypes/plugins-list.html
python3 -I docs/design/scoped-pages/source/kit/inline.py docs/design/plugins/source/plugin-page.html docs/design/plugins/prototypes/plugin-page.html
python3 -I docs/design/scoped-pages/source/kit/inline.py docs/design/plugins/source/plugin-skill.html docs/design/plugins/prototypes/plugin-skill.html
```

Keep each source file under about 18 KB; split a growing one into a `<name>.parts/` folder, which the inliner joins in numeric order.
