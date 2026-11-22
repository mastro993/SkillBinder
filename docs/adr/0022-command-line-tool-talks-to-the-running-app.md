# The command line tool talks to the running app over a local socket

Extended by ADR 0025: `skillbinder mcp` serves the same requests to agents over MCP.

Everything SkillBinder does needs its window or its menu bar menu, so a coding agent, a script, or a user in a terminal cannot reach it. The app now ships a command line tool, the `skillbinder` command ([spec](../specs/command-line.md)). Only one process may hold the library: the app takes `application.lock` and the engine `library.lock`, both exclusive, and since ADR 0020 the app keeps running behind its icon most of the day. A command that opened the library itself would fail whenever the app runs.

So the command never holds the library. It is a separate, small console program, `skillbinder-cli` in the app's package and `skillbinder` on the user's PATH, which depends on `crates/proto` and nothing that reaches storage. It connects to the running app over a local socket, a Unix domain socket in application data on macOS and Linux and a named pipe on Windows, both reachable only by the user who runs the app. It sends one typed request and reads typed progress and one typed result back, as JSON lines, after a handshake that names the protocol version. The app runs each request the way the menu bar runs its actions (ADR 0021): through the open window's view, or through the client and the `Gate` when no window is open, under the same rules, so the window and the menu show the change as it lands and refuse what they refuse while it runs.

When no app answers, the command starts SkillBinder with `--background`, which opens no window and on macOS no Dock icon, and waits up to 20 seconds for the socket. A background app with its icon showing stays running afterwards, as after closing its window. One with no icon, because the switch is off or Linux has no tray host, quits once no command is connected or running, so it never stays running where the user cannot see it (ADR 0020). Without a desktop session the app cannot start, and the command says so.

The socket also replaces the `activate` file the second launch writes today: a second launch connects and asks the running app to open its window.

## Considered Options

- The command opening the engine itself when the app is closed, and going through the app when it runs, was rejected by the user. It works over SSH and without a display, but it keeps two paths for every operation in step, and launching the app while a command holds the library would have to wait or fail.
- Refusing when the app is closed was rejected by the user. Agents and scripts would fail whenever the user had quit it.
- A headless daemon owning the engine, with the app as one more client, was rejected. It reverses the one-process design (`docs/architecture/native.md`), and the `Gate` and the blocking strip live in the app.
- The app's own executable taking subcommands was rejected. On Windows a GUI-subsystem program cannot reliably print to the terminal that started it, and the terminal does not wait for it. A separate console program works the same on every platform, starts fast because it links no GPUI, and cannot open the library by construction.
- HTTP on localhost was rejected. Any local process, and any web page through the browser, could reach a port. A socket file or a named pipe is limited to the user, and it is not a network transport.
- Driving the app through the `gpui-mcp` bridge was rejected. It exists only in `native-test` builds, and it drives the interface, not the domain.

## Consequences

- `docs/architecture/native.md` changes "there is no daemon or network command transport": there is still no daemon and no network transport, but the app serves a local command socket.
- A new workspace member, `apps/cli`, with its own Clippy policy, since it prints, reads the socket, and starts the app. `xtask/src/graph.rs` allows it `proto`, `clap`, and `interprocess`; `apps/desktop` gains `interprocess` for the listener.
- Every request needs a typed form in `crates/proto`, and every result a typed output, which the command prints as text or as JSON. A new operation in the app is not reachable from the command until it gets one.
- Any process running as the user can drive the app through the socket, as it can already edit the user's files. The command adds no authentication.
- Packages gain a second executable, signed and notarized with the app on macOS.
