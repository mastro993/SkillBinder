# Plugins

ADRs: [0034](../adr/0034-plugins-are-installed-through-the-agents-own-installers.md), [0035](../adr/0035-plugin-skills-stay-outside-the-library.md); builds on [0006](../adr/0006-activation-writes-managed-copies.md) (managed copies), [0012](../adr/0012-chosen-agents.md) (chosen agents), [0023](../adr/0023-command-line-tool-covers-domain-actions.md) (the command covers domain actions), and [0026](../adr/0026-connect-agents-through-their-own-installers.md) (agents' own installers). Design: [docs/design/plugins](../design/plugins/README.md), rules in [decisions.md](../design/plugins/decisions.md). Glossary: [domain-plugins.md](../agents/domain-plugins.md) (plugin, source, marketplace, plugin agent, installed, can't use, Off, behind, plugin skill).

This spec has three companion files: [Agents](plugins/agents.md) (the plugin systems of all 79 agents and which slice adds each), [Design](plugins/design.md) (technical decisions, architecture, risks), and [Acceptance and slices](plugins/acceptance.md).

## Problem

The owner's Claude Code has 13 plugins (`fstack`, `mattpocock-skills`, `caveman`, `beautiful-rust`, …) and Cursor has more; together they ship dozens of skills. SkillBinder reads no plugin folder (`crates/engine/src/scan/registry.rs:245-295`), so none of this shows: not which plugins each agent has, not the skills they load, not that the library's `caveman` and `rust-best-practices` duplicate skills a plugin also ships, which makes Claude Code load both and leaves the plugin's copy loading after the library skill is turned off. Putting a plugin into another agent means finding its source and running that agent's own commands by hand. 41 of the 79 agents SkillBinder supports now have plugins that can ship skills (Pi calls them packages, Gemini CLI extensions, Kiro powers), and about a dozen read Claude Code's plugin format directly ([Agents](plugins/agents.md)), so the plugin would often work there; only the steps are missing.

## Outcome

From one sidebar page the user sees every plugin their agents have installed, which agents have each and at what version, and its README and skills. They install a plugin into another of their agents, update a behind one, or uninstall one, in a click, and Library tells them which library skills a plugin also ships. We know it works when the owner installs `fstack` from Claude Code into GitHub Copilot from SkillBinder, then runs `/fstack:feature-design` in Copilot CLI, and Library marks `caveman` "Also in plugin caveman".

## Scope

In:

- A Plugins page (sidebar row under Library), a plugin page, and a read-only plugin skill page.
- Reading user-scope plugins, and installing, updating, and uninstalling them through the agent's own installer, for the 24 agents with an installer SkillBinder can run ([Agents](plugins/agents.md#first-version-agents-with-an-installer-skillbinder-can-run-p2)), added a few per slice; adding a plugin's marketplace first after a confirmation.
- The "Also in plugin" mark in Library.
- The command line tool and MCP server's plugin commands.

Out:

- Browsing marketplaces for plugins no agent has, and adding a marketplace on its own (Explore stays skills only).
- Project-scoped plugins, listed or installed (P4).
- Turning plugins on or off (P7), configuring a plugin's options, hooks, or MCP servers.
- Turning plugin skills into library skills, and any write SkillBinder makes itself into plugin folders or agents' settings (ADR 0034, 0035).
- Agents whose plugins install only from their own app or a slash command, such as Cursor, Junie, and Kimi Code (later, P2); plugins that are code (Amp, AiderDesk) or can't hold skills (OpenCode, Kilo Code, Cline, Zed) (P10).
- Converting a plugin between formats: an agent that can't use a plugin's format is told so (P9).

## Behavior

Decisions, from [decisions.md](../design/plugins/decisions.md), the owner's from 2026-11-25:

