# Settings, the new agent notice, and the app update notice

After onboarding, the user changes their agents in Settings, and is asked about agents found later (ADR 0012, design S1, S2, N1 to N5, P4, P5). The approved prototypes are `docs/design/onboarding/prototypes/settings.html` (variant A "Switch list") and `notice.html` (variant A "Corner card"). Settings also shows the version and the app update channel, and a notice offers a newer SkillBinder ([app updates spec](../../specs/app-updates.md) rules 1–5, 9, 13, 15–19, 24; prototype `docs/design/app-updates/prototypes/settings-updates.html`, variant A "Inline"). The Settings page itself follows `docs/design/settings/prototypes/header-tabs.html` (variant B "Search in header"); the older prototypes still guide their rows' copy and states.

## Settings page

Settings uses the header the way Library does (design `docs/design/settings/prototypes/header-tabs.html`, variant B "Search in header", chosen 2026-11-26). The header's title slot holds tabs styled like Library's folder tabs: **All**, a separator, then General, Agents, Library & sync, Updates, and Troubleshooting, scrolling sideways when they overflow. The search field, "Search settings", sits at the header's end at the scoped pages' search width. There is no toolbar: the page scrolls whole under the header.

A section's tab shows only that section's groups. All shows every section in order, each under a small muted label of its glyph and name. A group's `SectionHeader` title, when it has one, sits over its card of rows; the agents' switches carry the line "SkillBinder adds and removes skills only in the agents turned on.".

Settings opens on All with an empty search every time, and choosing a tab is not added to Back/Forward history. Review… on the new agent notice opens it on the Agents tab.

Every row reads the same way: an optional logo, the setting's title over a muted line, and one control at the row's end, chosen by what the setting is:

| Setting | Control |
|---|---|
| On or off | Kit switch |
| One of two | Kit's segmented control |
| One of three or more | a select: a button reading the choice with a chevron, opening the app menu that checks it |
| An action | a small button |
| A fact the user can't change | muted `body-sm` text in tabular figures |

Typing in the search keeps the rows whose title, line, or keywords hold the text, ignoring case, or every row of a group or section whose title holds it, such as "Sync" or "Updates". The search covers every section whatever tab is chosen: groups and sections left with no row hide, and their tabs fade and are disabled. The chosen tab stays chosen while its section has a match; while it has none, All shows, under "No match in <section> · showing all", and clearing the search returns to the chosen tab. With nothing left the page shows "No settings found" and "Nothing matches “<text>”.". Leaving Settings clears the search and returns to All.

## Settings › General

**Theme** is a select of System, Light, and Dark, the preference the sidebar's appearance toggle also sets ([Application shell](application-shell-continued.md)). Choosing one applies it at once and saves it on this device.

**Show in menu bar** (macOS), or **Show in system tray** (Windows and Linux), is a switch, on by default: closing the window keeps SkillBinder running behind its icon ([Menu bar menu](menu-bar.md)).

