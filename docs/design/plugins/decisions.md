# Plugins: decisions

Agreed with the owner on 2026-11-25 (feature-design rounds 1–2, and round 4 after researching all 79 agents). P = owner's decision, A = assumption (overrulable).

## Behavior

- P1. Installing a plugin for another agent runs that agent's own plugin installer (`<agent> plugin install name@marketplace`); SkillBinder never writes plugin files or agent settings itself (ADR 0006, 0026).
- P2. Agents (revised in round 4): plugins are listed for every **chosen** agent that has an install command SkillBinder can run, added a few per slice, and only those agents' plugins are read. Agents whose plugins install only from their own app or a slash command (Cursor, Junie, Kimi Code, Neovate, …) come later. Install, Update, and Uninstall show only where that agent's installer offers them.
- P3. (Revised 2026-11-27, L1–L5 below.) Library lists library skills only (unchanged). A library skill whose slug a plugin also ships gets an "Also in plugin <name>" mark that opens the plugin.
- P4. User scope only. Project-scoped plugins are not listed.
- P5. Words (revised in round 4): always **plugin**, whatever the agent calls it (package, extension, power); the agent's own word shows in a tooltip. A plugin is **installed** for an agent; buttons **Install** / **Uninstall** / **Update**. Where a plugin comes from is its **source**: a marketplace (`fedemas`), a Git repository, or an npm package. (Library-skill "install" is unchanged; different context.)
- P6. Install when the agent does not know the plugin's marketplace yet: a confirm dialog "Add marketplace <name> (<repo>) to <agent> and install <plugin>?" showing both commands; one button runs both.
- P7. A plugin installed but turned off in an agent shows "Installed · Off" on that agent, read-only; Uninstall still offered. No turn on/off in SkillBinder.
- P8. Each agent shows its installed version. When an agent's version is behind the newest version another agent has, that agent gets **Update** (runs `<agent> plugin update`).
- P9. An agent that can't use a plugin's format (Gemini CLI needs `gemini-extension.json`; fstack is a Claude Code plugin) shows "Not installed · <agent> can't use this plugin" with no Install button.
- P10. Plugins that are code registering skills at run time (Amp, AiderDesk) are left out, as are plugin systems that can't hold skills (OpenCode, Kilo, Cline, Zed).
- P11. Every Install confirms (owner, after review of PR #249): "Install <plugin> in <agent>?" naming the source, what the plugin adds (skills, hooks, MCP servers), and the commands; it absorbs P6's marketplace step and stands in for the agent's own trust prompt (`--trust`, `--yes`), which SkillBinder skips. Update and Uninstall don't use it.
- A1. Uninstall asks for confirmation naming the agent ("Uninstall fstack from Claude Code?"). No Undo.
- A2. A plugin with no source (synced from claude.ai, loaded from a local folder) can't be installed elsewhere: Install disabled with tooltip "No source to install from".
- A3. While a command runs, that agent's button shows a spinner; failure shows the agent's error output in an error toast/inline message; success refreshes the row.
- A4. A plugin skill opens a read-only skill view: SKILL.md rendered, its plugin, no activation controls. If the library has the same slug, it links to the library skill ("Also in Library").
- A5. Plugins page is sorted by name; search filters by plugin name. Plugins installed in several agents appear once (matched by name and source repository when known, else by name and marketplace).
- A6. Empty: "No plugins" (none of your agents has one). Agent not chosen → not shown anywhere.
- A7. The plugin page shows: name, description, source + repository link, version, README (rendered markdown), Skills list (name + description, click → skill view), Agents list (one row per chosen plugin-capable agent).

## Sample data (realistic, from the owner's machine)

Agents chosen: Claude Code, Codex, GitHub Copilot, Cursor, Droid. (Prototype sample from round 3: since round 4 Codex has Install and Cursor is not listed in the first version.)
Plugins (marketplace · versions · skills):
- fstack @ fedemas (github.com/mastro993/fstack) — Claude Code 0.9.0, GitHub Copilot 0.8.0 (behind → Update). 10 skills: feature-design, ux-brainstorm, domain-driven-design, design-system, create-issues, comment-sweep, pr-babysit, pr-ship, resolve-merge-conflicts, test-driven-development.
- mattpocock-skills @ claude-plugins-official — Claude Code 1.3.1. 12 skills (tdd, grilling, prototype, research, domain-modeling, codebase-design, code-review, pr, wizard, writing-for-agents, diagnosing-bugs, …).
- caveman @ caveman — Claude Code 600e8ef. 5 skills: caveman, caveman-commit, caveman-review, caveman-help, compress. Overlaps library slugs caveman, caveman-commit, …
- beautiful-rust @ beautiful-rust — Claude Code 1.4.0. 4 skills: rust-best-practices (overlaps library), rust-port, rust-review.
- frontend-design @ claude-plugins-official — Claude Code 1.1.1, Off. 1 skill.
- pr-review-toolkit @ claude-plugins-official — Claude Code 1.0.1. 0 skills (agents and commands only).
- context7 @ claude-plugins-official — Claude Code. 0 skills (MCP server).
- motion @ (local, no marketplace) — Cursor. 2 skills. Install elsewhere disabled (A2).
- a long one: "enterprise-observability-and-incident-response-toolkit" @ acme-internal-marketplace.

## Layout (owner's picks, 2026-11-25, round 3)

- U1. (Revised 2026-11-27: Library's Plugins tab, no sidebar row.) Sidebar: a fixed **Plugins** row under Library, with a count, opens the Plugins list page: one row per plugin with name, marketplace, one-line description, skill count, the logos of the agents that have it, and a chevron (variant A).
- U2. Plugin page: one scroll with header (name, description, marketplace, repository, newest version), then the **Agents** card, then **Skills**, then README (variant C).
- U3. A plugin skill opens its own page, like a library skill's, with the crumb Plugins › plugin › skill, a "From plugin <name>" chip, "Also in Library" when the slug overlaps, the front matter, and SKILL.md; no activation controls (variant A).
- A8. An agent whose installer lacks uninstall or update (Antigravity CLI has no update, Deep Agents none) shows only the buttons it has.
- A9. A plugin whose last agent uninstalled it stays on screen until the user leaves the page, then drops out of the list.

## Plugins in Library (2026-11-27)

Agreed with the owner on 2026-11-27 (feature-design rounds 1–2 and the Library prototype pick); ADR 0041 supersedes ADR 0035.

- L0. A plugin skill is never a library skill: never imported, installed, synced, placed, filed, or put in presets or workspaces. Library lists it beside library skills, marked as its plugin's, so Library is the one place for every skill your agents load. Plugin management moves into Library; no Plugins sidebar row.
- L1. A plugin skill row's trailing area shows the agents that have its plugin, read-only; it and the plugin chip open the plugin page (Install, Update, Uninstall there). No skill-level install toggle.
- L2. A slug in the library and in a plugin shows as two rows, side by side. The library row keeps its controls and adds "Also in <plugin>"; the plugin row has no "Also in Library" mark (owner: redundant). The plugin skill page keeps its "Also in Library" chip (A15).
- L3. All, Active, and Inactive count plugin skills: Active while the plugin is installed and on in at least one of your agents, Inactive when Off everywhere.
- L4. An agent's page lists the plugin skills that agent loads, read-only, in its one list.
- L5. Layout: Library variant A "Mixed + Plugins tab" (`prototypes/library.html`). Rejected: B "Grouped by plugin", C "Plugin tabs".
- A15. The plugin skill page keeps "Also in Library".
- A16. Plugin skills never show under a folder tab and never count as Updates.
- A17. Library's sidebar count includes plugin skills.