1. **P1:** installing a plugin for another agent runs that agent's own plugin installer; SkillBinder writes no plugin file or agent setting.
2. **P2:** plugins are read for every chosen agent with an installer SkillBinder can run, added a few per slice ([Agents](plugins/agents.md)); Install, Update, and Uninstall show where that installer offers them. App-only agents come later.
3. **P3:** Library lists library skills only. A library skill whose slug a plugin skill shares shows "Also in plugin <name>", which opens the plugin.
4. **P4:** user scope only; project-scoped installs are never listed.
5. **P5:** always "plugin", with the agent's own word (package, extension, power) in a tooltip; installed, Install, Uninstall, Update; where a plugin comes from is its source: a marketplace, a Git repository, or an npm package.
6. **P6:** Install into an agent that lacks the plugin's marketplace first confirms "Add marketplace <name> (<repository>) to <agent> and install <plugin>?", showing both commands; Install runs both.
7. **P7:** a plugin an agent turned off shows "Installed · <version> · Off" there, read-only; Uninstall stays offered.
8. **P8:** each agent row shows its version; a behind agent gets Update, which runs its own update.
9. **P9:** an agent that can't use the plugin's format shows "Not installed · <agent> can't use this plugin", with no Install.
10. **P10:** plugins that are code registering skills when they run (Amp, AiderDesk), and plugin systems that can't hold skills, are left out.
11. **U1:** a fixed Plugins row under Library, counting plugins, opens the Plugins list (variant A).
12. **U2:** the plugin page is one scroll: header, Agents card, Skills, README (variant C).
13. **U3:** a plugin skill opens its own page (variant A).

### Plugins list

1. Plugins appears in the sidebar below Library with the number of plugins across your plugin agents. It always shows, also with no plugin agent chosen.
2. The page lists each plugin once, matched across agents by name and source (A5), sorted by name: puzzle glyph, name, source (marketplace name, repository, npm package, or "Local"), one-line description from its manifest, skill count, the logos of the agents that have it installed, and a chevron. The search field filters by plugin name.
3. No plugins shows "No plugins". While the first read runs, the list shows placeholder rows. An agent whose plugin records can't be read adds one row at the top: "Couldn't read Claude Code's plugins", with the reason in its tooltip and the cause in the log.
4. A row opens the plugin page.

### Plugin page

1. Header: name, description, source with its repository link (opens in the browser), and the highest installed version.
2. **Agents** lists each chosen plugin agent, alphabetically, with its logo and status:
   - **Installed · 0.9.0**, with Uninstall; plus Update when behind (P8); plus "· Off" when turned off (P7).
   - **Not installed**, with Install; disabled with the tooltip "No source to install from" when the plugin is local (A2).
   - **Not installed · Gemini CLI can't use this plugin**, with no button, when the agent's installer doesn't take the plugin's format (P9). Which formats each agent takes is in [Agents](plugins/agents.md).
   - An agent whose installer lacks update or uninstall shows only the buttons it has (A8).
3. Install: when the plugin comes from a marketplace the agent lacks, P6's dialog shows first; otherwise it runs at once, from the marketplace or straight from the plugin's repository or npm package, whichever that agent takes (`pi install git:github.com/mastro993/fstack`). While a command runs, that row's button shows a spinner and the row's other buttons are disabled; other agents' rows stay usable. Success re-reads the plugins and the row shows its new state; failure shows the agent's last error line under the row in danger text, with the full output in the log (A3). A row's error clears on its next command or when the page is left.
4. Uninstall confirms "Uninstall fstack from Claude Code?" with Uninstall as a danger button (A1). Update runs without confirmation.
5. **Skills** lists the plugin's skills by name with their one-line description; a row opens the plugin skill page. A plugin with no skills shows "No skills" (its README says what else it holds).
6. **README** renders the plugin's `README.md` like a skill's SKILL.md: local links and remote images are not followed. With none, the section is left out.
7. A plugin uninstalled from its last agent stays on screen until the user leaves the page, then drops out of the list (A9).

### Plugin skill page

1. The crumb reads Plugins › fstack › feature-design. The header shows the skill name, its description, and a "From plugin fstack" chip that opens the plugin page; "Also in Library" appears when the library holds the slug, and opens Library with that skill selected.
2. The front matter table and SKILL.md render as on a library skill's page; there are no activation, preset, or workspace controls.
3. When the plugin is gone at the next read, the page falls back to the Plugins list.

### Library

1. A library skill row whose slug a plugin skill of one of your agents shares shows a chip with the puzzle glyph and the plugin's name; its tooltip reads "Also in plugin caveman"; clicking it opens the plugin page. Two plugins sharing the slug show the first by name, and the tooltip names both.
2. Nothing else in Library changes; counts, filters, and activation ignore plugins.

### Command line and MCP server

