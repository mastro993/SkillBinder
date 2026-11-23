# Native command line

The `skillbinder` command reaches the running app over a local socket and never opens the library itself ([spec](../specs/command-line.md), [design](../specs/command-line/design.md), ADR 0022; [feature](../features/native/command-line.md)). This page records how the pieces are built.

## Parts

| File | Responsibility |
| --- | --- |
| `crates/proto/src/command_line.rs` | The wire contract: `PROTOCOL`, `Hello`, `Greeting` (`Welcome` or `Mismatch`, decided by `Greeting::answer`), `Request` (`Status`, `Open`), `Reply::Outcome`, `Output`, `StatusOutput`, `CommandError`, and `ErrorCode` with its exit code. `Endpoint::of(data, temp)` names the socket both sides use. |
| `apps/cli` (`skillbinder-cli`) | The command. `main.rs` holds the `clap` grammar, built with the builder API because clap's derive allows Clippy's style lints, which the workspace forbids allowing. `connect.rs` connects, greets, sends the request, and reads the reply; `start.rs` starts the app when nothing answers; `render.rs` prints text or JSON and maps exit codes. Its dependencies are `skillbinder-proto`, `clap`, `interprocess`, `directories`, `serde`, and `serde_json`; it never reaches the engine, the client, the UI, or GPUI. |
| `apps/desktop/src/command_server.rs` | Binds the socket and serves it on the `skillbinder-runtime` Tokio runtime; a second launch's `open_running`; on Linux, `record_app_path`. |
| `crates/ui/src/commands.rs` | `start_commands` takes requests on a GPUI task, as the tray takes clicks (ADR 0021), counts connected commands, and answers them; `quit_if_unseen` is the one rule for quitting an app nothing shows or needs. |

## Endpoint

`Endpoint::of` derives the endpoint from the application data folder, so the command, the app, and a second launch agree without asking each other, and an isolated test root gets its own. On macOS and Linux it is `<application data>/command.sock`; when that path is longer than a Unix socket path allows (103 bytes), it is `skillbinder-<digest>.sock` in the user's temporary folder, where `<digest>` is the 64-bit FNV-1a of the data folder's path. On Windows it is the named pipe `skillbinder-<digest>`; the data folder lies under the user's profile, so each user has their own.

The app holding `application.lock` binds it with `interprocess`'s `try_overwrite`, which removes a socket file a crash left behind. On Unix it then sets the file's mode to 0600 (`interprocess`'s own mode option is not supported on macOS). Windows' default named pipe security lets only the creating user, administrators, and the system write to the pipe, so no security descriptor is passed. A socket the server cannot bind is logged, and the app runs without the command.

## Exchange

Each line is one JSON object. The command sends `Hello { protocol, version }`; the app answers `Greeting::answer`'s `Welcome` or `Mismatch` and closes after a mismatch, which the command reports as exit 5, "This `skillbinder` doesn't match SkillBinder <version>." with the hint "Repair it in Settings › General." The command then sends one `Request` and reads `Reply` lines until `Reply::Outcome`.

The server counts a command as connected (`Commands::connect`) from its request until its reply is written, and hands the request to the GPUI task through an `async_channel`. `dispatch` answers before onboarding finishes only for requests `needs_setup` excludes, an exhaustive match so each new request must decide, and otherwise `CommandError::setup()`. `Status` reads the client's snapshot and the `Gate`: the active preset from `resolved_active`, Library's counts with no folder, search, or filter (`snapshot_query::library_counts`), the waiting copies the review list shows (`waiting_copies`), the Sync state, and the running blocking change's title, or the Update all progress.

## Lifecycle

`--background` is the only argument a release build accepts; a native-test build accepts it beside its own. It skips opening the window and sets the macOS Accessory policy, so no Dock icon shows. A background launch that finds another app running exits 0, since the command reaches that one; any other second launch calls `open_running`, which connects, greets, sends `Open`, and waits for the app to close the connection once it has asked for the window, retrying for up to 12 seconds while the running app starts. No `activate` file or polling thread remains.

`quit_if_unseen` quits when no window is open, no command is connected, and the icon neither shows nor may still show (it is registering, or the preference to show it has not been followed yet). It runs when a command's connection closes, when the tray reaches `Unavailable` or is hidden, when the last window closes (after setting the Accessory policy), and 30 seconds after a background launch, should its command never connect. A background app quits through the same quit hook as any quit: it drains the engine, finishes a ready app update, and exits 0; only a foreground launch whose window failed to open still exits 1.

The command starts the app with `--background`, with no standard streams, in its own process group (Unix) or a new process group with no console window (Windows), so neither a pipe reading the command's output nor the terminal's Ctrl-C holds or stops it. It finds the app at `Contents/MacOS/skillbinder` beside itself in a bundle, at `..\skillbinder.exe` from Windows' `bin`, beside itself in a Cargo target folder, and on Linux at the path the app writes to `<application data>/app-path` at each launch (`$APPIMAGE`, else its executable). On Linux without `WAYLAND_DISPLAY` or `DISPLAY` it exits 5 naming the missing desktop session. It polls the endpoint every 100 ms for up to 20 seconds, and stops early when the app it started exits with an error. Its hidden `--data-dir` option points it at an isolated root and passes the root on to a native-test app it starts.

## Packaging

`xtask package` requires both release executables and places the command at `Contents/MacOS/skillbinder-cli` (signed with the hardened runtime before the bundle, both verified with `codesign --verify --strict`), `{app}\bin\skillbinder.exe`, and `usr/bin/skillbinder-cli` in the AppDir. `build.yml` builds `-p skillbinder -p skillbinder-cli` and joins each into the universal macOS target with `lipo`.
