# Ignore found skills: design decisions

The user settled these on 2026-11-25. Behavior comes from [the spec](../../specs/ignore-found-skills.md) and [ADR 0033](../../adr/0033-ignored-skills-are-remembered-per-slug.md). They add to [scan-import's decisions](../scan-import/decisions.md), whose ids stay valid. Ids are stable; cite them from issues and pull requests.

- **I1** Ignoring happens only in the Import skills dialog, from Scan for skills and from onboarding's Review list. Workspace and agent pages are unchanged.
- **I2** Each offered row ends with a small ghost "Ignore" button, shown while the row is hovered or holds keyboard focus, like Delete on Library rows. Tooltip "Don't offer this skill again"; accessible name "Ignore <slug>". Blocked, empty, and invalid rows have it too.
- **I3** Each place's group header (scan-import D3) ends with a small ghost "Ignore all", tooltip "Don't offer these N skills again". It ignores every offered skill in that group, which the group lists by its first location, so their slugs are ignored everywhere. Onboarding's Review list has no group header, so it has no Ignore all.
- **I4** Ignoring is saved at once: the row leaves its group, its tick, Select all, the Import count, and Scan for skills' count, and the Ignored group's header flashes once. Cancel, Escape, and closing keep it. No toast; the Ignored group is the undo.
- **I5** The Ignored group closes the list, after every place: a disclosure header with a chevron, "Ignored", the count, and the muted hint "Not offered or counted". It starts collapsed in every dialog that opens and is absent when nothing found is ignored. It has no checkbox.
- **I6** An ignored row has no checkbox, keeps the checkbox's space so slugs align, and shows the slug and description dimmed, then "Found in <places>" from every location of every version, and at its end a small outline "Stop ignoring" (accessible name "Stop ignoring <slug>").
- **I7** Stop ignoring returns the skill to its place's group, ticked if its chosen version is valid and the 500 cap allows (scan-import D5), and the row flashes once (scan-import D11's flash).
- **I8** The status line reads "N found · M already in library · K ignored", where N counts offered skills only; "· K ignored" shows only when K is above zero. Scan for skills' count (L2) and its accessible label (L3) count offered skills only.
- **I9** The search filters ignored rows too. While a query matches an ignored skill, the Ignored group shows expanded with only its matching rows; "No skills match '<query>'" shows only when neither part matches.
- **I10** When every found skill is ignored, the list's first line reads "No new skills found." with the Ignored group under it, and Import stays inactive. With nothing found at all, the dialog keeps scan-import D12's "No new skills found." with Close.
- **I11** Copy: "Ignore", "Ignore all", "Ignored", "Stop ignoring". Never "Hide", "Dismiss", or "Skip"; skipping is leaving a row unticked.
