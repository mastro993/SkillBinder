# Command line tool: acceptance and slices

Behavior: [command-line.md](../command-line.md). Design: [design.md](design.md).

Evidence per check is labeled as AGENTS.md requires: **unit** (Rust tests, including the CLI against a test server and the dispatch against a test client), **native** (interactive check on the running app, recorded), and **packaged** (smoke test of a release build). Command tests use isolated application data and local bare remotes.

## Slices

Implement the slices in order. Each one ships alone.

### S0 · The command reaches the app: `status`

`apps/cli` with the protocol, the command server, background start, the second launch over the socket, `skillbinder status` and `--version`, and the CLI in every package. Depends on nothing.

- C0.1 **unit:** `Hello` with another protocol number gets `Mismatch`; the CLI exits 5 with the Repair message.
- C0.2 **native:** with the app running and its window closed, `skillbinder status` prints the version, the active preset, and counts matching Library; `--json` prints one document with `"ok": true`. No window opens.
- C0.3 **native, all three OS:** with the app quit and its icon on, `skillbinder status` prints "Starting SkillBinder…", answers within 20 seconds, and the icon stays afterwards with no window and, on macOS, no Dock icon.
- C0.4 **native:** with the menu bar switch off (or GNOME without a tray host), the same run leaves no SkillBinder process once the command exits.
- C0.5 **native:** launching the app a second time opens the running app's window; no `activate` file is written.
- C0.6 **native:** killing the app and launching it again binds the socket despite the stale `command.sock`.
- C0.7 **native Linux:** over SSH with no display, `skillbinder status` exits 5 naming the missing desktop session.
- C0.8 **unit:** before onboarding finishes, a changing request returns `setup`; `status` answers.
- C0.9 **packaged, all three OS:** the package holds the CLI at the design's paths; on macOS `codesign --verify --strict` passes for it and the bundle is notarized.

### S1 · Install the command from Settings

The Command line tool row, `crates/engine::command_line`, the Homebrew `binary` stanza, the Inno uninstall entry removal, and the Linux refresh at launch. Depends on S0.

- C1.1 **unit:** status reads each table state from a temporary folder: missing, our link, a link to another `SkillBinder.app`, an unrelated file, a Caskroom link, a Linux copy with an unrecorded digest.
- C1.2 **native macOS:** from `/Applications`, Install asks for the password once; a new terminal runs `skillbinder status`. Cancelling the prompt leaves "Not installed". Uninstall removes the link.
- C1.3 **native macOS:** the app run from the disk image shows "Move SkillBinder to Applications to install it." with no button.
- C1.4 **native macOS:** after moving the app, the row shows Repair, and Repair makes `skillbinder` work again.
- C1.5 **native macOS:** after `brew install --cask` of a local cask file with the stanza, the row reads Homebrew with no button.
- C1.6 **native Windows:** Install adds `{app}\bin` once to `HKCU\Environment\Path`; a new PowerShell runs `skillbinder status` and prints to that console; Uninstall and the uninstaller remove it.
- C1.7 **native Linux:** Install copies to `~/.local/bin/skillbinder`; with that folder off the PATH the row shows the export line; after a newer AppImage launches, the copy matches it.
- C1.8 **native:** a `skillbinder` file the app did not write is never replaced; the row says another one is in the way.
- C1.9 **native:** Settings search for "terminal" and "CLI" shows the row.

### S2 · Library, presets, folders, and copies

`skills`, `skill`, `enable`, `disable`, `presets`, `preset …`, `folders`, `folder …`, `copies …`, the busy refusal, the consent flow, and Ctrl-C. Depends on S0.

- C2.1 **unit:** each command resolves names ignoring case; an ambiguous name exits 2 listing matches.
- C2.2 **native:** with the window open, `skillbinder preset activate Work` shows the blocking strip in the window and progress in the terminal, and Work is checked in the menu when it ends.
- C2.3 **native:** during that switch, `skillbinder enable pdf` exits 4 naming the switch; `skillbinder skills` still answers.
- C2.4 **unit:** a switch meeting an unmanaged same-slug copy, run with no terminal and no flag, writes the rest, exits 3, and the copy waits in `copies` and in the app's review list; with `--replace` the copy is replaced and managed; in a terminal, `n` keeps it waiting.
- C2.5 **unit:** `preset delete Work` with no terminal and no `--yes` exits 3 and changes nothing; with `--yes`, deleting the active preset activates Default and the result lists what the switch added and removed.
- C2.6 **native:** Ctrl-C during a switch prints "Still running in SkillBinder." and the switch completes.
- C2.7 **unit:** every request variant is matched in dispatch, and every `AppError` maps to one `CommandError.code` and exit code.

### S3 · Workspaces and your agents

`workspaces`, `workspace …`, `agents`, `agent on|off`. Depends on S2.

- C3.1 **native:** `skillbinder workspace add .` in a project adds it as the sidebar's Add folder would, and the new workspace shows in the window at once.
- C3.2 **unit:** `workspace enable . pdf --agent claude-code` holds pdf there for that directory only; `disable` removes its managed copy.
- C3.3 **unit:** `agent on cursor` applies the active preset to its directory under the gate; conflicts follow C2.4.
- C3.4 **unit:** `workspace remove` follows C2.5's confirmation, and leaves the copies in place, unmanaged.

### S4 · Scan, import, Explore, and repositories

`scan`, `import`, `search`, `install`, `repos`, `repo …`. Depends on S2.

- C4.1 **unit:** `scan` lists found skills not in library with each version's short digest and locations, as the Import skills dialog does.
- C4.2 **unit:** `import pdf` of a slug with two versions exits 2 asking for `@<version>`; `--all` imports what the dialog would tick.
- C4.3 **unit:** an import that replaces copies, with no terminal and no flag, changes nothing and exits 3 listing them; with `--replace` it imports and replaces.
- C4.4 **unit:** `install <owner>/<repo>/<slug>` against a `file://` catalog fixture installs it inactive, with its source; a slug held from another source exits 1 with code `conflict`.
- C4.5 **native:** `install` of a skills.sh `<owner>/<repo>@<skill>` installs it, and Explore shows it in library while the window is open.
- C4.6 **unit:** `repo add`, `repo install-all`, and `repo remove` match Explore's behavior on a local bare remote.

### S5 · Updates and Git

`updates [--check]`, `update`, `update --all`, `reinstall`, `sync`, `push`, `pull`, `remote …`. Depends on S2.

- C5.1 **unit:** `updates --check` runs Check now and lists skills with an update from a local bare source.
- C5.2 **native:** `update --all` shows the same progress in the Library header and the menu, and a second `update --all` meanwhile exits 4.
- C5.3 **unit:** `sync` against a local bare remote commits and pushes; without Git it exits 1 with the app's Git message.
- C5.4 **unit:** `reinstall` of a drifted skill restores its recorded content.

## Done

Every check passes with its labeled evidence, and the docs listed under "Dependencies and policy" in the design describe the shipped behavior.
