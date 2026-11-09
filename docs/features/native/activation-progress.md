# Activation progress

Three changes write and remove copies in every affected skill directory, one directory at a time, in a single engine job: activating a preset, deleting the active preset, which activates the Default preset, and applying an import (ADR 0007). While one runs, the app is blocked and a progress strip shows how far it has got. ADR 0006's rules for copies are unchanged: the choice is saved first, the copies follow it, and nothing rolls back. The look follows the approved prototype pixel for pixel; the values are in `docs/design/scoped-pages/spec/progress-1-states.md` to `progress-5-gpui.md`.

## Progress

Progress is the share of the (directory, skill) pairs the change brings in line. The engine keeps one progress record beside the scan and source-check records, so the client reads it without waiting for the worker. The record is cleared when the change is called, before it is queued, and the change's job counts into it: each pass adds the pairs it plans to the total, then counts each pair it brings in line, and how many of those leave a copy that needs the user. A copy needs the user when it conflicts, was edited in place, could not be written, or could not be removed; one the user chose to keep does not. Every pass of the job counts, so an import counts its activation pass and, once per device, its adoption pass. Passes in other jobs never touch the record: the pass before each scan, the rewrite after Update, Reinstall, Pull, Sync and the launch catch-up, workspace changes, and Retry. The record also names the pair the pass is on, which ends when the job does.

While the change runs, the client publishes the record as the snapshot's `activation` every 100 milliseconds, and `None` once it returns, whatever its result. A change faster than one interval returns at once. The change's result comes back with its final counts. The scan that follows a change runs in the background and never holds the progress open.

The percentage is done × 100 / total, rounded to the nearest percent, from `0%` to `100%`; it reads `0%` until the total is known. The strip's percentage never goes down: a refresh that publishes no progress while the change runs keeps the last one. Once the change lands, the strip holds at `100%` while it finishes its database save and refresh.

## While it runs

Input is blocked from the click: a transparent layer over the whole window takes every pointer event, keyboard focus moves to it, and Tab cycles inside it. ⌘B, ⌘[, ⌘] and the menu bar's Toggle Sidebar, Back and Forward do nothing, and Escape is ignored. There is no close button and no cancel. Quitting still works, and the rest of the change finishes at the next launch, like any partly applied change.

The strip shows only after 300 milliseconds, so a short change does not flash: the whole window, sidebar included, takes a wash of the page color at 70%, and a pill centered on the window 10 points below its top edge shows a spinner turning every 0.9 seconds, the label, the percentage in the muted color with tabular figures, and a 2-point line in the primary color along its bottom edge. The label reads `Switching…` for a preset switch and for deleting the active preset, and `Importing…` for an import. The pill's tooltip names the change and its step, such as "Switching to Frontend" over "Copying tailwind-helper → Cursor" or "Removing release-notes from Claude Code". Once shown, the strip stays at least 500 milliseconds.

Some changes keep their own behavior and show no strip, but are refused while it is up: a Library toggle or agents menu, a workspace toggle, Retry, Replace and Keep, Sync, Update, Update all, Reinstall, and adding a workspace or choosing its image. Update all keeps its progress ring in the Library header.

## The end

- **Nothing needs the user.** The strip goes away by itself and a toast at the bottom center of the window confirms the change: `Switched to <preset>`, `Deleted <preset> · Switched to Default`, or `Imported <n> skills` (`Imported 1 skill`). A change that ended before the strip showed still shows its toast, and a change that wrote nothing shows only the toast. The toast is an inverted chip with the check-circle glyph; it closes after 4 seconds, the pointer resting on it keeps it, and it closes 1.5 seconds after the pointer leaves. A newer toast replaces it.
- **Copies need the user.** The pill appears at once if it had not, and reads `Done · 1 needs attention` or `Done · <n> need attention` after the warning glyph, with Review and Close. The wash stays, and the content behind it shows the new state. Input stays blocked except for Review, Close and Escape, which acts as Close. Review opens the review list; Close closes the pill. Either way the copies that ask count as asked, so the Skill copies dialog does not open by itself right after; each skill's warning glyph and the Review on its page still list them. While the strip is up, copies that need the user wait for its end instead of opening Skill copies halfway.
- **A refused change**, such as "The library changed after the deletion preview.", closes the strip after its minimum time with no toast, and the error shows in the error box. A refused switch shows the previous preset as active again.

## Review list

Review opens a dialog titled `<n> need attention`, 420 points wide and 56 points from the window's top edge, over a scrim. It lists every copy that needed the user when it opened, in a card: the slug, then `→ <place>`, where the place is the agents that read the directory or `<workspace> · <agents>`, and under them the reason after the warning glyph with two answers. A different copy reads "Different copy there" with Replace and Keep; an edited copy "You changed it here" with Replace and Keep, or Remove and Keep when the skill is no longer active there; a directory that cannot be written "Can't write the folder" with Retry and Skip. An answer takes effect as in the Skill copies dialog; once the copy no longer needs the user, the row ends in a muted check and `Replaced`, `Removed`, `Kept`, `Done` or `Skipped`. Skip only settles the row. An answer that does not take effect, such as a failed retry, offers the answers again. Done, the close button, Escape and a click on the scrim close it.

## Hand-over from dialogs

When Delete preset deletes the active preset, it closes its confirmation and the strip takes over. Deleting a preset that is not active writes nothing and keeps the confirmation's own flow. Import closes Review import, and the strip takes over; the import's result banner still reports it until dismissed.

## Native test controls

The `native-test` build advertises the MCP app command `hold_activation` so a parity capture can wait for a state that real timing passes too fast to catch. `{"strip": "hidden"}` keeps a strip that starts or is up unrevealed, showing only the input blocker; `{"strip": <percent>}` shows that percentage once the strip reveals; a held strip never ends, and `{"strip": null}` releases it, ending one whose change has landed. `{"toast": true}` keeps the end toast up until a call with `false`. Ordinary builds have no such control.
