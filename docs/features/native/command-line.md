# Command line

The `skillbinder` command runs from a terminal and talks to the running app; it never opens the library itself ([spec](../../specs/command-line.md); ADR 0022, 0023). The architecture is in [Native command line](../../architecture/native-command-line.md). Slice S0 (#199) ships the path every later command uses: `skillbinder status`, `--version`, `--help`, background start, and the second launch over the same socket. Slice S1 (#200) puts the command on the PATH from Settings › General. Slice S2 (#201) adds the library, presets, library folders, and copies, slice S3 (#202) workspaces and your agents, slice S4 (#203) scan and import, Explore's search and install, and your repositories, and slice S5 (#204) skill updates, Reinstall, and Sync. What each slice was checked with is in [Command line evidence](command-line-evidence.md). `skillbinder mcp` serves the same actions to agents as MCP tools ([MCP server](mcp-server.md)).

## Where it is

Each package holds it beside the app: `SkillBinder.app/Contents/MacOS/skillbinder-cli` on macOS, `{app}\bin\skillbinder.exe` on Windows, and `usr/bin/skillbinder-cli` inside the AppImage on Linux.

## Installing it

Nothing puts the command on the PATH until you press Install in Settings › General › **Command line tool**, except the Homebrew casks (ADR 0024). The row reads its state when Settings opens and after each action; search finds it under "command", "terminal", "CLI", and "PATH".

| State | Line | Control |
| --- | --- | --- |
| Not installed | "Run SkillBinder from a terminal as skillbinder." | Install |
| Installed | "skillbinder in /usr/local/bin" (the folder it is in) | Uninstall |
| Installed by Homebrew (macOS) | "skillbinder in /opt/homebrew/bin" | muted "Homebrew" |
| Leads to another SkillBinder | "skillbinder opens another copy of SkillBinder." | Repair |
| Not on PATH (Linux) | "Add ~/.local/bin to your PATH:" over the line `export PATH="$HOME/.local/bin:$PATH"` to copy | Uninstall |
| Something else in the way | "Another skillbinder is in /usr/local/bin." | none |
| Can't install here | "Move SkillBinder to Applications to install it." (macOS), "Install SkillBinder with its setup program to add it." (Windows), "Run SkillBinder as its AppImage to add it." (Linux) | none |

- **macOS.** Install links `/usr/local/bin/skillbinder` to the bundle's `skillbinder-cli`, as the user when the folder is writable and otherwise through the administrator prompt, which also creates the folder with mode 755. Cancelling the prompt changes nothing and shows no toast. It is offered only while the app runs from `/Applications` or `~/Applications`. A link into another `SkillBinder.app`, as after moving the app, offers Repair. The casks link the command with their `binary` stanza; a link at `<prefix>/bin/skillbinder` under a Homebrew prefix (`/opt/homebrew`, `/usr/local`) whose `Caskroom` holds `skillbinder` or `skillbinder@beta` reads as Homebrew's and is left to `brew`.
- **Windows.** Install adds `{app}\bin` once to `HKCU\Environment\Path`, as an expandable string, and tells running programs the environment changed; only terminals opened afterwards see it, and the toast says "Open a new terminal to use skillbinder." Uninstall, and the app's uninstaller, remove the entry. A copy the setup program didn't install can't install the command.
- **Linux.** Install copies the running AppImage's command to `~/.local/bin/skillbinder` (mode 755) and records its digest in `<application data>/command-line.sha256`. Each launch replaces a copy with the recorded digest that differs from the running AppImage's, so an app update updates it. A copy with any other digest is someone else's.

Install never replaces a `skillbinder` it did not write, and Uninstall removes only what Install wrote. Install's toast says "skillbinder is installed." (Windows: "Open a new terminal to use skillbinder."); a failure's toast says what failed, with no glyph, and the row stays as it was.

## `skillbinder status`

Prints the app's version, then the active preset, the library's counts, the copies waiting for your answer (only when some wait), the Sync state, and the change that runs, if any:

```
SkillBinder 0.1.0
Active preset: Default
Library: 9 skills, 9 active, 2 with updates
Sync: not set up
```

The counts are Library's All, Active, and Updates with no folder tab, search, or filter. Before onboarding finishes it prints the version and "Setup isn't finished. Open SkillBinder to finish setting up."; every other command will exit 5 with "Open SkillBinder to finish setting up." until then (spec rule 4).

With `--json` it prints exactly one document on stdout: `{"ok": true, "result": {"version", "onboarding_finished", "active_preset", "skills", "active", "updates", "attention", "sync", "running"}}`, `sync` being one of `not_configured`, `synced`, `needs_pull`, `needs_push`, `needs_sync`, `needs_resolve`, which text prints as "skills changed on both sides", and `unreachable`, which text prints as "can't reach the remote". A failure prints `{"ok": false, "error": {"code", "message", "hint"}}`, and in text its message and hint on stderr. A wrong command line exits 2, as one JSON document with `--json`; `--help` and `--version` print text.

## Library, presets, folders, and copies

| Command | Does |
| --- | --- |
| `skills [--active \| --inactive \| --updates] [--folder F]` | Library's list under one filter and folder tab, by name: each skill, its marks (`update`, `drifted`, `new`), the agents the active preset holds it for, and the workspaces holding it, or `off` |
| `skill <slug>` | The skill's page: description, source, folder, presets, where it is on, and its copies |
| `enable <slug>… [--agent A]… [--replace]` | With no agent, holds the skills in the active preset for all your agents, as a Library row's "Active in <preset>" menu item does; with agents, adds them to what the preset holds |
| `disable <slug>… [--agent A]…` | With no agent, takes the skills out of the active preset; with agents, only for them, so a skill held for all agents stays held for your other ones |
| `presets` | The presets, `*` marking the active one, with their skill counts |
| `preset create <name>`, `rename <name> <new>`, `clone <name>` | The sidebar's presets |
| `preset delete <name> [--yes]` | Deletes it once confirmed; the active one switches to Default |
| `preset activate <name> [--replace]` | Clicking the preset; prints `Work is active: 3 added, 5 removed.` |
| `preset add <name> <slug>… [--agent A]… [--replace]`, `preset remove <name> <slug>…` | `enable` and `disable` on any preset |
| `folders`, `folder create <name>`, `rename <name> <new>`, `delete <name> [--yes]` | Library's folder tabs |
| `folder move <slug>… (--to F \| --none)` | Files the skills in a folder, or in none |
| `copies` | Every copy waiting for your answer, from any surface |
| `copies replace \| keep <slug> [--in DIR]`, `copies retry` | The review list's answers and Retry; `--in` names the directory when the skill waits in more than one |

A skill is named by its slug, a preset or folder by its name, a workspace as below, and an agent by its registry identity (`claude-code`), its Agents row's name, or its global skill directory (`~/.claude/skills`), which names every agent reading it. Names compare ignoring case. An unknown name exits 2, "No skill is named "pd"." with the close names as its hint ("Did you mean pdf, pdf-forms?"); a name several things answer to exits 2 listing them, unless one matches its case too. Every name resolves before anything runs, so a wrong one changes nothing.

**Busy.** While a blocking change (a switch, deleting the active preset, an import, adding skills, turning an agent on, from the window or a command), an Update all, the window's own short work (a save, an Update, a Git operation), or another command's short change runs, every changing command exits 4 at once: "SkillBinder is busy: Switching to Work…" and "Try again when it finishes." A Git operation is named as it runs ("SkillBinder is busy: Pushing…"), here and in `status`. Reading commands always answer.

**Progress.** A change that writes copies prints `Switching to Work… 3/14` on one stderr line in a terminal, from the same progress the window's strip shows. With a window open, `preset activate` and deleting the active preset run through the window's strip; without one, behind the menu's gate.

**Copies (rule 6).** A change writes everything else, then asks about each copy it left waiting that was not waiting before: in a terminal "Replace your copy of pdf in ~/.claude/skills? [y/N]" (or "Remove your edited copy of …" for an edited copy of a skill no longer active there), where `y` replaces it and anything else keeps it waiting; with `--replace`, every one is replaced without asking; with neither, all wait. Copies left waiting stay in `copies` and the app's review list, the change's result lists them, and the command exits 3: "1 copy needs your answer:", each copy, and "Run `skillbinder copies replace pdf --in ~/.claude/skills`, or review it in SkillBinder." (for several, "Run `skillbinder copies replace <slug> --in <directory>` for each, or review them in SkillBinder."). Running the change again with `--replace` would find it done and leave them waiting, so the hint names `copies replace`.

**Deleting (rule 8).** `preset delete`, `folder delete`, and `workspace remove` ask in a terminal with the window's confirmation text, "Delete Work? Its 3 skills stay in the library." followed, for the active preset, by "Default becomes the active preset, which adds … and removes … for your agents." Without a terminal they need `--yes`, else exit 3 with that text and "Run again with --yes." and change nothing; answering no exits 3 with "Nothing changed." Deleting the active preset with `--yes` lists what the switch to Default added and removed. The confirmation checks the library's revision, so a change made meanwhile fails rather than acting on an old answer. `--yes` never replaces copies, and `--replace` never confirms a deletion (A5).

**Ctrl-C (rule 3).** During a change it prints "Still running in SkillBinder." and exits 130; the app finishes the change as if its window had closed, leaves the copies it would have asked about waiting (with `--replace` it still replaces them), and stays running until the change ends even with no icon.

**Exit codes.** 0 done; 1 failed (`failed`, `conflict` for a stale confirmation, `offline`); 2 usage, unknown or ambiguous name (`usage` for a name the engine refuses, such as a duplicate); 3 waiting for your answer (`needs_answer`); 4 busy; 5 unavailable or not set up; 130 interrupted. `--json` prints `{"ok": true, "result": …}`, `{"ok": false, "error": {"code", "message", "hint"}}`, or for a change that partly landed `{"ok": false, "error": …, "result": {"message", "activated", "added", "removed", "waiting": [{"skill", "directory", "state", "reason"}], "installed", "updated"}}`, `state` being one of `applied`, `kept`, `conflict`, `edited`, `edited_inactive`, `not_applied`, and `not_removed`.

## Workspaces and your agents

| Command | Does |
| --- | --- |
| `workspaces` | The sidebar's workspaces: each one's name and folder, `(missing)` when its folder was missing at the last scan, its agents (the workspace skill directories it uses), and the skills on there |
| `workspace add <path> [--replace]` | The sidebar's Add folder: the folder, or each project in it that has an agent skill directory, becomes a workspace and shows in the window at once. Prints `Added workspace "invoices-app" (.agents/skills, .claude/skills).`; a library skill already in a new workspace's directories turns on there, and a same-slug copy with other content waits as rule 6 says |
| `workspace rename <ws> <name>` | Renaming it in the sidebar |
| `workspace remove <ws> [--yes]` | The sidebar's Remove workspace, once confirmed: "Remove invoices-app? Removes the workspace from SkillBinder. Files in the folder are not touched." Its copies stay in place, unmanaged |
| `workspace enable <ws> <slug>… [--agent A]… [--replace]` | The workspace's Add skill picker: with no agent, holds the skills for every agent the workspace uses, including later ones, as the All pill does; with agents, for exactly their directories, which join the workspace's agents when it does not use them yet. A workspace using no agent needs `--agent` (exit 2); one whose folder is missing refuses (exit 1). It blocks as the picker's Add does |
| `workspace disable <ws> <slug>…` | Removing the skills from the workspace's page, which removes their managed copies there; a skill it does not hold is off already |
| `agents` | Settings › Agents: every agent found on this computer, `*` marking yours, with their ids and global skill directory; the universal agents share one row |
| `agent on <agent> [--replace]` | Its switch in Settings › Agents: applies the active preset to it as a blocking change, `Adding skills to Cursor… 3/9`, with copies settled as rule 6 says |
| `agent off <agent>` | Its switch off, saved at once: "Cursor is off. Its skills stay in place." Its copies stay, unmanaged |

A workspace is named by its name or by a folder inside it: `.` is the workspace holding the current folder. The command resolves a name that is an existing folder, links resolved, and the app takes the innermost workspace holding it; when none does, it takes the workspace by that name, and when none has it either exits 2 with "No workspace holds ~/code/notes." and "Add it with `skillbinder workspace add .`." A folder held by one workspace that is also another workspace's name, such as `web` typed in `~/code/platform`, which holds a `web` folder, names both and exits 2; `./web` names the folder. In `workspace enable`, an agent is named by its registry identity or name, which picks the agent's own workspace skill directory, or by the directory itself (`.claude/skills`, or the path in the workspace), among those the Add skill picker offers there: the workspace's own and your agents'. `agent on` and `agent off` name any agent found, yours or not, as Settings › Agents does; turning on an agent that is on, or off one that is off, changes nothing.

## Scan, import, Explore, and your repositories

`scan`, `import`, `ignore`, `unignore`, `search`, `install`, `repos`, and `repo add|remove|install-all` are in [Command line: scan, import, and Explore](command-line-explore.md).

## Updates and Sync

`updates [--check]`, `update (<slug>… [--diff] | --all)`, `reinstall`, `sync`, `push`, `pull`, `remote set|remove`, `sync status|history|restore|discard|resolve`, and `device rename` are in [Command line: updates and Sync](command-line-updates.md).

## Starting the app

With no app answering, the command prints "Starting SkillBinder…" on stderr, starts the app with `--background`, and waits up to 20 seconds for it. The background app opens no window and, on macOS, shows no Dock icon. With its icon on it keeps running afterwards; with the switch off, or on Linux with no tray host, it quits with exit 0 once no command is connected, and 30 seconds after starting should no command connect. Its quit is an ordinary one, so a ready app update installs then.

It exits 5 naming why when the app cannot start: on Linux with no Wayland or X11 display, "SkillBinder needs a desktop session, and there is none here."; with the app missing beside the command, "SkillBinder isn't where this `skillbinder` expects it."; when the app exits with an error, or doesn't answer within 20 seconds. A command whose protocol differs from the app's exits 5: "This `skillbinder` doesn't match SkillBinder <version>." and "Repair it in Settings › General."

## A second launch

Launching the app while it runs opens its window, or brings it forward, over the same socket; no `activate` file is written. It prints `SkillBinder is already running; opened its window.` to stderr. A second launch made while the first is still starting retries for up to 12 seconds; when the running app never answers, or speaks another protocol, it prints `SkillBinder is already running.` instead and leaves the app alone.

