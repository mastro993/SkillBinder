# Found skills on pages: design decisions

The user settled these on 2026-11-26. Behavior comes from [the spec](../../specs/found-skills-on-pages.md) and [ADR 0036](../../adr/0036-pages-list-their-found-and-ignored-skills.md). They add to [scoped-pages' decisions](../scoped-pages/decisions.md) and [ignore-skills' decisions](../ignore-skills/decisions.md), whose ids stay valid. Ids are stable; cite them from issues.

- **U1** Layout is variant A "Sections". The page keeps its search field and its card of skills on here, unchanged. Under the card come up to two sections, in this order: "Not in library" and "Ignored". A section with no rows is absent.
- **U2** The "Not in library" head is one line: "Not in library" in semibold, its count in muted figures, and the muted hint "Agents load these; SkillBinder does not manage them". The rows follow in a bordered card like the list above them. The head is a disclosure like Ignored's (U7), open on every page visit, so the user can fold the rows; a search that matches them shows them open (user request, 2026-11-26).
- **U3** A "Not in library" row shows the slug with its validation glyph, then the description ("No description" when empty), then a muted mono line. On a workspace's page, that line names the directories holding the skill, such as `.claude/skills`. On every page, it then adds "Also found in <places>" when the skill was found elsewhere. The rows are not links, since a found skill has no page.
- **U4** A "Not in library" row ends with a small ghost "Ignore" and a small outline "Import…". Both are shown while the row is hovered or holds keyboard focus, like Delete on Library rows.
  - Ignore's tooltip is "Ignore everywhere: never offered for import".
  - The accessible names are "Ignore <slug>" and "Import <slug>".
- **U5** The "Not in library" head ends with a small ghost "Ignore all" and a small outline "Import…".
  - Ignore all ignores every skill the section lists, including rows the search hides.
  - Ignoring follows I4: it is saved at once, there is no toast, and the Ignored section is the undo.
- **U6** Import… opens the Import skills dialog with only this place's not-in-library skills, or from a row, only that skill.
  - Every rule of the dialog holds inside it: versions, the 500 cap, and ticks.
  - Each listed skill still shows every location it was found in, and importing turns it on wherever it was found.
- **U7** The Ignored section's head is a disclosure: a chevron, "Ignored", and the count, with no hint (the prototype's "Not offered for import anywhere" was dropped at the user's request, 2026-11-26).
  - It starts collapsed on every page visit.
  - Its rows follow I6: dimmed, with the page's directory line and a small outline "Stop ignoring".
  - Stop ignoring moves the row back to "Not in library" and flashes it.
- **U8** The search narrows both sections by slug or description, ignoring case. While a query matches an ignored skill, the Ignored section shows expanded with only its matching rows (I9). "No skills match '<query>'" replaces the page only when nothing matches.
- **U9** A page with nothing on keeps its empty state above the sections, as one line in the card: "No skills on in <place>", plus " · No agents here yet" for a workspace with no directories. With nothing found either, it keeps today's full empty state with Add skill.
- **U10** Sidebar counts, Scan for skills' count, Library, and the Add skill picker do not change.
- **U11** Copy: "Not in library", "Ignored", "Ignore", "Ignore all", "Import…", "Stop ignoring". Never "Hide", "Dismiss", or "Skip" (I11).