**Command line tool** shows whether `skillbinder` works in a new terminal, with Install, Repair, Uninstall, or the fact that Homebrew installed it at its end ([Command line](command-line.md#installing-it)).

## Settings › Agents

The Agents section lists one row per Agents row of every detected agent, chosen or not: Universal first, then each other agent alphabetically. An agent whose global skill directory does not exist is not listed. Each row has the agent's Kit avatar (Universal's layers glyph), its name, its global skill directory in muted monospace or, for Universal, its agents' names with a tooltip, and a trailing Kit switch, on while the row's directory is chosen. With no agent found the section reads "No agents found on this computer." Click, Enter, or Space toggles a switch.

- **On** applies the active preset to the agents reading that directory under the blocking progress strip, `Adding…`, whose tooltip reads "Adding skills to <agent>" ([Activation progress](activation-progress.md)). Copies with the library's content are adopted and others ask, as any change that writes copies does ([Activation](activation.md)). Every switch is disabled, and the notice taken down, until the strip ends, in the toast "<Agent> is on", or in "Done · <N> need attention".
- **Off** saves at once, without the strip. The agents' copies stay in place, unmanaged: SkillBinder no longer updates or removes them, and later preset changes leave them alone. Until the app quits, the row reads "Its skills stay in place." in place of its folder line; an agent that was never on keeps its folder line.

Either answer counts the directory as asked, so the notice never asks about it.

### MCP server

After the switches, the **MCP server** group connects agents to SkillBinder's [MCP server](mcp-server.md) through their own installers (ADR 0026), described "Let your agents use SkillBinder's tools.", which adds "Connecting also installs the skillbinder command." while the command is not installed (ADR 0027). It shows once the command line tool's state is read, as Settings opens.

- Where the command can't be installed, the group shows only why, with no Connect and no Copy configuration: "Move SkillBinder to Applications to connect agents.", "Install SkillBinder with its setup program to connect agents.", "Run SkillBinder as its AppImage to connect agents.", or "Another skillbinder is in <folder>.".
- Otherwise a row per agent on this computer, in this order: **Claude Code**, **Codex**, **VS Code**, and **Cursor**, each with its logo (VS Code's square terminal glyph), its name, and "Connected", "Not connected", or "<tool> not found" (`claude`, `codex`, or `code`). An agent is on this computer when its folder exists (`~/.claude`, `$CODEX_HOME` or `~/.codex`, VS Code's user settings folder, `~/.cursor`) or its program is found. The control is **Connect**, **Disconnect** for Claude Code and Codex, or for VS Code and Cursor when connected the muted "Remove it in VS Code" or "Remove it in Cursor".
- Last, **Other agents**, "Add SkillBinder to any agent's MCP settings.", with **Copy configuration**, which copies `{"mcpServers": {"skillbinder": {"command": "<command>", "args": ["mcp"]}}}` with the command's absolute path and toasts "Configuration copied". While the command is not installed, it installs it first, as Connect does, so the copied path exists.

Onboarding's Use SkillBinder with your agents step runs the same Connect for the agents it ticks ([Onboarding](onboarding.md)), so a user who skipped it, or finished onboarding before it existed, connects agents here. Connect installs the command first when it is missing, with macOS's password prompt where needed; cancelling it changes nothing and shows no toast. Then it runs the agent's installer with the command's path and toasts "<Agent> can use SkillBinder. Start a new <Agent> session to load it.", or for Cursor opens its install link, where Cursor asks the user. Disconnect runs the agent's remover. A failing installer toasts the last line of its error output, or that it could not start or took too long. While one runs, every Connect, Disconnect, and Copy configuration is inactive, and so are the Command line tool row's Install, Repair, and Uninstall, which in turn keep the group's controls inactive while they run, so two installs never overlap. The rows are read when Settings opens, after each action, and when the window regains focus on Settings, so Cursor's row reads Connected once the user accepts. Searching "MCP", "tools", or an agent's name finds the rows.

## Settings › Library & sync

The Library group lists **Library folder**, its path in monospace with Reveal, which shows it in the file manager, and **Storage**, "Writable" or "Read-only". The Sync group lists **Git**, its version, or "Sync and skill updates need Git 2.39 or newer." with the Git step's primary action, its status line (which names a too-old version), and on Linux the package manager's command under it, and **Remote**, its URL or "Local only", set up in Sync. The Sync page lists its own facts in the design system's `KeyValueList`: Status (the state, ahead, behind, and local changes), and with a remote its URL in monospace, Branch, and Last checked.

## Settings › Updates

The Updates section lists:

- **Version**: the running version, such as `0.1.1-beta.12`, in tabular figures.
- **Channel**: "Beta gets new versions first.", and Kit's segmented control, Stable | Beta, starting on the build's own channel (Beta for a beta build) and saved per device. While a beta build follows Stable, "You'll move to Stable at its first release after <version>." sits under the row's line.
- **Check automatically**: "At launch and every hour.", and a Kit switch, on by default and saved per device; not shown in a build without the release stamp.
- **Status**: the status line under the title, any reason muted under it, and its action at the row's end.

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

## Settings › Troubleshooting

**Logs** has Reveal logs, which shows the logs folder in the file manager. Its `skillbinder.log` holds the engine's, the app's, and GPUI's and Kit's lines ([Sync and durability](sync-and-durability.md)).

## The app update notice

When a check finds a newer version this copy can't install itself, a Kit notification appears at the window's bottom-right, with its own id and no auto-hide, stacking with the new agent notice: "SkillBinder 0.2.0 is available" in medium weight, the download-only reason from the Status table muted, then Dismiss and the primary Download, which opens the release page. A ready update shows "SkillBinder 0.2.0 is ready" with Restart instead ([App updates in place](app-updates.md#the-ready-notice)). Like the new agent notice, it never shows during onboarding and is taken down while the progress strip is up, returning once it ends. Dismiss, Download, and Kit's close button hide it until the next launch; Settings still shows the version. A newer version found later replaces the shown notice.

## The new agent notice

A detected agent whose global skill directory is neither chosen nor asked about waits for the notice. New agent folders are found when the window regains focus, which refreshes the app; no watcher runs. The notice never shows during onboarding or while the progress strip is up: it is taken down when a strip starts and shown again once it ends.

It is a Kit notification at the window's bottom-right with its own id and no auto-hide, stacked with the change toasts. One notice lists every waiting Agents row: their avatars in a stack, then "Found <a>.", "Found <a> and <b>.", or "Found <a>, <b>, and <N> more." in medium weight, then "Let SkillBinder manage its skills?" (their skills for several) in muted text, and right-aligned buttons. One agent gets Not now and the primary Manage; several get Review… and the primary Manage all, with a close button at the end of the logo row. Kit's own close button also shows under the pointer. It takes no focus and does not close on Escape. A longer list replaces the shown notice.

- **Manage** and **Manage all** turn the agents on as the Settings switch does, ending in "<Agent> is on", "<a> and <b> are on", or "<N> agents are on".
- **Not now**, the close buttons, and **Review…** leave them off. Review… also opens Settings on its Agents tab, where they show off.

Every answer counts every listed directory as asked, saved at once, so the notice never returns for them, across restarts too. A button pressed while another change is in flight does nothing and the notice stays.

## Engine

`set_agents_chosen(dirs, chosen)` takes global skill directories as the app shows them, refuses before onboarding finished and for a directory no detected agent has, and adds every one to `asked` in the `agents` device setting. On saves the choice through `change_places`, which runs the pass for each skill whose directories changed. Off saves the choice and, in the same transaction, forgets the copy records of every directory your agents no longer read; a missing workspace's directories keep theirs. Not now is off for directories that are off already, so it only records the answer.

A workspace skill directory stays in use while any chosen agent reads it, as its registry directory or one the reviewed overlay records, such as `.claude/skills` for Cursor after Claude Code is turned off. A directory a workspace uses that no chosen agent reads is idle: it stays in the workspace's list, but nothing is written or removed there and its records are not shown or acted on, until such an agent is chosen again ([Workspace activation](workspace-activation.md)).
