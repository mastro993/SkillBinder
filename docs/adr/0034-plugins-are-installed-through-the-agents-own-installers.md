# Plugins are listed from the agents' own records and installed through their own installers

Several agents now install plugins: bundles from a marketplace that hold skills and often commands, subagents, hooks, MCP servers, and scripts. Claude Code keeps them under `~/.claude/plugins`, GitHub Copilot under `~/.copilot/installed-plugins`, Codex under `~/.codex/plugins`, Cursor under `~/.cursor/plugins`. SkillBinder sees none of them, so the user can't tell what each agent loads beside its skills, and moving a plugin to another agent means running that agent's commands by hand. Most of these agents read Claude Code's plugin format and its `.claude-plugin/marketplace.json`, so one plugin can serve several agents when each installs it ([spec](../specs/plugins.md)).

Plugins becomes a sidebar page. It lists the plugins your agents have installed for the user, read from each agent's own records, and installs, updates, and uninstalls a plugin for another of your agents by running that agent's own plugin installer, as Connect does for the MCP server (ADR 0026). SkillBinder never writes a plugin's files, an agent's plugin records, or its settings: outside application storage and the library it still writes only managed copies of library skills (ADR 0006). Installing a plugin is the agent's own act. It is not installing a skill (ADR 0015), and it never touches the library.

| Agent | Read from | Install, update, uninstall |
| --- | --- | --- |
| Claude Code | `installed_plugins.json` (user scope), `known_marketplaces.json`, `enabledPlugins` in `settings.json` | `claude plugin …` |
| GitHub Copilot | `~/.copilot/installed-plugins/<marketplace>/<plugin>` | `copilot plugin …` |
| Droid | its plugin cache | `droid plugin …` |
| Antigravity CLI | `~/.gemini/config/plugins/<plugin>` | `agy plugin …` |
| Codex | `~/.codex/plugins/cache`, `[plugins]` in `config.toml` | none: Install from Codex's /plugins |
| Cursor | `~/.cursor/plugins` | none: Install from Cursor's plugins |

Only chosen agents (ADR 0012) and user-scope installs are read. When the agent being installed to lacks the plugin's marketplace, SkillBinder asks once, naming the marketplace and its repository and showing both commands, then adds the marketplace and installs. A plugin no agent installed from a marketplace (synced from claude.ai, or loaded from a folder) can't be installed elsewhere. An agent whose version of a plugin is lower than another agent's gets Update. A plugin an agent has turned off shows Off and is not turned on or off from SkillBinder. Plugin state is read on launch, on focus, and after each command; nothing about plugins is stored or synced (ADR 0004).

## Considered Options

- Copying a plugin's skills into the library and from there into any agent as managed copies was rejected by the user. It reaches every agent, but it drops the plugin's hooks, MCP servers, commands, and scripts, and leaves `${CLAUDE_PLUGIN_ROOT}` as literal text in the skills that use it.
- Writing the plugin's files and records into the other agent's plugin folder and settings was rejected by the user. It breaks the managed-copies rule, owns every agent's undocumented bookkeeping, and a bad write can break the agent, the reason ADR 0026 gave for MCP settings.
- Project-scoped plugins, listed or installed per workspace, were rejected by the user for now. Installing writes into a project's settings, which ADR 0026 also refused, and presets likewise apply only to global skill directories.
- Adding a missing marketplace without asking, and leaving it to the user, were both rejected by the user. The first silently gives an agent a new third-party source; the second makes the common case manual.
- Turning plugins on and off from SkillBinder was rejected by the user: it adds an `enable`/`disable` command per agent for a choice the agents already offer.
- Listing Claude Code alone first was rejected by the user. Moving a plugin between agents is the reason for the page.

## Consequences

- The engine runs `claude`, `copilot`, `droid`, and `agy` with fixed arguments and shows their error output when they fail. Each agent's commands and record readers live in one table in the engine with a test per agent; a row changes when the agent's plugin layout or command line changes.
- Reading Codex's, Droid's, Antigravity CLI's, and Cursor's plugin folders depends on layouts their docs describe loosely or not at all; each reader names its source and treats an unreadable record as "can't read this agent's plugins", never as no plugins.
- The command line tool and the MCP server gain plugin commands (ADR 0023).
- AGENTS.md's approved scope gains Plugins.