1. `skillbinder plugins` lists plugins with their agents and versions; `skillbinder plugins <plugin>` shows one, with its skills. `<plugin>` is `name`, or `name@marketplace` when the name is ambiguous.
2. `skillbinder plugin install|update|uninstall <plugin> --agent <agent>` runs the change. Install into an agent lacking the marketplace refuses with the marketplace and repository unless `--add-marketplace` is given, the command's form of P6's confirmation.
3. The MCP server gains `list_plugins`, `get_plugin`, `install_plugin` (with `add_marketplace`), `update_plugin`, and `uninstall_plugin`, with the same rules.

### Lifecycle

1. Plugins are read on launch, when the window regains focus, and after each plugin command, alongside the scan; nothing about plugins is stored or synced.
2. Choosing or dropping an agent re-reads the plugins; a dropped agent's plugins disappear from every list, its files untouched.
3. Installs made outside SkillBinder show at the next focus.

## UX

Prototypes in [docs/design/plugins](../design/plugins/README.md), picked 2026-11-25: list A "Nav row → list", page C "Agents first, stacked", skill view A "Own page", and the Library mark. Rejected: list B (agent table) and C (sidebar section), page A (tabs) and B (side column), skill view B (dialog).

## Assumptions

- **A1 (P1):** Uninstall confirms, naming plugin and agent; there is no Undo. *If wrong:* drop the dialog; an Undo would have to reinstall, which can fetch a newer version.
- **A2 (P2):** a plugin with no source (local) can't be installed elsewhere. *If wrong:* installing from a folder needs each agent's local-install form, which not all have.
- **A3 (P1):** progress is a spinner on the row; errors show the agent's last error line inline. *If wrong:* a toast instead, which loses the error when it times out.
- **A4 (P3):** a plugin skill page has no Import or "Add to library". *If wrong:* that needs a source for the skill, which plugins don't record per skill.
- **A5 (P2):** the same plugin in several agents is one plugin when name and source repository match, or, where an agent records no repository, name and marketplace name. *If wrong:* two marketplaces sharing a name would merge.
- **A6 (P2):** only chosen agents' plugins are read; detected, unchosen agents never show. *If wrong:* the page would need an "other agents" group.
- **A7 (U2):** the plugin page shows name, description, marketplace and repository, version, Agents, Skills, README.
- **A8 (P2):** an agent whose installer lacks update or uninstall (Antigravity CLI and Reasonix have no update) shows only the buttons it has. *If wrong:* add a hint naming the agent's own way.
- **A14 (P2):** an agent row ships only after its slice checks the agent's `--help` and a real install; one that can't be confirmed moves to Later with the reason. Tabnine CLI (maintenance until 2026-12-31) is left out.
- **A9 (U2):** a plugin uninstalled from its last agent stays until the page is left.
- **A10 (P8):** behind compares installed versions only, as numbers (semver); versions that aren't numbers, such as Git commit ids, never show Update, and SkillBinder never asks a marketplace for its newest version. *If wrong:* reading the agent's marketplace clone would add a source of truth per agent.
- **A11 (U1):** the sidebar row uses Hugeicons' puzzle glyph, which presets can also pick. *If wrong:* choose another retained glyph.
- **A12 (ADR 0023):** the command line tool and MCP server cover plugins, with `--add-marketplace` as the confirmation.
- **A13 (P1):** no plugin command runs without the user's click, the command, or an MCP call: SkillBinder never installs, updates, or uninstalls on its own, not even to resolve an overlap.

## Acceptance

See [Acceptance and slices](plugins/acceptance.md).

## Slices

See [Acceptance and slices](plugins/acceptance.md#slices). In order: S0 docs, S1 Claude Code plugins listed, S2 Library mark, S3 Claude Code uninstall and update, S4 GitHub Copilot and Codex (the first installs into another agent), S5 Droid, Grok, CodeBuddy, Qwen Code, S6 Deep Agents, OpenClaw, Qoder, Kode, Augment, S7 Pi, Gemini CLI, Antigravity CLI, S8 Cortex, Devin, Hermes, Goose, Kiro CLI, Reasonix, Autohand, AstrBot, S9 command line and MCP server.

## Open questions

None blocking. Paths and commands marked "to confirm" in [Agents](plugins/agents.md) are confirmed at the start of the slice that adds that agent (A14).
