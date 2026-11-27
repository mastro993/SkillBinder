# Plugin skills stay outside the library, which marks the slugs they share

Superseded by ADR 0041: plugin skills are still never library skills, but Library lists them beside library skills, marked with their plugin, and plugins are managed from its Plugins tab.

A plugin's skills load in the agent as `<plugin>:<skill>`, beside the agent's skill directories, and the scan never reads plugin folders, so they are not found skills. A library skill can still share a slug with one: on the owner's machine the library holds `caveman` and its siblings and `rust-best-practices`, imported from `~/.agents/skills`, while Claude Code also has the `caveman` and `beautiful-rust` plugins. Claude Code then loads both, and turning the library skill off leaves the plugin's copy loading, which reads as a skill SkillBinder can't manage ([spec](../specs/plugins.md)).

Plugin skills are never library skills. They are not imported, installed, counted, activated, put in presets or workspaces, or listed in Library, Agents, the Import skills dialog, or Explore; the plugin owns them, and they change only when an agent installs, updates, or uninstalls the plugin (ADR 0034). The Plugins page lists them under their plugin, and each opens a read-only page like a library skill's, showing its front matter and SKILL.md, with no activation controls.

Library marks a library skill whose slug a plugin of one of your agents also ships with "Also in plugin <name>", which opens the plugin. The plugin skill's page links back with "Also in Library". Nothing resolves the overlap automatically: the user decides whether to uninstall the plugin or turn the library skill off.

## Considered Options

- Listing plugin skills in Library as read-only rows with a plugin badge was rejected by the user. It puts rows that can't be turned on or off among library skills, which every Library control assumes can be, and it goes against ADR 0007's rule that pages list library skills.
- Leaving the overlap unmarked was rejected by the user: the double load stays invisible.
- Opening a plugin skill in a dialog, as Explore does, was rejected by the user in favor of a page with Back and Forward (`docs/design/plugins`).

## Consequences

- The skill page's route and its preview take only library skills; plugin skills get their own route and preview.
- The overlap is compared the way the library compares slugs, against the plugins in the latest read, so it can lag an install made outside SkillBinder until the next focus.
