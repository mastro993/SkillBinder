# Plugins are listed from the agents' own records and installed through their own installers

Many agents now install plugins: bundles that hold skills and often commands, subagents, hooks, MCP servers, and scripts. Pi calls them packages, Gemini CLI and Qwen Code extensions, Kiro powers; Claude Code keeps them under `~/.claude/plugins`, Pi lists them in `~/.pi/agent/settings.json`. Of the 79 agents SkillBinder supports, 41 have plugins that can ship skills, and about a dozen read Claude Code's plugin format and its `.claude-plugin/marketplace.json` directly ([agents](../specs/plugins/agents.md)). SkillBinder sees none of them, so the user can't tell what each agent loads beside its skills, and moving a plugin to another agent means running that agent's commands by hand ([spec](../specs/plugins.md)).

Plugins becomes a sidebar page. It lists the plugins your agents have installed for the user, read from each agent's own records, and installs, updates, and uninstalls a plugin for another of your agents by running that agent's own installer, as Connect does for the MCP server (ADR 0026). SkillBinder never writes a plugin's files, an agent's plugin records, or its settings: outside application storage and the library it still writes only managed copies of library skills (ADR 0006). Installing a plugin is the agent's own act. It is not installing a skill (ADR 0015), and it never touches the library.

The first version covers the 24 agents whose installer SkillBinder can run from the command line, added a few per slice, each confirmed against the agent's `--help` and a real install. One engine table holds a row per agent: where it records plugins, the agent's own word for them, the formats it takes, and the commands it offers. SkillBinder always says "plugin", and calls where a plugin comes from its source: a marketplace, a Git repository, or an npm package. An agent installs a plugin in whichever form it takes, `copilot plugin install fstack@fedemas` or `pi install git:github.com/mastro993/fstack`; an agent that can't use the plugin's format says so and offers no Install, since SkillBinder doesn't convert plugins.

Only chosen agents (ADR 0012) and user-scope installs are read. When the agent being installed to lacks the plugin's marketplace, SkillBinder asks once, naming the marketplace and its repository and showing both commands, then adds the marketplace and installs. A plugin with no source can't be installed elsewhere. An agent whose version of a plugin is lower than another agent's gets Update where its installer has one. A plugin an agent has turned off shows Off and is not turned on or off from SkillBinder. Plugin state is read on launch, on focus, and after each command; nothing about plugins is stored or synced (ADR 0004).

## Considered Options

- Copying a plugin's skills into the library and from there into any agent as managed copies was rejected by the user. It reaches every agent, but it drops the plugin's hooks, MCP servers, commands, and scripts, and leaves `${CLAUDE_PLUGIN_ROOT}` as literal text in the skills that use it.
- Writing the plugin's files and records into the other agent's plugin folder and settings was rejected by the user. It breaks the managed-copies rule, owns every agent's undocumented bookkeeping, and a bad write can break the agent, the reason ADR 0026 gave for MCP settings.
- Listing the plugins of the 14 agents that install only from their own app or a slash command (Cursor, Junie, Kimi Code, …), with a hint, was deferred by the user: it doubles the readers, many on undocumented layouts, for agents SkillBinder can't act on.
- Only the agents that take Claude Code's `name@marketplace` form was rejected by the user: Pi, Gemini CLI, and the other agents that install from a repository would wait.
- Offering Install on every agent and letting the agent's error explain an unusable format, and hiding agents that can't use a plugin, were rejected by the user. The first leads to failures SkillBinder could foresee; the second hides why an agent is missing.
- Each agent's own word (package, extension, power) as the label was rejected by the user in favor of one word, with the agent's in a tooltip.
- Plugins that are code registering skills when they run (Amp, AiderDesk) were left out by the user: their skills can't be listed without running them, and neither has an installer.
- Project-scoped plugins, listed or installed per workspace, were rejected by the user for now. Installing writes into a project's settings, which ADR 0026 also refused, and presets likewise apply only to global skill directories.
- Adding a missing marketplace without asking, and leaving it to the user, were both rejected by the user. The first silently gives an agent a new third-party source; the second makes the common case manual.
- Turning plugins on and off from SkillBinder was rejected by the user: it adds an `enable`/`disable` command per agent for a choice the agents already offer.

## Consequences

- The engine runs 24 agents' programs (`claude`, `copilot`, `codex`, `pi`, …) with fixed arguments and shows their error output when they fail. A row changes when its agent's plugin layout or command line changes, and is tested alone.
- Many agents' plugin records are documented loosely or not at all; each reader names its source and treats an unreadable record as "can't read this agent's plugins", never as no plugins.
- Whether an agent can use a plugin is decided from the manifests on disk, so an agent that accepts more than its row knows shows as can't use until the row learns the format.
- The command line tool and the MCP server gain plugin commands (ADR 0023).
- AGENTS.md's approved scope gains Plugins.
