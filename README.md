# SkillBinder

SkillBinder is being rewritten as a native Rust desktop application using GPUI and Ely components. This checkout contains the native dependency gate. Product features are not available yet.

The previous application was removed after preserving revision `83d55ef789a7cf256021f6620742e0e9021a8312`, declarative behavior contracts, licensed artwork, and 88 macOS baseline images. The replacement must pass native builds and interaction checks on macOS, Windows, and Linux before feature reconstruction begins.

## Build the native gate

Install the platform prerequisites in [the native gate guide](docs/native-gate.md). The workspace uses the checked-in Rust toolchain and exact dependency revisions.

```sh
cargo build -p skillbinder --locked
cargo run -p skillbinder --features native-test --locked
```

The test build exercises text input, focus, Hugeicons artwork, a modal dialog, and the native directory picker. It does not open or mutate application data. The ordinary build displays the current development status.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo run -p xtask -- verify
cargo build -p skillbinder --release --locked
```

`xtask verify` checks preserved evidence and the reviewed registry. It does not claim native interaction, visual parity, or packaging acceptance.

See [architecture](docs/architecture/native.md), [baseline evidence](fixtures/baseline/README.md), and [contributing](CONTRIBUTING.md). The [gate status](docs/native-gate.md#verification-status) records unresolved acceptance requirements.
