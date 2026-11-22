# Command line tool

ADRs: [0022](../adr/0022-command-line-tool-talks-to-the-running-app.md) (talks to the running app), [0023](../adr/0023-command-line-tool-covers-domain-actions.md) (what it covers, how it asks), [0024](../adr/0024-install-the-command-from-settings.md) (installing it from Settings). It builds on [0006](../adr/0006-activation-writes-managed-copies.md) (confirming replacements), [0007](../adr/0007-pages-list-active-skills.md) (blocking changes), [0008](../adr/0008-import-from-scan-dialog.md) (import), [0015](../adr/0015-install-into-library-only.md) (install), [0019](../adr/0019-classic-install-per-platform.md), [0020](../adr/0020-stay-running-in-the-menu-bar.md), and [0021](../adr/0021-tray-through-tray-icon-and-ksni.md) (the menu bar's dispatch). Glossary: [domain-app.md](../agents/domain-app.md) (command line tool) and [domain.md](../agents/domain.md) (preset, workspace, managed copy, install, import).

Agents get the same actions as MCP tools through `skillbinder mcp` ([MCP server](mcp-server.md)). Companion files: [Design](command-line/design.md) (architecture, protocol, risks), [Acceptance and slices](command-line/acceptance.md), and [Onboarding steps](command-line/onboarding.md) (ADR 0027).

## Problem

Every SkillBinder action needs the window or the menu bar menu. A user working in a terminal, a script, or a coding agent, the very thing SkillBinder manages skills for, cannot list the library, switch the active preset, install a skill from the catalog and turn it on in a project, or run Update all. An agent told "install the pdf skill and turn it on here" has no way to do it except writing into `.claude/skills` behind SkillBinder's back, which is what managed copies exist to prevent (ADR 0006).

## Outcome

From any terminal, after installing the command once from Settings, `skillbinder <command>` runs any domain action the app offers, under the same rules, and an open window or menu shows the result as it lands. An agent can drive it non-interactively with `--json` and stable exit codes, and it never overwrites the user's own copies unless told to.

## Scope

In:
- the `skillbinder` console program on macOS, Windows, and Linux, talking to the running app and starting it in the background when needed;
- commands for reading the library, presets, library folders, the active preset's skills, workspaces, your agents, managed copies waiting for an answer, scan and import, Explore's search and install, your repositories, skill updates, Reinstall, Sync, Push, Pull, and the sync remote;
- Settings › General › Command line tool: status, Install, Repair, Uninstall;
- the Homebrew casks linking the command.

Out:
- preset and workspace colors, icons, and images, reordering, theme, sidebar, and every other appearance choice;
- onboarding, the app's own settings (menu bar switch, app update channel, automatic checks), app updates, Restart, and Move to Applications;
- deleting a skill from the library (not in the app either, ADR 0007);
- running without the app or without a desktop session (SSH into a machine with no display);
- the SkillBinder skill, which teaches agents the tools ([onboarding.md](command-line/onboarding.md#the-skillbinder-skill), ADR 0028);
- shell completions and man pages.

## Decisions

1. **D1** The command covers the domain actions, not appearance or ordering (ADR 0023).
2. **D2** When the app is not running, the command starts it in the background. With its icon showing it stays running afterwards; with no icon it quits once no command needs it (ADR 0022).
3. **D3** It serves people and agents: readable text by default, `--json` on every command, stable exit codes, no prompts without a terminal (ADR 0023).
4. **D4** Replacing a copy SkillBinder does not manage asks in a terminal; elsewhere only `--replace` confirms it, and without it the copies wait and the command exits 3 (ADR 0023).
5. **D5** On macOS, Install links `/usr/local/bin/skillbinder`, through the administrator prompt when needed (ADR 0024).
6. **D6** The Homebrew casks link the command themselves; Settings reads "Installed by Homebrew" (ADR 0024).
7. **D7** ~~A skill that teaches agents the command is a separate, later feature.~~ Replaced by D16: the SkillBinder skill (ADR 0028).

## Behavior

### Commands

Names follow the glossary. A skill is named by its slug, a preset or library folder by its name, a workspace by its name or by a path inside it (`.` is the workspace holding the current folder), and an agent by its registry identity (`claude-code`), its name, or its global skill directory. Names compare ignoring case; an ambiguous or unknown name fails with exit 2 and lists the close matches.

| Command | Does what the app does in |
| --- | --- |
| `status` | Version, whether onboarding is finished, the active preset, counts of library skills, active skills, updates, copies needing attention, Sync state, and what is running |
| `skills [--active \| --inactive \| --updates] [--folder F]` | Library's list and filters: each skill with where it is active and its marks (update, drifted, New) |
| `skill <slug>` | A skill's page: description, source, record, presets, where active, copies |
| `enable <slug>… [--agent A]… [--replace]`, `disable <slug>… [--agent A]…` | Turning a skill on or off for your agents, which edits the active preset (ADR 0007) |
| `presets`, `preset create \| rename \| clone \| delete [--yes]` | The sidebar's presets |
| `preset activate <name> [--replace]` | Clicking a preset |
| `preset add <name> <slug>… [--agent A]…`, `preset remove <name> <slug>…` | Editing a preset that may not be active |
| `folders`, `folder create \| rename \| delete [--yes]`, `folder move <slug>… (--to F \| --none)` | Library's folder tabs and filing |
| `workspaces`, `workspace add <path> [--replace]`, `workspace rename`, `workspace remove [--yes]` | The sidebar's workspaces |
| `workspace enable <ws> <slug>… [--agent A]… [--replace]`, `workspace disable <ws> <slug>…` | A workspace's page and its Add skill picker |
| `agents`, `agent on <agent> [--replace]`, `agent off <agent>` | Settings › Agents |
| `copies`, `copies replace \| keep <slug> [--in DIR]`, `copies retry` | The review list and Retry |
| `scan`, `import (<slug>[@<version>]… \| --all) [--replace]` | Scan for skills and the Import skills dialog |
| `search <query>` | Explore's search over the catalog, your repositories, and skills.sh |
| `install <ref>…` | Explore's Install. `<ref>` is a slug found once in the catalog or your repositories, `<owner>/<repo>/<slug>`, or a skills.sh `<owner>/<repo>@<skill>` |
| `repos`, `repo add <url>`, `repo remove <url>`, `repo install-all <url>` | Your repositories and Install all |
| `updates [--check]`, `update (<slug>… \| --all)`, `reinstall <slug>` | Library's Updates filter, Check now, Update, Update all, Reinstall |
| `sync`, `push`, `pull`, `remote set <url> [--branch B]`, `remote remove` | Sync and its remote setup |

`skillbinder --version` prints the command's version; `skillbinder --help` and `<command> --help` describe each command with an example.

### Rules

1. Every command runs the operation the app runs, under the same rules. A change that blocks the app (ADR 0007) shows the blocking strip in an open window and the menu's progress, and the command shows its progress until it ends.
2. While something the window would refuse runs (a blocking change, Update all, Sync, an import), a change refuses at once with exit 4, naming what runs: "SkillBinder is busy: Switching to Work… Try again when it finishes." Reading commands always work.
3. Ctrl-C stops waiting, not the work: the command prints "Still running in SkillBinder." and exits 130; the app finishes the change as if its window had closed.
4. Before onboarding finishes, every command but `status` and `--version` exits 5 with "Open SkillBinder to finish setting up."
5. A command that needs Git without Git fails with the app's own copy, "Sync and skill updates need Git 2.39 or newer.", exit 1.
6. **Copies.** A change that meets same-slug copies SkillBinder does not manage writes everything else, then:
   - in a terminal, asks once per copy, "Replace your copy of pdf in ~/.claude/skills? [y/N]", and replaces or keeps it as answered (no answer keeps it waiting);
   - with `--replace`, replaces them all, without asking;
   - otherwise leaves them waiting under "need attention", prints them with "Run again with --replace, or review them in SkillBinder.", and exits 3.
   An edited managed copy is handled the same way. `copies` lists everything waiting, from any surface.
7. **Import** states its replacements before it runs, as the dialog does: in a terminal it lists them and asks "Import 3 skills and replace 2 copies? [y/N]"; with `--replace` it proceeds; otherwise it changes nothing and exits 3 listing them. A slug with several versions needs `@<version>` (the short digest `status` and `scan` print) unless `--all`, which picks what the dialog ticks.
8. **Deleting** a preset or library folder and removing a workspace ask in a terminal, with the app's confirmation text (a deleted active preset names the skills the switch to Default adds and removes), and need `--yes` otherwise; without it they exit 3.
9. **Output.** Text goes to stdout for results and stderr for progress and prompts, without color when stdout is not a terminal or `NO_COLOR` is set. `--json` prints exactly one JSON document on stdout in one of three shapes: `{"ok": true, "result": …}`; `{"ok": false, "error": {"code", "message", "hint"}}` when nothing changed; or `{"ok": false, "error": …, "result": …}` when a change partly landed, as with copies left waiting (exit 3), where `result` holds what changed and what waits. Progress goes as text on stderr only in a terminal.
10. **Exit codes.** 0 done; 1 failed; 2 usage or unknown name; 3 waiting for the user's answer; 4 SkillBinder busy; 5 SkillBinder unavailable or not set up; 130 interrupted.
11. **Starting the app.** With no app answering, the command prints "Starting SkillBinder…" on stderr, starts it in the background, and waits up to 20 seconds. If it cannot start (no desktop session, app missing after a move), it exits 5 naming why. A background app shows no window and, on macOS, no Dock icon. With its icon on it keeps running; with none it quits once no command is connected or running.
12. A second launch of the app opens the running app's window, as today, now over the same socket.

### Settings › General › Command line tool

One row, after Show in menu bar, built by the Settings control rules (an action is a button, a fact is muted text). Title **Command line tool**, line by state:

| State | Line | Control |
| --- | --- | --- |
| Not installed | "Run SkillBinder from a terminal as `skillbinder`." | Install |
| Installed | "`skillbinder` in /usr/local/bin" (the folder it is in) | Uninstall |
| Installed by Homebrew (macOS) | "`skillbinder` in /opt/homebrew/bin" | muted "Homebrew" |
| Leads to another SkillBinder | "`skillbinder` opens another copy of SkillBinder." | Repair |
| Not on PATH (Linux) | "Add `~/.local/bin` to your PATH: `export PATH=\"$HOME/.local/bin:$PATH\"`" | Uninstall |
| Something else in the way | "Another `skillbinder` is in /usr/local/bin." | none |
| Can't install here (macOS outside Applications, Windows copy the setup program didn't install) | "Move SkillBinder to Applications to install it." or "Install SkillBinder with its setup program to add it." | none |

- Install and Repair on macOS ask for the administrator password only when `/usr/local/bin` is not writable; cancelling changes nothing. On Windows they add the `bin` folder to the user's PATH, and the toast says "Open a new terminal to use `skillbinder`." Elsewhere the toast says "`skillbinder` is installed."
- Uninstall removes only what Install wrote. A failure shows the error in a toast and leaves the row as it was.
- The row re-reads its state when Settings opens and after each action. Search finds it under "command", "terminal", "CLI", and "PATH".

## UX

The only screen touched is Settings' General section, one row following the approved Settings page ([prototype](../design/settings/prototypes/settings.html), variant B) and its control rules; no new prototype is needed. The terminal output follows the examples in [design.md](command-line/design.md#output).

## Assumptions

- **A1** Commands, flags, and reference forms are as listed above. If wrong: rename before S2 ships; after it, names are a contract.
- **A2** Busy refuses rather than queues (rule 2). If wrong: add `--wait` later without breaking anything.
- **A3** Ctrl-C leaves the work running (rule 3), matching closing the window. If wrong: cancellation would need engine support the app lacks today.
- **A4** Only `status` works before onboarding (rule 4). If wrong: onboarding commands are their own design.
- **A5** `--yes` covers deletions and `--replace` covers copies, kept separate so a script that deletes a preset never replaces copies by accident.
- **A6** The app's own settings stay out (Scope). If wrong: they fit as a `settings` command later.
- **A7** The command is never put on the PATH without Install, except by Homebrew (ADR 0024).

## Open questions

None.
