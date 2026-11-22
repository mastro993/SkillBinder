# Contributing to SkillBinder

## Build and verify

1. Read [feature coverage](docs/features/native.md), [architecture](docs/architecture/native.md), and [verification status](docs/native-gate.md).
2. Install the pinned Rust toolchain and platform prerequisites.
3. Run the app on the [synthetic world](fixtures/synthetic/README.md) (`cargo run -p xtask -- fixture <dir>`) with an isolated `native-test` data directory for interaction checks.
4. Run the [verification commands](#verification-commands), including the locked release build.
5. Update affected behavior documentation in `docs/`.

### Verification commands

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cccc .
cargo test --workspace --all-targets --locked
cargo run -p xtask --locked -- verify
cargo build -p skillbinder --release --locked
cargo run -p xtask --locked -- package
```

Run `git config core.hooksPath .githooks` once per clone to turn on the pre-commit hook. It rejects a commit that stages a file over 5,000 tokens (bytes / 4) and explains how to split it; CI runs the same check over every tracked file. Lockfiles, generated files, vendored skills and text, captured baseline fixtures, artwork, and other binary files are exempt; `.gitattributes` lists the exempt text paths as `linguist-generated` or `linguist-vendored`.

Packaging writes a macOS app and DMG, a per-user Windows setup program (Inno Setup 6), or a Linux AppImage beneath `dist/`. The release workflows sign and notarize the macOS image and publish releases and nightly betas on GitHub ([releases](docs/release.md)). `xtask verify` checks retained registry and baseline evidence and the workspace crate graph; it does not claim native interaction or visual parity.

### Isolated native testing

```sh
cargo run -p xtask --locked -- fixture /tmp/skillbinder-world
cargo run -p skillbinder --features native-test --locked -- --data-dir /tmp/skillbinder-world/data --home /tmp/skillbinder-world/home
```

The first command builds a [synthetic world](fixtures/synthetic/README.md) of fictional agents, projects, a skill source, and a Sync remote. A `native-test` build refuses to run without an isolated `--data-dir` or on your real home.

Add `--smoke-test` to exercise native rendering, text, focus, overlays, and orderly exit automatically against the isolated data directory. The `native-test` build also accepts `--mcp --data-dir <isolated-path>` for opt-in UI automation through the [GPUI MCP bridge](docs/native-gate/checks.md#run-the-opt-in-mcp-bridge). Ordinary builds contain no bridge or test flags.

The README's screenshots come from the synthetic world with extra fictional skills, projects, presets, and library folders, captured from a `native-test` build through the MCP bridge in the light and dark themes.

Keep modules focused on one feature or responsibility. Centralize dependencies and lint policy in the root manifest. New workspace members or crate dependencies must update `xtask/src/graph.rs` in the same change. Filesystem, process, network, environment, and terminal work belongs in the engine; the workspace `clippy.toml`, generated from `xtask/src/clippy_policy.rs`, enforces it. Preserve the exact GPUI Kit and GPUI backend versions unless a separate dependency decision changes them.

## Native UI

Use GPUI Kit public editing, interaction, focus, and overlay APIs without modifying dependency sources. Use retained Hugeicons assets. The application sets the native system font after GPUI Kit initialization.

Functional parity comes first; the user deferred visual fidelity and polish. Retained screenshots remain the reference for later appearance work. Never infer native behavior from compilation or fixture images. Record real input, focus, directory selection, and platform results separately.

## Protect local data

Tests use isolated data roots and local bare Git remotes. The application uses a fresh `SkillBinder` directory beneath each operating system's local application-data directory and leaves `~/.skillbinder` untouched. Never run destructive tests against a user's library or sources.

Keep filesystem, hashing, SQLite, and Git work off the GPUI thread. UI code uses typed client operations. Recovery must finish before another durable mutation. Do not bypass source, revision, journal, or Git isolation checks to make a test pass.

Packaging does not authorize signing, publishing, or merging. Perform those actions only when the task explicitly permits them.
