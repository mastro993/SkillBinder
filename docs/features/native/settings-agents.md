# Settings, the new agent notice, and the app update notice

After onboarding, the user changes their agents in Settings, and is asked about agents found later (ADR 0012, design S1, S2, N1 to N5, P4, P5). The approved prototypes are `docs/design/onboarding/prototypes/settings.html` (variant A "Switch list") and `notice.html` (variant A "Corner card"). Settings also shows the version and the app update channel, and a notice offers a newer SkillBinder ([app updates spec](../../specs/app-updates.md) rules 1–5, 9, 13, 15–19, 24; prototype `docs/design/app-updates/prototypes/settings-updates.html`, variant A "Inline").

## Settings › Agents

Settings opens with an "Agents" section header (the design system's `SectionHeader`, `caption-sm` muted) above "Application", then a bordered list with one row per Agents row of every detected agent, chosen or not: Universal first, then each other agent alphabetically. An agent whose global skill directory does not exist is not listed. Each row has the agent's Kit avatar (Universal's layers glyph), its name, its global skill directory in muted monospace or, for Universal, its agents' names with a tooltip, and a trailing Kit switch, on while the row's directory is chosen. With no agent found the list reads "No agents found on this computer." Click, Enter, or Space toggles a switch.

- **On** applies the active preset to the agents reading that directory under the blocking progress strip, `Adding…`, whose tooltip reads "Adding skills to <agent>" ([Activation progress](activation-progress.md)). Copies with the library's content are adopted and others ask, as any change that writes copies does ([Activation](activation.md)). Every switch is disabled, and the notice taken down, until the strip ends, in the toast "<Agent> is on", or in "Done · <N> need attention".
- **Off** saves at once, without the strip. The agents' copies stay in place, unmanaged: SkillBinder no longer updates or removes them, and later preset changes leave them alone. Until the app quits, the row reads "Its skills stay in place." in place of its folder line; an agent that was never on keeps its folder line.

Either answer counts the directory as asked, so the notice never asks about it.

## Settings › Application

Under the "Application" section header, the design system's `KeyValueList` lists the application's facts in a flat card, each label in a 128-point `caption` muted column and rows divided by a hairline: Managed library (its path in monospace), Git (its version, or the Git row above), Storage writable (Yes or No), and Remote (its URL, or "Local only"). Reveal logs follows the card. The Sync page lists its facts the same way: Status (the state, ahead, behind, and local changes), and with a remote its URL in monospace, Branch, and Last checked.

## Settings › Updates

Under the "Updates" section header, after Reveal logs, a `KeyValueList` lists:

- **Version**: the running version, such as `0.1.1-beta.12`, in tabular figures.
- **Channel**: Kit's segmented control, Stable | Beta, starting on the build's own channel (Beta for a beta build) and saved per device. While a beta build follows Stable, "You'll move to Stable at its first release after <version>." sits under it.
- **Check automatically**: a Kit switch, on by default and saved per device; not shown in a build without the release stamp.
- **Status**: one line, with its action at the row's end.

| State | Status line | Action |
|---|---|---|
| Unstamped build | Updates are off in this build. | none |
| Up to date | Up to date · checked 5 minutes ago, or Not checked yet | Check now |
| Checking | Kit's spinner, Checking… | Check now, disabled |
| Downloading | Downloading 0.2.0… 42% | Check now, disabled |
| Ready | SkillBinder 0.2.0 is ready. | the primary Restart, disabled with its tooltip while long work runs, and the ghost What's new, which opens the release page |
| Download-only | SkillBinder 0.2.0 is available., with the reason muted under it: "Move SkillBinder to Applications to update automatically." (macOS outside Applications), "SkillBinder can't write to its folder." (an Applications folder the user can't write), "This copy wasn't installed by the SkillBinder installer." (a Windows copy setup did not install), or "Download it from the release page." (Linux and development builds) | Download and the ghost What's new, both opening the release page |
| Check now failed, offline | Couldn't check for updates. Check your connection. | Check now |
| Check now failed, otherwise | Couldn't check for updates. | Check now |

Check now always checks. While Check automatically is on, the app also checks at launch, every hour, and right after the channel changes, and turning the switch on checks at once. A failed automatic check changes nothing, and a refused signature is logged. Switching channel drops a version found on the other one, and when it was checked. On macOS from Applications a newer version downloads and installs at quit ([App updates in place](app-updates.md)); elsewhere it is download-only.

## The app update notice

When a check finds a newer version this copy can't install itself, a Kit notification appears at the window's bottom-right, with its own id and no auto-hide, stacking with the new agent notice: "SkillBinder 0.2.0 is available" in medium weight, the download-only reason from the Status table muted, then Dismiss and the primary Download, which opens the release page. A ready update shows "SkillBinder 0.2.0 is ready" with Restart instead ([App updates in place](app-updates.md#the-ready-notice)). Like the new agent notice, it never shows during onboarding and is taken down while the progress strip is up, returning once it ends. Dismiss, Download, and Kit's close button hide it until the next launch; Settings still shows the version. A newer version found later replaces the shown notice.

## The new agent notice

A detected agent whose global skill directory is neither chosen nor asked about waits for the notice. New agent folders are found when the window regains focus, which refreshes the app; no watcher runs. The notice never shows during onboarding or while the progress strip is up: it is taken down when a strip starts and shown again once it ends.

It is a Kit notification at the window's bottom-right with its own id and no auto-hide, while change toasts keep the bottom center. One notice lists every waiting Agents row: their avatars in a stack, then "Found <a>.", "Found <a> and <b>.", or "Found <a>, <b>, and <N> more." in medium weight, then "Let SkillBinder manage its skills?" (their skills for several) in muted text, and right-aligned buttons. One agent gets Not now and the primary Manage; several get Review… and the primary Manage all, with a close button at the end of the logo row. Kit's own close button also shows under the pointer. It takes no focus and does not close on Escape. A longer list replaces the shown notice.

- **Manage** and **Manage all** turn the agents on as the Settings switch does, ending in "<Agent> is on", "<a> and <b> are on", or "<N> agents are on".
- **Not now**, the close buttons, and **Review…** leave them off. Review… also opens Settings, where they show off.

Every answer counts every listed directory as asked, saved at once, so the notice never returns for them, across restarts too. A button pressed while another change is in flight does nothing and the notice stays.

## Engine

`set_agents_chosen(dirs, chosen)` takes global skill directories as the app shows them, refuses before onboarding finished and for a directory no detected agent has, and adds every one to `asked` in the `agents` device setting. On saves the choice through `change_places`, which runs the pass for each skill whose directories changed. Off saves the choice and, in the same transaction, forgets the copy records of every directory your agents no longer read; a missing workspace's directories keep theirs. Not now is off for directories that are off already, so it only records the answer.

A workspace skill directory stays in use while any chosen agent reads it, as its registry directory or one the reviewed overlay records, such as `.claude/skills` for Cursor after Claude Code is turned off. A directory a workspace uses that no chosen agent reads is idle: it stays in the workspace's list, but nothing is written or removed there and its records are not shown or acted on, until such an agent is chosen again ([Workspace activation](workspace-activation.md)).
