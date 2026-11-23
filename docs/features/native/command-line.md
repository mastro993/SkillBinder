# Command line

The `skillbinder` command runs from a terminal and talks to the running app; it never opens the library itself ([spec](../../specs/command-line.md); ADR 0022). The architecture is in [Native command line](../../architecture/native-command-line.md). Slice S0 (#199) ships the path every later command uses: `skillbinder status`, `--version`, `--help`, background start, and the second launch over the same socket. Slice S1 (#200) puts the command on the PATH from Settings › General.

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

With `--json` it prints exactly one document on stdout: `{"ok": true, "result": {"version", "onboarding_finished", "active_preset", "skills", "active", "updates", "attention", "sync", "running"}}`, `sync` being one of `not_configured`, `synced`, `needs_pull`, `needs_push`, and `needs_sync`. A failure prints `{"ok": false, "error": {"code", "message", "hint"}}`, and in text its message and hint on stderr. A wrong command line exits 2, as one JSON document with `--json`; `--help` and `--version` print text.

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
- **Unit (S1):** `command_line::tests` in the engine read each state from a temporary folder: missing, our link, a link to another `SkillBinder.app`, a link elsewhere and an unrelated file, Homebrew's link with the cask's `Caskroom` under each prefix (on `/usr/local`, the same path Install writes), outside Applications, and a Linux copy with an unrecorded digest; Repair re-points the link, Uninstall removes only it, and a file SkillBinder didn't write is never replaced or removed; the Linux copy is executable, follows the running AppImage at launch, is left alone once edited, and goes with its record. The Windows PATH entry is added once, compared ignoring case and a trailing backslash, and removed keeping the other entries (C1.1, C1.8). `settings::command_line::tests` check each state's line and control, and that the search finds the row by "terminal", "CLI", "command", and "PATH".
- **Native, macOS, 2026-11-23, S1** (native-test debug build, synthetic world, a fake bundle in the fixture's Applications, the link under `<data>/fixture-root` so `/usr/local/bin` was never touched): the row read "Run SkillBinder from a terminal as skillbinder." with Install; Install wrote the link as the user, the toast said "skillbinder is installed.", the row read Installed with Uninstall, and `skillbinder status` through the link answered the running app. A link into `/Volumes/SkillBinder/SkillBinder.app` showed "skillbinder opens another copy of SkillBinder." with Repair once Settings reopened, and Repair pointed it back (C1.4, by fixture). An unrelated script there showed "Another skillbinder is in …" with no control and stayed unchanged (C1.8). A fixture `opt/homebrew/Caskroom/skillbinder` with its link showed muted "Homebrew" (C1.5, by fixture). Uninstall removed the link and the row offered Install again. Without a bundle the row read "Move SkillBinder to Applications to install it." with no control (C1.3's line). Searching "terminal" and "CLI" showed the row (C1.9).
- **Not yet run (S1):** C1.2 with the administrator prompt from a real `/Applications` install, C1.3 from the disk image, C1.4 after a real move, and C1.5 with `brew install --cask` of a local cask; C1.6 on Windows (the registry code was compiled for `x86_64-pc-windows-msvc` alone, as the full engine needs the Windows C toolchain), C1.7 on Linux.
- **Not yet run:** C0.3 on Windows and Linux; C0.4 with a staged app update and on GNOME without a tray host; C0.7 over SSH on Linux; C0.8's refusal of a changing request, which arrives with S2's first one; C0.9 on Windows and Linux and the notarized macOS release.
