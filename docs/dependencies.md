# Dependency baseline

Pinned foundation baseline, verified 2026-09-20:

| Tool or library         | Version                             |
| ----------------------- | ----------------------------------- |
| Node.js                 | 24.14.1                             |
| pnpm                    | 10.33.0                             |
| Rust                    | 1.96.0                              |
| Tauri CLI / API         | 2.11.5 / 2.11.1                     |
| Tauri Rust crate        | 2.11.6                              |
| Vite                    | 8.3.0                               |
| React                   | 19.3.0                              |
| TypeScript              | 6.0.3                               |
| TanStack Router / Query | 1.170.38 / 5.103.1                  |
| Tailwind CSS            | 4.3.3                               |
| Scroll area primitive   | @radix-ui/react-scroll-area 1.2.18  |
| SQLite crate            | rusqlite 0.40.2 with bundled SQLite |
| Generated IPC           | ts-rs 12.0.1                        |
| SHA-256                 | sha2 0.10.9                         |
| YAML parser             | yaml-rust2 0.10.4                   |
| Unicode normalization   | unicode-normalization 0.1.24        |
| Supported Git           | 2.39.0 or newer                     |

Dependencies are locked by `pnpm-lock.yaml` and `Cargo.lock`. Runtime diagnostics must report the compiled SQLite version before WAL is enabled; this scaffold intentionally uses rollback journaling with `synchronous=FULL`.

## Native setup

- macOS: Xcode Command Line Tools and WebKit supplied by macOS.
- Windows: Microsoft C++ Build Tools and WebView2 development/runtime prerequisites.
- Linux: distribution packages required by Tauri 2, including WebKitGTK and system tray/build libraries.

Run Tauri's current prerequisite instructions for the host OS before `pnpm dev` or `pnpm build`. Local feature tests require no account, GitHub token, cloud service, skills CLI, or real credential store.

Tested during scaffold creation: macOS on Apple Silicon, Git 2.50.1 (Apple Git-155). Other OS families require CI evidence before compatibility claims.
