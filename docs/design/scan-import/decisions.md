# Scan and import: design decisions

The user settled these on 2026-11-16. Behavior comes from [the spec](../../specs/scan-and-import.md), [ADR 0008](../../adr/0008-import-from-scan-dialog.md), and [ADR 0009](../../adr/0009-prepare-once-apply-selection.md). Where this file and the spec disagree, this file holds. Ids are stable; cite them from issues and pull requests.

Ignoring found skills adds I1–I11 in [ignore-skills](../ignore-skills/decisions.md) (ADR 0033). The count in L2 and L3, the ticks in D5 and D6, and the status line in D2 cover only skills that are not ignored. "Skipped skills keep counting" (L2) still holds for skills left unticked.

## General

- **G1** Import happens only on onboarding's last step and from Scan for skills in Library. No Discovery page or sidebar row.
- **G2** Build with GPUI Kit components first (Dialog, Checkbox, Radio, Button, Input, Avatar), styled through their public API. The prototypes are a design reference, not pixel specs (as in [scoped-pages](../scoped-pages/README.md)).
- **G3** Use the glossary's terms: import, found skill, in library, your agents, workspace, version. Never "Discovery" or "install".

## Library entry (variant B, "Inline count")

- **L1** Scan for skills is a ghost icon button in Library's header, after the update control, in the place Import holds today. Glyph: Hugeicons folder-search. Tooltip "Scan for skills".
- **L2** A quiet count follows the glyph inside the same button. It shows the number of found skills that are not in library, from the latest completed scan, counting a skill with several versions once. It uses the muted text color and tabular figures with thousands separators, like sidebar counts, and is not capped. With none, only the glyph shows. Skipped skills keep counting until they are imported.
- **L3** The button's accessible label includes the count: "Scan for skills, 22 not in library". It is unavailable while another operation runs.
- **L4** A library with no skills, and no folder or filter narrowing it, shows an empty state at the top left of the list frame: the folder-search glyph, "No skills in your library yet", and a primary "Scan for skills" button. Nothing opens by itself.

## Import skills dialog (variant C, "Grouped by place")

- **D1** Clicking Scan for skills opens the dialog at once, titled "Import skills", in its Scanning state ("Scanning…" with a spinner, in the same frame) while a fresh scan runs and the plan is prepared.
- **D2** The top row holds the search field ("Search found skills") on the left and the status line "N found · M already in library" on the right. The search filters rows by slug and description.
- **D3** Below it are a Select all row, then the groups. "Your agents" comes first, followed by each workspace in sidebar order. Each group header has a tri-state checkbox, the group's glyph (robot for your agents, code-folder for a workspace), its name, and its count. A skill found in several places is listed once, under the place of its first location. A group's checkbox ticks or unticks that group's importable skills.
- **D4** Each row has a checkbox, the slug, and the chosen version's description. A skill with one version shows its locations as "path · readers" and "N files · B bytes". A skill with several versions shows an "N versions" disclosure with a chevron, which opens one Kit Radio per version, labelled "Modified <relative time>". Invalid or Blocked follows the label, joined by " · ", and the version the lock recorded carries a "Matches source" tag. Validation messages follow. Each replacement reads "Replaces the copy in <place>." with a warning-colored alert glyph.
- **D5** Every importable skill whose chosen version is valid starts ticked, up to 500. Invalid skills start unticked. Blocked skills and skills with no files show a disabled checkbox.
- **D6** Select all ticks every importable skill, including invalid ones, as Discovery's did. When an invalid skill is ticked, the footer's left side shows the checkbox "Import skills with invalid metadata", and Import stays inactive until it is checked.
- **D7** While the search hides ticked rows, the Select all row adds "N selected not shown". Select all, the group checkboxes, and Import cover every ticked skill, hidden ones included.
- **D8** With more than 500 ticked, the footer's left side reads "Import at most 500 at a time." and Import is disabled.
- **D9** Choosing another version shows a brief busy state on that row while that skill alone is prepared again; the plan keeps its library revision and expiry (ADR 0009). Every tick is kept, and the row's replacements update.
- **D10** The footer holds Cancel and a primary "Import N skills", which is inactive at 0. Cancel, Escape, and the close button import nothing. Clicking outside the dialog does nothing.
- **D11** Import closes the dialog, and the app's blocking progress strip runs to its end, "Imported N skills". If Import is refused because the plan expired, the library changed after review, or a ticked skill's source changed, the dialog stays open and refreshes its rows with the same ticks. One line reads "The list changed since the scan. Check it and import again.", and the changed rows flash once. A change to an unticked skill never refuses the import.
- **D12** Nothing to import: "No new skills found." with Close, in the same frame. A failure: "Couldn't scan: <reason>" with Retry and Close.
- **D13** A background scan that completes while the dialog is open does not change it.

## Onboarding step (variant B, "Summary first")

- **O1** The step keeps the onboarding page: "SkillBinder", the title "Import your skills", and the lead "Importing copies these skills from your agents into your library, each staying active for the agents that have it. You can import others later with Scan for skills in Library."
- **O2** A summary card shows a stack of the found skills' agents (Kit Avatars, up to four) and "N skills found in <agents>", naming the agents in the Agents section's order.
- **O3** *Replaced by O7.* Under it, a "Review list" disclosure with a chevron expands the dialog's list inline: search, Select all, rows, versions, replacements, and the acknowledgement, with the same rules (D4 to D9). It is collapsed at first. Your agents form one place, so the list shows no group header. There is no status line, because a new library holds nothing.
- **O4** The buttons are Skip and a primary "Import N skills", where N counts the ticked skills. N can be smaller than the summary's count, because invalid, blocked, and empty skills start unticked; Review list shows which ones.
- **O5** States: Scanning (a spinner and "Scanning…" in place of the card); nothing found ("No skills to import were found in your agents." with Continue); error (one line with the reason, then Retry, with Skip still offered). Import runs the progress strip, then finishes onboarding into Library.
- **O6** *Removed with the source boundaries step (ADR 0010).* The source boundaries step reads "SkillBinder reads your agents' skills and your workspaces. Importing a skill copies it into your private library."
- **O7** Review list is a ghost button under the summary card that opens the Import skills dialog over onboarding (variant C's dialog), titled "Import skills", with no status line and no group header, since your agents are one place. Its footer holds the acknowledgement or the cap note on the left and a primary "Done" on the right; Done, Escape, and closing all keep the ticks and chosen versions and return to the step, whose "Import N skills" is the only button that imports. The user chose this on 2026-11-16 over the inline list, which stretched the onboarding column below the card.

## Rejected variants

- Import dialog A "Checklist" (one flat list) and B "Two panes" (a details pane for the focused row).
- Library entry A "Corner badge" (a Kit Badge on the button's corner, capped at 99+, with the empty state centered).
- Onboarding A "Inline list" (the full list always shown). C "Dialog over onboarding" was first rejected, then adopted for Review list (O7).
