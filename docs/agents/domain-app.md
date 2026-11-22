# Domain documentation: the app

The vocabulary for SkillBinder as an application, beside the library's in [domain.md](domain.md), whose rules apply here too. It is one context with one vocabulary; this file is split out only for size.

## Vocabulary

- An app update is a newer SkillBinder release on the user's channel (ADR 0017). It is downloaded, verified, and staged in the background, and is then ready: Restart installs it now, and otherwise it installs at the next quit. Where the app cannot replace itself, the update is download-only and opens its release page. The installed version never goes down. Always say "app update" or name SkillBinder; a bare "update" means a skill update from its source. Code says `AppUpdate`.
- A channel is Stable or Beta, chosen in Settings. Stable follows releases (`X.Y.Z`). Beta follows the newer of the latest release and the newest beta (`X.Y.Z-beta.N`), which the Nightly workflow publishes. Avoid "nightly" for the channel; Nightly is the workflow.
- The menu bar menu is SkillBinder's icon and menu in the menu bar or system tray (ADR 0020); while it shows, closing the window keeps the app running. Copy says "menu bar" on macOS, "system tray" elsewhere; code says `Tray`.
- The command line tool is the `skillbinder` command, which asks the running app to run domain actions from a terminal or agent (ADR 0022). Installing the command (ADR 0024) is not installing a skill. `skillbinder mcp` is its MCP server for agents (ADR 0025, ADR 0026). Code says `CommandLine`.
- The SkillBinder skill is SkillBinder's own skill, `skillbinder`, which teaches agents to use its MCP tools (ADR 0028). It is not a library skill: it never shows in Library, Agents, a workspace, Explore, the Import skills dialog, or any count, and the command and MCP list it nowhere but in its own status. It ships inside the app and changes only with an app update. SkillBinder writes it as a managed copy into the global skill directory of each chosen agent connected to the MCP server while the Settings switch **Teach connected agents** is on, whatever the active preset or workspaces; nothing else adds or removes it, except that a library skill `skillbinder`, already held or arriving through Pull, takes the slug and removes the app's copies. Its slug is reserved: scans skip `skillbinder` folders, and import and install refuse the slug. Avoid "built-in skill" and "system skill". Code says `AppSkill`.

Expand this vocabulary only when a domain type or an accepted behavior introduces a term.
