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

- A workspace's page lists the skills the workspace holds. With no match: `No skills match '<query>'`. With none: `No skills in <workspace>`, then `No agents here yet` while the workspace uses no directory, and `Add skill`.
- A row's agents button reads `All agents`, one folder's name, `<n> agents`, or `No agents`; its accessible name is `Agents: <label>`. Its stack shows four avatars before `+<n>`, whose tooltip names the rest. Its menu lists `All agents`, tooltip `Includes agents added later`, disabled while the workspace uses no directory, then a separator and one checkbox row per folder the workspace's directories resolve to: `Universal` with the layers glyph for `.agents/skills`, then the others by name, each named after the detected agents that read it, joined with `, `. A row's tooltip is the directory with home as `~`, then `Used by <agents>` for Universal or a folder several rows share.
- Delete's tooltip and accessible name: `Remove from <workspace>`. Unticking a skill's last folder asks `Remove <skill> from <workspace>?` with `Cancel` and `Remove`. The toast reads `Removed <skill>` with `Undo`.
- A removal that leaves an edited copy asking shows `You changed this skill here`, then `<skill> · <workspace> · <folders>`, with `Keep` and `Remove`.
- A row's marks: the robot glyph, tooltip `Also in your agents`, `Hidden by your agents`, `Overrides your agents`, or `One copy is used`, then `All agents` or your agents' names; the pencil-edit glyph, tooltip `Changed here`, then the folders; the review glyph, tooltip `1 copy needs review` or `<n> copies need review`.
- Refusals: `Choose which agents this workspace uses.`, `Select skills to add.`, `Pick an agent for <slug>.`, `Choose the skill directory of a detected agent.`, `The workspace folder is missing.`, `Skill no longer exists.`, `Workspace no longer exists.`

## Add skill picker

Added with ADR 0007 (#100); the look follows `docs/design/scoped-pages/prototypes/picker.html`, variant C "Two panes".

- A workspace's, an agent's, or Universal's page header: the button `Add skill`; on a workspace whose folder is missing it is disabled, tooltip `The workspace folder is missing.`
- Title `Add skill`, a muted `to`, then the place's chip: a workspace's name, tooltip its folder; an agent's name, or `Universal`, tooltip its global skill directory with home as `~`, and for Universal `Used by <agents>` on the next line. On an agent's page the preset chip follows, tooltip `Saved to <preset>`. The close button's tooltip is `Close`.
- Left pane: the search placeholder `Search library`; chips `All <n>` and `<folder> <n>`; a row's update glyph, tooltip `Update available`; a skill already there, tooltip `Already here`. Empty list: `No skills match` with the button `Clear search`, or `No skills in your library`.
- Right pane: `Selected`, then the count once something is picked; `Nothing selected`. A card's ×, tooltip `Deselect`, accessible name `Deselect <slug>`. In a workspace, the pill `All`, tooltip `Includes agents added later`; a directory pill's tooltip is its name, its agents for Universal, then its directory, adding ` · not used here yet`; `Pick an agent` on a card with none ticked; `No agents are detected on this device.`
- Footer: `Cancel`, then `Add` or `Add <n>`, disabled with the tooltip `Select skills` or `Pick an agent for <slug>`.

## Blocking progress

Added with ADR 0007 (#98); the look follows `docs/design/scoped-pages/prototypes/progress.html`, variant D "Inline".

- The strip's pill reads `Switching…` for a preset switch and for deleting the active preset, `Importing…` for an import, or `Adding…` for the Add skill picker, then `<n>%`. Its tooltip is the change on one line, `Switching to <preset>`, `Switching to Default`, `Importing <n> skills`, `Adding <slug>`, or `Adding <n> skills`, and the step on the next, `Copying <slug> → <place>` or `Removing <slug> from <place>`.
- Done pill: `Done · 1 needs attention` or `Done · <n> need attention`, with `Review` and `Close`.
- Toasts: `Switched to <preset>`, `Deleted <preset> · Switched to Default`, `Imported <n> skills` (`Imported 1 skill`), `Added <slug>`, `Added <n> skills`.
- Review list, titled `1 needs attention` or `<n> need attention`, footer `Done`. A row reads `<slug>` then `→ <place>`, a place being the agents that read the directory or `<workspace> · <agents>`, over its reason and answers:

| Copy | Reason | Actions | Answered |
| --- | --- | --- | --- |
| Conflict | `Different copy there` | `Replace`, `Keep` | `Replaced`, `Kept` |
| Edited | `You changed it here` | `Replace`, `Keep` | `Replaced`, `Kept` |
| Edited, inactive | `You changed it here` | `Remove`, `Keep` | `Removed`, `Kept` |
| Not applied, not removed | `Can't write the folder` | `Retry`, `Skip` | `Done`, `Skipped` |
