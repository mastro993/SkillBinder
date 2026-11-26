# Found skills on pages: design decisions

The user settled these on 2026-11-26. Behavior comes from [the spec](../../specs/found-skills-on-pages.md) and [ADR 0036](../../adr/0036-pages-list-their-found-and-ignored-skills.md). They add to [scoped-pages' decisions](../scoped-pages/decisions.md) and [ignore-skills' decisions](../ignore-skills/decisions.md), whose ids stay valid. Ids are stable; cite them from issues.

- **U1** Layout is variant A "Sections". The page keeps its search field and its card of skills on here, unchanged. Under the card comes one section, "Not in library", absent with no rows. Ignored skills stay in it, in slug order with the others, marked with an "Ignored" tag; there is no separate Ignored section (user request, 2026-11-26, replacing the prototype's collapsed Ignored section).
- **U2** The "Not in library" head is one line: "Not in library" in semibold, its count in muted figures, and the muted hint "Agents load these; SkillBinder does not manage them". The rows follow in a bordered card like the list above them. The head is a disclosure, a ghost button with a chevron, open on every page visit, so the user can fold the rows; a search that matches them shows them open (user request, 2026-11-26). Its count includes ignored skills.
- **U3** A "Not in library" row shows the slug with its validation glyph, then the description ("No description" when empty), then a muted mono line. On a workspace's page, that line names the directories holding the skill, such as `.claude/skills`. On every page, it then adds "Also found in <places>" when the skill was found elsewhere. The rows are not links, since a found skill has no page.
- **U4** A "Not in library" row ends with a small ghost "Ignore" and a small outline "Import…". Both are shown while the row is hovered or holds keyboard focus, like Delete on Library rows.
  - Ignore's tooltip is "Ignore everywhere: never offered for import".
  - The accessible names are "Ignore <slug>" and "Import <slug>".
- **U5** The "Not in library" head ends with a small ghost "Ignore all" and a small outline "Import…".
  - Ignore all ignores every skill the section lists that is not ignored yet, including rows the search hides. It is absent when every skill is ignored.
  - Ignoring follows I4: it is saved at once and there is no toast. The row stays in place with the Ignored tag, and its Stop ignoring is the undo.
- **U6** Import… opens the Import skills dialog with only this place's not-in-library skills, or from a row, only that skill.
  - Every rule of the dialog holds inside it: versions, the 500 cap, and ticks.
  - Each listed skill still shows every location it was found in, and importing turns it on wherever it was found.
- **U7** An ignored skill's row is a "Not in library" row with a small secondary "Ignored" tag after the slug. In place of Ignore and Import… it ends with a small outline "Stop ignoring" (accessible name "Stop ignoring <slug>"), shown like Ignore while the row is hovered or holds keyboard focus. Stop ignoring drops the tag and flashes the row once.
- **U8** The search narrows the section by slug or description, ignoring case, ignored skills included. While a query matches some, the section shows open with only its matching rows. "No skills match '<query>'" replaces the page only when nothing matches.
- **U9** A page with nothing on keeps its empty state above the section, as one line in the card: "No skills on in <place>", plus " · No agents here yet" for a workspace with no directories. With nothing found either, it keeps today's full empty state with Add skill.
- **U10** Sidebar counts, Scan for skills' count, Library, and the Add skill picker do not change.
- **U11** Copy: "Not in library", "Ignored", "Ignore", "Ignore all", "Import…", "Stop ignoring". Never "Hide", "Dismiss", or "Skip" (I11).
