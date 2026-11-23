# Command line

The `skillbinder` command runs from a terminal and talks to the running app; it never opens the library itself ([spec](../../specs/command-line.md); ADR 0022). The architecture is in [Native command line](../../architecture/native-command-line.md). Slice S0 (#199) ships the path every later command uses: `skillbinder status`, `--version`, `--help`, background start, and the second launch over the same socket. Installing the command on the PATH from Settings is slice S1.

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
- **Not yet run:** C0.3 on Windows and Linux; C0.4 with a staged app update and on GNOME without a tray host; C0.7 over SSH on Linux; C0.8's refusal of a changing request, which arrives with S2's first one; C0.9 on Windows and Linux and the notarized macOS release.
