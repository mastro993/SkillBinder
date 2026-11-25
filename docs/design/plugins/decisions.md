# Plugins: decisions

Agreed with the owner on 2026-11-25 (feature-design rounds 1–2). P = owner's decision, A = assumption (overrulable).

## Behavior

- P1. Installing a plugin for another agent runs that agent's own plugin installer (`<agent> plugin install name@marketplace`); SkillBinder never writes plugin files or agent settings itself (ADR 0006, 0026).
- P2. Agents: plugins are listed for every **chosen** agent with a known plugin layout: Claude Code, Codex, GitHub Copilot (CLI), Droid, Antigravity CLI, Cursor. One-click Install/Uninstall for Claude Code, GitHub Copilot, Droid, Antigravity CLI. Codex and Cursor have no install command: their row shows a hint ("Install from Codex's /plugins") instead of a button.
- P3. Library lists library skills only (unchanged). A library skill whose slug a plugin also ships gets an "Also in plugin <name>" mark that opens the plugin.
- P4. User scope only. Project-scoped plugins are not listed.
- P5. Words: a plugin is **installed** for an agent; buttons **Install** / **Uninstall**; a plugin comes from a **marketplace**. (Library-skill "install" is unchanged; different context.)
- P6. Install when the agent does not know the plugin's marketplace yet: a confirm dialog "Add marketplace <name> (<repo>) to <agent> and install <plugin>?" showing both commands; one button runs both.
- P7. A plugin installed but turned off in an agent shows "Installed · Off" on that agent, read-only; Uninstall still offered. No turn on/off in SkillBinder.
- P8. Each agent shows its installed version. When an agent's version is behind the newest version another agent has, that agent gets **Update** (runs `<agent> plugin update`).
- A1. Uninstall asks for confirmation naming the agent ("Uninstall fstack from Claude Code?"). No Undo.
- A2. A plugin with no marketplace (synced from claude.ai, loaded from a skills folder) can't be installed elsewhere: Install disabled with tooltip "No marketplace to install from".
- A3. While a command runs, that agent's button shows a spinner; failure shows the agent's error output in an error toast/inline message; success refreshes the row.
- A4. A plugin skill opens a read-only skill view: SKILL.md rendered, its plugin, no activation controls. If the library has the same slug, it links to the library skill ("Also in Library").
- A5. Plugins page is sorted by name; search filters by plugin name. Plugins installed in several agents appear once (matched by plugin name + marketplace).
- A6. Empty: "No plugins" (none of your agents has one). Agent not chosen → not shown anywhere.
- A7. The plugin page shows: name, description, marketplace + repo link, version, README (rendered markdown), Skills list (name + description, click → skill view), Agents list (one row per chosen plugin-capable agent).

## Sample data (realistic, from the owner's machine)

Agents chosen: Claude Code, Codex, GitHub Copilot, Cursor, Droid.
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

- U1. Sidebar: a fixed **Plugins** row under Library, with a count, opens the Plugins list page: one row per plugin with name, marketplace, one-line description, skill count, the logos of the agents that have it, and a chevron (variant A).
- U2. Plugin page: one scroll with header (name, description, marketplace, repository, newest version), then the **Agents** card, then **Skills**, then README (variant C).
- U3. A plugin skill opens its own page, like a library skill's, with the crumb Plugins › plugin › skill, a "From plugin <name>" chip, "Also in Library" when the slug overlaps, the front matter, and SKILL.md; no activation controls (variant A).
- A8. An agent without an uninstall command (Codex, Cursor) shows status only when the plugin is installed there.
- A9. A plugin whose last agent uninstalled it stays on screen until the user leaves the page, then drops out of the list.
