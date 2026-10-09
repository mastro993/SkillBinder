# SkillBinder

SkillBinder manages a local library of agent skills with native GPUI screens and GPUI Kit components. Discover installed skills, review copies before importing them, organize a flat folder library, and synchronize its portable files through Git.

## Run

Install Rust 1.96 through `rustup` and the [platform prerequisites](docs/native-gate.md). Then run:

```sh
cargo run -p skillbinder --locked
```

SkillBinder requires macOS 26 or newer on Apple silicon or Intel Macs. Windows and Linux builds are also supported.

First launch checks Git 2.39 or newer and storage access, then creates a local library through four setup steps. Synchronization is optional. In Settings, select project folders through the native directory picker. Discovery searches the 79-agent registry and those project boundaries. Imports copy files; they never modify sources.

The application uses a fresh `SkillBinder` directory beneath each operating system's local application-data directory. Existing `~/.skillbinder` data is left untouched. There is no migration or compatibility application.

The shipped features are implemented; UI styling and visual fidelity are deferred. See [feature coverage](docs/features/native.md) and [verification status](docs/native-gate.md).

## Build and verify

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo run -p xtask --locked -- verify
cargo build -p skillbinder --release --locked
cargo run -p xtask --locked -- package
```

Packaging writes a macOS app and DMG, Windows ZIP, or Linux tarball beneath `dist/`. Signing and publication are separate. `xtask verify` checks retained registry and baseline evidence; it does not claim native interaction or visual parity.

For isolated native testing:

```sh
cargo run -p skillbinder --features native-test --locked -- --data-dir /tmp/skillbinder-test --home /tmp/skillbinder-test-home
```

Add `--smoke-test` to exercise native rendering, text, focus, overlays, and orderly exit automatically against the isolated data directory. The `native-test` build also accepts `--mcp --data-dir <isolated-path>` for opt-in UI automation through the [GPUI MCP bridge](docs/native-gate.md#run-the-opt-in-mcp-bridge). Ordinary builds contain no bridge or test flags.

See [architecture](docs/architecture/native.md), [baseline evidence](fixtures/baseline/README.md), and [contributing](CONTRIBUTING.md).
