# Domain documentation: plugins

The vocabulary for agents' plugins (ADR 0034, ADR 0035), beside the library's in [domain.md](domain.md), whose rules apply here too. It is one context with one vocabulary; this file is split out only for size.

## Vocabulary

- A plugin is a bundle an agent installs from a marketplace: skills, and often commands, subagents, hooks, MCP servers, and scripts, described by a manifest such as `.claude-plugin/plugin.json`. SkillBinder never writes one; the agent that installed it owns its files. A plugin is identified by its name and its marketplace's name, `fstack@fedemas`, so the same plugin in several agents is one plugin. Code says `Plugin`.
- A marketplace is where an agent installs plugins from: a Git repository whose marketplace file lists plugins, under a name the agent shows, such as `claude-plugins-official`. Use "marketplace" only for plugins; Explore's catalog is not a marketplace. A plugin with no marketplace, synced from claude.ai or loaded from a folder, is local and can't be installed in another agent.
- A plugin agent is one of your agents whose plugins SkillBinder reads: Claude Code, GitHub Copilot, Droid, Antigravity CLI, Codex, and Cursor. Of those, the agents with a plugin installer (all but Codex and Cursor) can have plugins installed, updated, and uninstalled from SkillBinder; the others show where to do it. Code says `PluginAgent`.
- A plugin is installed for an agent when the agent installed it for the user (user scope). Installing, updating, and uninstalling a plugin are the agent's acts, which SkillBinder runs through the agent's own installer; they are not installing a skill, and never touch the library. Plugins installed for one project are not read. Avoid "added", "enabled", and "connected" for this state.
- A plugin is Off in an agent that installed it and turned it off; it is still installed. SkillBinder shows Off but never turns a plugin on or off.
- A plugin is behind in an agent whose installed version is lower than the highest version another of your agents has installed. Only versions that compare as numbers can be behind. Update runs that agent's own plugin update.
- A plugin skill is a skill a plugin ships, which its agent loads as `<plugin>:<skill>`. It is never a library skill or a found skill: it is not imported, installed, counted, or activated, and it changes only with its plugin. A library skill whose slug a plugin skill shares is "Also in plugin <name>".

Expand this vocabulary only when a domain type or an accepted behavior introduces a term.
