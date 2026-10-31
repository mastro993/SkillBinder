# Activation UI copy

These strings were added with activation (ADR 0006); they have no captured baseline.

## Your agents

- A List row's and Grid card's trailing edge, and a skill page's header before `Reinstall` and `Update`, hold the `Active` checkbox, tooltip `Active in <preset>`, then the agents button: `All agents`, one Agents row's name such as `Universal`, `<n> agents`, or `No agents`, with a down chevron. It opens the agents menu: `All agents`, a separator, then `Universal` with the layers glyph and each other detected agent with its logo, each checked while the skill is held for an agent reading its directory. The row context menu keeps `Active in <preset>`.
- A row's bottom line, or a card's footer, shows the avatars of the Agents rows whose global skill directory holds the skill's applied copy, Universal first; four show before `+<n>`, whose tooltip names the rest. An avatar's tooltip shows the directory, and Universal's adds `Used by <agents>`. A warning glyph follows while a copy is not applied, asks, or was not removed, tooltip `1 copy needs review` or `<n> copies need review`. With neither, the line reads `Not in any agent`. A skill page shows that text as a warning note with `Review`. The glyph and `Review` open `Skill copies` for that skill.
- Dialog `Skill copies`, footer `Close`, also opens once when copies newly ask, listing every copy that asks; `<dir>` shows home as `~`:

| Copy | Text | Actions |
| --- | --- | --- |
| Conflict | `<dir> already holds a different copy of <slug>.` | `Replace`, `Skip` |
| Skipped | `<dir> keeps its own copy of <slug>, so <slug> is not applied there.` | `Replace` |
| Edited | `The copy of <slug> in <dir> was edited there.` | `Overwrite`, `Keep` |
| Edited, inactive | `<slug> is no longer active in <dir>, but its copy there was edited.` | `Remove`, `Keep` |
| Not applied | `<slug> is not applied in <dir>. <reason>` | `Retry` |
| Not removed | `The copy of <slug> in <dir> was not removed. <reason>` | `Retry` |

- Reasons: `Not applied yet.`, `SkillBinder could not write to this directory.`, `Another copy appeared meanwhile.`, `The skill changed while it was copied.`, `This skill cannot safely be copied.`, `The library skill could not be read.`, `This directory is inside SkillBinder's own storage.` A stale answer is refused with `This copy changed meanwhile. Review it again.`
- Deleting the active preset adds, after `Delete <preset>? Its <count> skills stay in the library.`, either `Default becomes the active preset, which adds <a, b> and removes <c> for your agents.`, with only `adds <a>` or `removes <c>` when one side is empty, or `Default becomes the active preset, with the same skills.`

## Workspaces

- A workspace's page holds the same rows and cards as Library. Their `Active` checkbox has the tooltip `Active in <workspace>`, and the row menu item reads `Active in <workspace>`. While the workspace holds no skills the notice reads `No skills active in <workspace>`.
- Its agents button reads `All agents`, one directory row's name, `<n> agents`, or `No agents`. Its menu lists `All agents`, disabled while the workspace uses no agents, then a separator and one row per workspace skill directory: `Universal` with the layers glyph for `.agents/skills`, then the others by name, each named after the detected agents that read it, joined with `, `.
- A skill also active for your agents, for an agent that reads both, shows the warning tag `Also in your agents`, with the tooltip `An agent reads this skill both here and in your agents, so it may load it twice.`
- Dialog `Choose agents`, shown by the first activation in a workspace that uses no agents: `Which agents does <workspace> use? <slug> is copied into their skill directories.`, or `No agents are detected on this device.`. Each row is a checkbox naming the directory's agents, then the directory with home as `~`, and for Universal `<dir> · Used by <agents>`. Actions: `Cancel`, `Activate`.
- Refusals: `Choose which agents this workspace uses.`, `Choose the skill directory of a detected agent.`, `The workspace folder is missing.`, `Skill no longer exists.`, `Workspace no longer exists.`
