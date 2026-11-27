# Library lists plugin skills, which stay their plugins', and manages plugins in a Plugins tab

ADR 0035 kept plugin skills out of Library and gave plugins their own sidebar page. The owner then asked for Library to be the one place for every skill their agents load: on their machine, 13 Claude Code plugins ship about 30 skills, and listing them on a separate page split "what my agents load" in two. Nothing of the Plugins pages was built yet. This supersedes ADR 0035, and partly supersedes ADR 0034 (the sidebar page), ADR 0007 (Library and an agent's page list only library skills), and ADR 0036 (agent pages never list plugin skills) ([spec](../specs/plugins.md)).

A plugin skill is still never a library skill. It is not imported, installed, synced, put in presets, folders, or workspaces, or activated, and it changes only when an agent installs, updates, or uninstalls its plugin (ADR 0034). What changes is where it shows:

- **Library lists plugin skills among library skills**, alphabetically under All skills, each with a filled chip naming its plugin. Where a library skill's row has its placement area, a plugin skill's row shows the agents that have its plugin installed, read-only; it and the chip open the plugin. A plugin skill is Active when its plugin is installed and on in at least one of your agents, and Inactive when it is Off everywhere; Library's counts and filters include plugin skills. They never appear under a folder tab and never count as updates.
- **A slug in the library and in a plugin shows as two rows**, side by side: the library skill with its usual controls and an "Also in <plugin>" mark, and the plugin skill with its plugin chip. The plugin row carries no second mark, since the rows sit together.
- **Plugins are managed from Library.** A Plugins tab after All skills lists the plugins; a plugin opens its page (header, Agents with Install, Update, and Uninstall, Skills, README) under the crumb Library › Plugins, and a plugin skill opens its read-only page under it. There is no Plugins row in the sidebar.
- **An agent's page also lists the plugin skills that agent loads**, read-only, in its one list beside its library skills and found skills, with the same chip. A plugin that is Off there loads nothing, so its skills are not listed. Workspace pages are unchanged: only user-scope plugins are read (ADR 0034).

## Considered Options

- Adding a plugin skill to the library as an ordinary library skill, installed from the plugin's repository, was rejected by the owner: a plugin skill is never a library skill. It would also have loaded twice in an agent that keeps the plugin, since a plugin's skill cannot be turned off alone.
- Mirroring every plugin skill into the library automatically was rejected for the same reason, and because it would sync skills the user never picked.
- Grouping plugin skills under one collapsible header per plugin below the library skills (`docs/design/plugins`, Library variant B) was rejected by the owner in favor of one alphabetical list.
- A tab per plugin beside the folder tabs (variant C) was rejected: with a dozen plugins the tab strip scrolls, and plugins mix with the user's folders.
- An "Also in Library" mark on the plugin row was dropped by the owner as redundant beside the library row.
- Turning a plugin on in an agent from a plugin skill's row, as a library skill's popover does, was rejected by the owner: it installs the whole plugin, hooks and MCP servers included, from a control that looks like a skill's. The row opens the plugin instead.

## Consequences

- Library's list holds two kinds of item; every filter, count, and search runs over both, and a library-only action (placement, folders, presets, Update, the context menu) has no plugin-skill counterpart.
- Library's sidebar count and segment counts grow by the plugin skills.
- An agent's page lists what that agent loads from plugins, so a double load (a library copy and a plugin copy of one slug) shows there as two rows.
- Library's routes gain the plugin page and the plugin skill page; the Plugins sidebar route is gone.
