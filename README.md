# SkillBinder

[![Checks](https://github.com/mastro993/SkillBinder/actions/workflows/checks.yml/badge.svg?branch=main)](https://github.com/mastro993/SkillBinder/actions/workflows/checks.yml?query=branch%3Amain)
[![Native builds](https://github.com/mastro993/SkillBinder/actions/workflows/native.yml/badge.svg?branch=main)](https://github.com/mastro993/SkillBinder/actions/workflows/native.yml?query=branch%3Amain)

SkillBinder manages a local library of agent skills with native GPUI screens and GPUI Kit components. Discover the skills in your agents and workspaces, review copies before installing them, organize library skills into presets, see which agents hold each library skill, and synchronize its skill directories through Git. Presets and library folders stay on each device.

## Run

Install Rust 1.96 through `rustup` and the [platform prerequisites](docs/native-gate.md). Then run:

```sh
cargo run -p skillbinder --locked
```

SkillBinder requires macOS 26 or newer on Apple silicon or Intel Macs. Windows and Linux builds are also supported.

First launch checks Git 2.39 or newer and storage access, then creates a local library through four setup steps. Synchronization is optional. The sidebar's Workspaces section lists the project folders you add through the native directory picker. Picking a folder of projects, such as `~/Projects`, adds each project that has an agent skill directory as its own workspace. Your home folder is never a workspace; its skills are shown under Agents. SkillBinder scans your agents' global skill directories, such as `~/.claude/skills`, and every workspace's agent skill directories on launch and whenever its window regains focus. Discovery lists the skills it found that are not installed, and the Agents section lists each detected agent's found skills. Installing copies files into the library; SkillBinder never modifies sources.

The application uses a fresh `SkillBinder` directory beneath each operating system's local application-data directory. Existing `~/.skillbinder` data is left untouched. There is no migration or compatibility application.

The shipped features are implemented; UI styling and visual fidelity are deferred. See [feature coverage](docs/features/native.md) and [verification status](docs/native-gate.md).

## Build and verify

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cccc .
cargo test --workspace --all-targets --locked
cargo run -p xtask --locked -- verify
cargo build -p skillbinder --release --locked
cargo run -p xtask --locked -- package
```

Run `git config core.hooksPath .githooks` once per clone to turn on the pre-commit hook. It rejects a commit that stages a file over 20,000 bytes and explains how to split it. Lockfiles, generated files, vendored skills and text, fixtures, artwork, and other binary files are exempt; `.gitattributes` lists the exempt text paths as `linguist-generated` or `linguist-vendored`.

Packaging writes a macOS app and DMG, Windows ZIP, or Linux tarball beneath `dist/`. Signing and publication are separate. `xtask verify` checks retained registry and baseline evidence and the workspace crate graph; it does not claim native interaction or visual parity.

For isolated native testing:

```sh
cargo run -p skillbinder --features native-test --locked -- --data-dir /tmp/skillbinder-test --home /tmp/skillbinder-test-home
```

Add `--smoke-test` to exercise native rendering, text, focus, overlays, and orderly exit automatically against the isolated data directory. The `native-test` build also accepts `--mcp --data-dir <isolated-path>` for opt-in UI automation through the [GPUI MCP bridge](docs/native-gate.md#run-the-opt-in-mcp-bridge). Ordinary builds contain no bridge or test flags.

See [architecture](docs/architecture/native.md), [baseline evidence](fixtures/baseline/README.md), and [contributing](CONTRIBUTING.md).
