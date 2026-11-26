# Workspace and agent pages also list their found skills that are not in library, and their ignored skills

A workspace's page and an agent's page list only the library skills that are on there (ADR 0007). A project often holds a skill made for it alone, which makes no sense in the library, and the agent loads it all the same. To deal with it, the user had to open Scan for skills, whose dialog mixes every place (ADR 0008), and find it there; to undo an ignore, they had to find the dialog's Ignored group (ADR 0033). The user wants to open the workspace, see the skill, and ignore it there. This partly supersedes ADR 0007, ADR 0008, and ADR 0033 ([spec](../specs/found-skills-on-pages.md)).

A workspace's page, an agent's page, and Universal's page now list, under the skills that are on there, the found skills the latest completed scan found in that place that are not in library, in one **Not in library** section. Each can be ignored, and imported through the Import skills dialog narrowed to this place's skills. An import still turns a skill on wherever it was found (ADR 0008), and the rows say where else it was found. An ignored skill stays in the section, marked Ignored, with Stop ignoring in place of Ignore.

What counts as "there" follows the page's existing rule. An agent's page covers its own global skill directory, and Universal's covers `~/.agents/skills`; neither covers the other directories an agent reads. A workspace's page covers every agent workspace skill directory the scan walks in its folder, as the Import skills dialog groups them under that workspace, including those of agents the user did not choose. An ignore still names a slug everywhere: ignoring on one page ignores the skill on every page and in the dialog. Sidebar counts still count only the skills that are on.

## Considered Options

- A separate, collapsed Ignored section was built first and rejected by the user once seen: ignored skills stay among the others, marked Ignored.

- Read-only lists, with importing and ignoring left to Scan for skills, were rejected by the user. The point is to ignore a project's own skill where it is seen.
- Also listing copies of library skills that sit in a place where the skill is off was rejected by the user. A page should show what may need a decision about the library, not mirror the library's state, and such copies can be many.
- A filter beside the search (On here · Not in library · Ignored), and one card with group headers, were rejected by the user in favor of separate sections (`docs/design/found-skills-on-pages`).
- Importing only into the page's place was rejected by the user. It would add a second import rule beside ADR 0008's.
- Listing every directory an agent loads on its page, such as Universal's `~/.agents/skills` on Cursor's, was rejected. Universal's skills would repeat on many pages.
- Counting not-in-library skills in the sidebar was rejected. Library's Scan for skills count is the one prompt to import.
- Listing only the chosen agents' directories on a workspace's page was rejected. The Import skills dialog lists every walked directory under that workspace, and the page would show fewer.

## Consequences

- ADR 0007's rejection of found skills on an agent's page no longer holds. Its Consequence that a copy of a library skill where it is not active is listed on no page still holds.
- ADR 0008's "found skills that are not in library are listed only in that dialog" no longer holds. Importing still happens only in the Import skills dialog, which can now also open from a page with only that place's skills.
- ADR 0033's rule that ignored skills are listed only in the dialog, and its rejection of listing a workspace's ignored skills on its page, no longer hold. Ignores stay per slug and machine-local.
- The pages show the latest completed scan, so they can lag a change made outside SkillBinder until the next focus, as Scan for skills' count does.
