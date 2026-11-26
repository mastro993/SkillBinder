# Found skills on pages: design decisions

The user settled these on 2026-11-26. Behavior comes from [the spec](../../specs/found-skills-on-pages.md) and [ADR 0036](../../adr/0036-pages-list-their-found-and-ignored-skills.md). They add to [scoped-pages' decisions](../scoped-pages/decisions.md) and [ignore-skills' decisions](../ignore-skills/decisions.md), whose ids stay valid. Ids are stable; cite them from issues.

- **U1** Layout is variant A "Flat" of `prototypes/one-list.html` (user's pick, 2026-11-26), replacing variant A "Sections" of `place-pages.html`. The page keeps its search field and one card: the skills on here and the found skills not in library, ignored ones included, in one list sorted by slug. There is no section or group header.
- **U2** A found row carries an outline "Not in library" chip after its slug, or a neutral "Ignored" chip while its slug is ignored, with its slug and description dimmed. The search row ends, while any found skill is not ignored, with "<N> not in library" in muted figures.
- **U3** A found row shows the slug with its validation glyph, then the description ("No description" when empty), then a muted mono line. On a workspace's page, that line names the directories holding the skill, such as `.claude/skills`. On every page, it then adds "Also found in <places>" when the skill was found elsewhere. The rows are not links, since a found skill has no page.
- **U4** A found row that is not ignored ends with a small ghost "Ignore" and a small outline "Import…". Both are shown while the row is hovered or holds keyboard focus, like Delete on Library rows.
  - Ignore's tooltip is "Ignore everywhere: never offered for import".
  - The accessible names are "Ignore <slug>" and "Import <slug>".
- **U5** The search row ends with a small ghost "Ignore all" and a small outline "Import…", after the count (U2).
  - Ignore all ignores every found skill on the page that is not ignored yet, including rows the search hides. It is absent when every one is ignored.
  - Ignoring follows I4: it is saved at once and there is no toast. The row stays in place with the Ignored tag, and its Stop ignoring is the undo.
- **U6** Import… opens the Import skills dialog with only this place's not-in-library skills, or from a row, only that skill.
  - Every rule of the dialog holds inside it: versions, the 500 cap, and ticks.
  - Each listed skill still shows every location it was found in, and importing turns it on wherever it was found.
- **U7** An ignored row ends with a small outline "Stop ignoring" (accessible name "Stop ignoring <slug>") in place of Ignore and Import…, shown like Ignore while the row is hovered or holds keyboard focus. Stop ignoring drops the Ignored chip and flashes the row once.
- **U8** The search narrows the whole list by slug or description, ignoring case, found and ignored skills included. "No skills match '<query>'" replaces the list only when nothing matches.
- **U9** A page with nothing on but found skills starts its card with one line: "No skills on in <place>", plus " · No agents here yet" for a workspace with no directories. With nothing found either, it keeps today's full empty state with Add skill.
- **U10** Sidebar counts, Scan for skills' count, Library, and the Add skill picker do not change.
- **U11** Copy: "Not in library", "Ignored", "Ignore", "Ignore all", "Import…", "Stop ignoring". Never "Hide", "Dismiss", or "Skip" (I11).
