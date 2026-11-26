# Workspace and agent pages also list the skills SkillBinder does not manage there

A workspace's page and an agent's page list only the library skills that are on there (ADR 0007). Agents also load every other skill in those folders: found skills that are not in library, ignored skills, and copies of library skills that are off there. The user audits a place by opening its page and could not see them. Found skills were listed only in the Import skills dialog, which mixes every place (ADR 0008), and ignored skills only in that dialog's Ignored group (ADR 0033). This partly supersedes ADR 0007, ADR 0008, and ADR 0033 ([spec](../specs/unmanaged-skills-on-pages.md)).

A workspace's page, an agent's page, and Universal's page now list, under the skills that are on there, every skill the latest completed scan found in that place's skill directories that SkillBinder does not manage there, in three groups:

- **Not in library**: found skills that are not in library and not ignored. Each can be ignored, and imported through the Import skills dialog narrowed to this place's skills. An import still turns a skill on wherever it was found (ADR 0008), and the rows say where else it was found.
- **In library, off here**: unmanaged copies, which are copies of a library skill's slug in a directory where that skill is not on. Each can be turned on through the page's usual control: an agent's page turns it on for that agent's global skill directory in the active preset, and a workspace's page for the workspace's directories the user ticks. This is the same change Add skill makes, so a copy whose content differs is a conflict that the Skill copies dialog asks about, as ADR 0006 requires.
- **Ignored**: ignored skills found there, in a collapsed group, each with Stop ignoring.

What counts as "there" follows the page's existing rule. An agent's page covers its own global skill directory, links resolved, and Universal's covers `~/.agents/skills`. Neither covers the other directories an agent reads. A workspace's page covers every agent workspace skill directory that the scan walks in its folder, including those of agents the user did not choose. Each row names its directory, but only the chosen agents' directories can be turned on. An ignore still names a slug everywhere: ignoring on one page ignores the skill on every page and in the dialog. Sidebar counts still count only the skills that are on.

## Considered Options

- Read-only lists, with importing and ignoring left to Scan for skills, were rejected by the user. An audit that finds a stray skill should end on the same page.
- A filter beside the search (On here · Not managed · Ignored), and one card with group headers, were rejected by the user in favor of separate sections (`docs/design/unmanaged-skills`). The filter hides the stray skills behind a click, and one card puts rows that are on and rows that are not in the same frame.
- Importing only into the page's place was rejected by the user. It would add a second import rule beside ADR 0008's, and the skill's copies elsewhere would become unmanaged copies at once.
- Listing only found skills not in library and ignored skills, without unmanaged copies, was rejected by the user. ADR 0007 already conceded that those copies are "listed on no page" while agents load them, and an audit page that leaves them out is incomplete.
- Listing every directory an agent loads on its page, such as Universal's `~/.agents/skills` on Cursor's, was rejected. Universal's skills would repeat on many pages, and Turn on would have to name a directory the page does not otherwise show.
- Counting not-in-library skills in the sidebar was rejected. Library's Scan for skills count is the one prompt to import, and a second number on every row is noise.
- Listing only the chosen agents' directories on a workspace's page was rejected. The Import skills dialog lists the skills in every walked directory under that workspace, and the page would show fewer.

## Consequences

- ADR 0007's rejection of found skills on an agent's page, and its Consequence that unmanaged copies are listed on no page, no longer hold. ADR 0007's meaning of active, its removal rules, and its blocking rules are unchanged.
- ADR 0008's "found skills that are not in library are listed only in that dialog" no longer holds. Importing still happens only in the Import skills dialog, which can now also open from a page with only that place's skills.
- ADR 0033's rule that ignored skills are listed only in the dialog, and its rejection of listing a workspace's ignored skills on its page, no longer hold. Ignores stay per slug and machine-local.
- The pages show the latest completed scan. They can lag a change made outside SkillBinder until the next focus or scan, as Scan for skills' count does.
- A scan location now records the skill directories that hold it, so a page can tell which directory a copy sits in even through nested folders and links.
