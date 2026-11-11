# Activation UI copy

These strings were added with activation (ADR 0006); they have no captured baseline.

## Your agents

- A skill page's header, before `Reinstall` and `Update`, holds the `Active` checkbox, tooltip `Active in <preset>`, then the agents button: `All agents`, one Agents row's name such as `Universal`, `<n> agents`, or `No agents`, with a down chevron. It opens the agents menu: `All agents`, a separator, then `Universal` with the layers glyph and each other detected agent with its logo, each checked while the skill is held for an agent reading its directory. Every row's context menu keeps `Active in <preset>`.
- Library's toolbar: the search placeholder `Search skills`, segments `All <n>`, `Active <n>`, `Inactive <n>`, the Updates chip with its count, tooltip and accessible name `Updates`, and the List and Grid segments, tooltips `List` and `Grid`. With nothing listed: `No skills match` with the button `Clear filters`, or `No skills in <folder>`.
- A Library row's or tile's placement area: your agents' logo stack, tooltip `Your agents` then `All agents` or their names joined with `, `; a workspace chip, tooltip `<workspace>` then its directories' names, or a count chip, one line per workspace `<workspace> · <directories>`. Accessible name `Change where <slug> is on`. An inactive skill's dashed `+`: tooltip `Turn on`, accessible name `Turn on <slug>`.
- The popover, a dialog named `Where <slug> is on`: `Your agents` with the preset chip, tooltip `Saved to <preset>`; `All agents` with a switch, tooltip `Includes agents added later`; pills whose tooltip is the directory's name, then for Universal its agents; `No agents detected`. `Workspaces`, then one line per workspace; `No agents yet` under a workspace that uses none; a missing workspace's line has the tooltip `The workspace folder is missing.`
- A warning glyph sits before the placement area while a copy is not applied, asks, or was not removed, tooltip `1 copy needs review` or `<n> copies need review`. A skill page shows that text as a warning note with `Review`. The glyph and `Review` open `Skill copies` for that skill.
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

- A workspace's page holds the earlier rows and cards. Their `Active` checkbox has the tooltip `Active in <workspace>`, and the row menu item reads `Active in <workspace>`. While the workspace holds no skills the notice reads `No skills active in <workspace>`.
- A row's bottom line, or the line above a card's footer, shows the avatars of the workspace's directories that hold the skill's applied copy, Universal first; four show before `+<n>`, whose tooltip names the rest. An avatar's tooltip shows the directory, and Universal's adds `Used by <agents>`. With no applied copy and no warning glyph, the line reads `Not in any agent`.
- Its agents button reads `All agents`, one directory row's name, `<n> agents`, or `No agents`. Its menu lists `All agents`, disabled while the workspace uses no agents, then a separator and one row per workspace skill directory: `Universal` with the layers glyph for `.agents/skills`, then the others by name, each named after the detected agents that read it, joined with `, `.
- A skill also active for your agents, for an agent that reads both, shows the warning tag `Also in your agents`, with the tooltip `An agent reads this skill both here and in your agents, so it may load it twice.`
- Dialog `Choose agents`, shown by the first activation in a workspace that uses no agents: `Which agents does <workspace> use? <slug> is copied into their skill directories.`, or `No agents are detected on this device.`. Each row is a checkbox naming the directory's agents, then the directory with home as `~`, and for Universal `<dir> · Used by <agents>`. Actions: `Cancel`, `Activate`.
- Refusals: `Choose which agents this workspace uses.`, `Choose the skill directory of a detected agent.`, `The workspace folder is missing.`, `Skill no longer exists.`, `Workspace no longer exists.`

## Blocking progress

Added with ADR 0007 (#98); the look follows `docs/design/scoped-pages/prototypes/progress.html`, variant D "Inline".

- The strip's pill reads `Switching…` for a preset switch and for deleting the active preset, or `Importing…` for an import, then `<n>%`. Its tooltip is the change on one line, `Switching to <preset>`, `Switching to Default`, or `Importing <n> skills`, and the step on the next, `Copying <slug> → <place>` or `Removing <slug> from <place>`.
- Done pill: `Done · 1 needs attention` or `Done · <n> need attention`, with `Review` and `Close`.
- Toasts: `Switched to <preset>`, `Deleted <preset> · Switched to Default`, `Imported <n> skills` (`Imported 1 skill`).
- Review list, titled `1 needs attention` or `<n> need attention`, footer `Done`. A row reads `<slug>` then `→ <place>`, a place being the agents that read the directory or `<workspace> · <agents>`, over its reason and answers:

| Copy | Reason | Actions | Answered |
| --- | --- | --- | --- |
| Conflict | `Different copy there` | `Replace`, `Keep` | `Replaced`, `Kept` |
| Edited | `You changed it here` | `Replace`, `Keep` | `Replaced`, `Kept` |
| Edited, inactive | `You changed it here` | `Remove`, `Keep` | `Removed`, `Kept` |
| Not applied, not removed | `Can't write the folder` | `Retry`, `Skip` | `Done`, `Skipped` |
