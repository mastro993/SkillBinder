# Command line

The `skillbinder` command runs from a terminal and talks to the running app; it never opens the library itself ([spec](../../specs/command-line.md); ADR 0022, 0023). The architecture is in [Native command line](../../architecture/native-command-line.md). Slice S0 (#199) ships the path every later command uses: `skillbinder status`, `--version`, `--help`, background start, and the second launch over the same socket. Slice S2 (#201) adds the library, presets, library folders, and copies. Installing the command on the PATH from Settings is slice S1.

## Where it is

Each package holds it beside the app: `SkillBinder.app/Contents/MacOS/skillbinder-cli` on macOS, `{app}\bin\skillbinder.exe` on Windows, and `usr/bin/skillbinder-cli` inside the AppImage on Linux.

## `skillbinder status`

Prints the app's version, then the active preset, the library's counts, the copies waiting for your answer (only when some wait), the Sync state, and the change that runs, if any:

```
SkillBinder 0.1.0
Active preset: Default
Library: 9 skills, 9 active, 2 with updates
Sync: not set up
```

The counts are Library's All, Active, and Updates with no folder tab, search, or filter. Before onboarding finishes it prints the version and "Setup isn't finished. Open SkillBinder to finish setting up."; every other command will exit 5 with "Open SkillBinder to finish setting up." until then (spec rule 4).

With `--json` it prints exactly one document on stdout: `{"ok": true, "result": {"version", "onboarding_finished", "active_preset", "skills", "active", "updates", "attention", "sync", "running"}}`, `sync` being one of `not_configured`, `synced`, `needs_pull`, `needs_push`, and `needs_sync`. A failure prints `{"ok": false, "error": {"code", "message", "hint"}}`, and in text its message and hint on stderr. A wrong command line exits 2, as one JSON document with `--json`; `--help` and `--version` print text.

## Library, presets, folders, and copies

| Command | Does |
| --- | --- |
| `skills [--active \| --inactive \| --updates] [--folder F]` | Library's list under one filter and folder tab, by name: each skill, its marks (`update`, `drifted`, `new`), the agents the active preset holds it for, and the workspaces holding it, or `off` |
| `skill <slug>` | The skill's page: description, source, folder, presets, where it is on, and its copies |
| `enable <slug>… [--agent A]… [--replace]` | With no agent, holds the skills in the active preset for all your agents, as the Active checkbox and the agents menu's All agents do; with agents, adds them to what the preset holds |
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

A skill is named by its slug, a preset or folder by its name, and an agent by its registry identity (`claude-code`), its Agents row's name, or its global skill directory (`~/.claude/skills`), which names every agent reading it. Names compare ignoring case. An unknown name exits 2, "No skill is named "pd"." with the close names as its hint ("Did you mean pdf, pdf-forms?"); a name several things answer to exits 2 listing them, unless one matches its case too. Every name resolves before anything runs, so a wrong one changes nothing.

**Busy.** While a blocking change (a switch, deleting the active preset, an import, adding skills, turning an agent on), an Update all, the window's own short save, or another command's short change runs, every changing command exits 4 at once: "SkillBinder is busy: Switching to Work…" and "Try again when it finishes." Reading commands always answer. A Git operation does not refuse these commands, as it does not refuse the window's.

**Progress.** A change that writes copies prints `Switching to Work… 3/14` on one stderr line in a terminal, from the same progress the window's strip shows. With a window open, `preset activate` and deleting the active preset run through the window's strip; without one, behind the menu's gate.

**Copies (rule 6).** A change writes everything else, then asks about each copy it left waiting that was not waiting before: in a terminal "Replace your copy of pdf in ~/.claude/skills? [y/N]" (or "Remove your edited copy of …" for an edited copy of a skill no longer active there), where `y` replaces it and anything else keeps it waiting; with `--replace`, every one is replaced without asking; with neither, all wait. Copies left waiting stay in `copies` and the app's review list, the change's result lists them, and the command exits 3: "1 copy needs your answer:", each copy, and "Run `skillbinder copies replace pdf --in ~/.claude/skills`, or review it in SkillBinder." (for several, "Run `skillbinder copies replace <slug> --in <directory>` for each, or review them in SkillBinder."). Running the change again with `--replace` would find it done and leave them waiting, so the hint names `copies replace`.

**Deleting (rule 8).** `preset delete` and `folder delete` ask in a terminal with the window's confirmation text, "Delete Work? Its 3 skills stay in the library." followed, for the active preset, by "Default becomes the active preset, which adds … and removes … for your agents." Without a terminal they need `--yes`, else exit 3 with that text and "Run again with --yes." and change nothing; answering no exits 3 with "Nothing changed." Deleting the active preset with `--yes` lists what the switch to Default added and removed. The confirmation checks the library's revision, so a change made meanwhile fails rather than acting on an old answer. `--yes` never replaces copies, and `--replace` never confirms a deletion (A5).

**Ctrl-C (rule 3).** During a change it prints "Still running in SkillBinder." and exits 130; the app finishes the change as if its window had closed, leaves the copies it would have asked about waiting (with `--replace` it still replaces them), and stays running until the change ends even with no icon.

**Exit codes.** 0 done; 1 failed (`failed`, `conflict` for a stale confirmation, `offline`); 2 usage, unknown or ambiguous name (`usage` for a name the engine refuses, such as a duplicate); 3 waiting for your answer (`needs_answer`); 4 busy; 5 unavailable or not set up; 130 interrupted. `--json` prints `{"ok": true, "result": …}`, `{"ok": false, "error": {"code", "message", "hint"}}`, or for a change that partly landed `{"ok": false, "error": …, "result": {"message", "activated", "added", "removed", "waiting": [{"skill", "directory", "state", "reason"}]}}`, `state` being one of `applied`, `kept`, `conflict`, `edited`, `edited_inactive`, `not_applied`, and `not_removed`.

## Starting the app

With no app answering, the command prints "Starting SkillBinder…" on stderr, starts the app with `--background`, and waits up to 20 seconds for it. The background app opens no window and, on macOS, shows no Dock icon. With its icon on it keeps running afterwards; with the switch off, or on Linux with no tray host, it quits with exit 0 once no command is connected, and 30 seconds after starting should no command connect. Its quit is an ordinary one, so a ready app update installs then.

It exits 5 naming why when the app cannot start: on Linux with no Wayland or X11 display, "SkillBinder needs a desktop session, and there is none here."; with the app missing beside the command, "SkillBinder isn't where this `skillbinder` expects it."; when the app exits with an error, or doesn't answer within 20 seconds. A command whose protocol differs from the app's exits 5: "This `skillbinder` doesn't match SkillBinder <version>." and "Repair it in Settings › General."

## A second launch

Launching the app while it runs opens its window, or brings it forward, over the same socket; no `activate` file is written. A second launch made while the first is still starting retries for up to 12 seconds, then gives up silently.

## Evidence

- **Unit:** `command_line::tests::another_protocol_gets_a_mismatch` (proto) and the command's `another_protocol_exits_5_naming_repair`, against a test server on a temporary socket that answers `Mismatch` (C0.1); `exit_codes_follow_the_spec`; `lines_round_trip`; the endpoint's private-folder fallback; `no_app_is_not_running`; `commands::tests` for `status` answering before onboarding finishes, counting the library under the active preset, and naming the running change (C0.8, `status` half); `render::tests` for the text; the graph test that the command never reaches the engine, client, UI, or GPUI.
- **Native, macOS, 2026-11-23** (native-test debug build, isolated data folder under the scratchpad, whose long path exercised the fallback socket in the temporary folder): with the app quit and its icon on, `skillbinder status` printed "Starting SkillBinder…" and answered in 2.3 s; the app stayed running as a background-only process with no window, its socket mode 0600 (C0.3, macOS). `--json` printed one document with `"ok": true` (C0.2). A second launch opened the running app's window, turning it into a regular app with one window, and wrote no `activate` file (C0.5). After `kill -9`, the next `status` started the app again over the stale socket (C0.6). With `show_in_tray` off, `status` left no SkillBinder process, and the app exited 0; a background app no command reached quit after 30 seconds with exit 0 (C0.4, without the staged app update). `status | cat` returned at once while the started app kept running. With the switch off, closing a foreground window still quit with exit 0.
- **Native, macOS, 2026-11-23, onboarded** (synthetic world, onboarding's import of 9 skills driven through the MCP bridge, window open): `status` printed Default, 9 skills, 9 active, 2 with updates, matching Library's All 9, Active 9, and Updates 2 (C0.2 counts).
- **Packaged, macOS, 2026-11-23** (local `xtask package` up to `dmgbuild`, which was not installed, then ad-hoc signed): the bundle held `Contents/MacOS/skillbinder-cli`, and `codesign --verify --strict` passed for it and the bundle (C0.9, without Developer ID signing or notarization).
- **Unit, S2:** `commands::resolve::tests` (C2.1: skills, presets, folders, and agents by id, name, and directory ignoring case; unknown names exit 2 with close ones; an ambiguous name exits 2 listing matches, the exact case winning; a shared directory naming every agent reading it); `commands::plan::tests` (enable and disable for all or named agents, the switch's adds and removes, unknown names changing nothing, the folder confirmation text, a copy waiting in two directories); `commands::consent::tests` (C2.4 and C2.5: only copies the change left asking wait; no terminal and no flag leaves them and exits 3; `--replace` replaces each without asking; in a terminal `n`, or a command gone, keeps a copy waiting; a deletion without `--yes` exits 3 with the window's text and `--yes` goes ahead); `commands::reads::tests` (status, Library's filters and folder tab, the skill page, presets, and the busy refusal naming the switch while reads answer); `commands::tests` (C0.8: reads and changes need onboarding, `status` does not); `command_line::tests::every_app_error_maps_to_one_code_and_exit` (C2.7, with `Request`, `Query`, `Change`, and `ErrorCategory` matched without wildcards, so a new variant fails to compile until handled); the command's `grammar::tests` (flags, `--replace` and `--yes` kept apart), `render` tests (the design's partial-result text and JSON), and `connect::tests::a_switch_shows_progress_asks_and_exits_3_with_what_landed` against a test server.
- **Native, macOS, 2026-11-23, S2** (native-test debug build, synthetic world onboarded through the MCP bridge, window open, with an unmanaged `release-notes` in `~/.agents/skills` that Work holds): with the window's strip held by `hold_activation`, `preset activate Work` with no terminal exited 3 with "Work is active: …", "1 copy needs your answer:", the copy, and the hint, while the window showed `activation-strip` "Switching…"; meanwhile `enable pdf-tools` exited 4, "SkillBinder is busy: Switching to Work…", `skills --active` and `status` answered, and `copies` listed the copy (C2.2 without the menu observed, C2.3, C2.4). The window kept refusing changes, and so did the command, until its Done pill closed. In a pseudo-terminal, progress showed `Switching to Work… 28/34` and answering `n` to "Replace your copy of release-notes in ~/.agents/skills? [y/N]" kept it waiting, exit 3; `--replace` replaced it with the library's content and left nothing waiting. `preset delete work` without a terminal exited 3 with the window's text and changed nothing; `--yes --json` deleted the active Work and listed what the switch to Default added and removed, matching the confirmation. SIGINT 50 ms into `preset activate Work` printed "Still running in SkillBinder.", exited 130, and `status` then showed Work active (C2.6). With the app running in the background with no window, the switch ran behind the gate and exited 3 for the same copy; `copies keep`, `copies replace --json`, `--in` with an unknown directory (exit 2), `folder create|move|delete`, and `disable --agent` behaved as described above.
- **Not yet run:** C0.3 on Windows and Linux; C0.4 with a staged app update and on GNOME without a tray host; C0.7 over SSH on Linux; C0.9 on Windows and Linux and the notarized macOS release; C2.2's menu check mark, which the bridge cannot read (the menu reads the same snapshot `status` showed as Work).
