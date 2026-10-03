# Contributing to SkillBinder

## Build and verify a change

1. Read [the native gate status](docs/native-gate.md) before changing product behavior. Feature reconstruction is blocked until the minimal replacement passes all three native platforms.
2. Install Rust through `rustup` and the platform prerequisites listed in that guide.
3. Run `cargo run -p skillbinder --features native-test --locked` to exercise native controls.
4. Run the verification commands in the README.
5. Update the affected documents in `docs/` when behavior changes.

Keep modules focused on one feature or responsibility. Share dependencies and lints through the root manifest. Preserve the exact Ely and GPUI revisions unless a separate dependency decision changes them.

## Preserve the UI contract

Use the canonical baseline captures and licensed Hugeicons assets. The baseline's production font is the native system font. Development captures injected Inter through development tools and are not the typography reference.

Use Ely's public APIs without modifying dependency sources. Compose application-owned GPUI styling when a finished component cannot match the contract. Record a concrete blocker when a public API cannot support a required interaction or visual effect.

Do not claim visual parity from a successful build or a headless test. Capture the same operating system, theme, window size, and state for comparisons. Record native input, focus restoration, IME, reduced motion, and directory-picker results separately.

## Protect local data

The foundation does not open application data. Future engine tests must use explicit isolated data roots and local Git remotes. The native application uses a fresh `SkillBinder` directory beneath the operating system's local application-data directory. It must leave `~/.skillbinder` untouched.

Do not push, publish, sign, or merge unless the task authorizes that action. Changes to credentials and publishing remain separate from packaging verification.
