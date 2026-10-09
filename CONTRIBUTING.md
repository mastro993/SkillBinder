# Contributing to SkillBinder

## Build and verify

1. Read [feature coverage](docs/features/native.md), [architecture](docs/architecture/native.md), and [verification status](docs/native-gate.md).
2. Install the pinned Rust toolchain and platform prerequisites.
3. Run the app with an isolated `native-test` data directory for interaction checks.
4. Run the README verification commands, including the locked release build.
5. Update affected behavior documentation in `docs/`.

Keep modules focused on one feature or responsibility. Centralize dependencies and lint policy in the root manifest. Preserve the exact GPUI Kit and GPUI backend versions unless a separate dependency decision changes them.

## Native UI

Use GPUI Kit public editing, interaction, focus, and overlay APIs without modifying dependency sources. Use retained Hugeicons assets. The application sets the native system font after GPUI Kit initialization.

Functional parity comes first; the user deferred visual fidelity and polish. Retained screenshots remain the reference for later appearance work. Never infer native behavior from compilation or fixture images. Record real input, focus, directory selection, and platform results separately.

## Protect local data

Tests use isolated data roots and local bare Git remotes. The application uses a fresh `SkillBinder` directory beneath each operating system's local application-data directory and leaves `~/.skillbinder` untouched. Never run destructive tests against a user's library or sources.

Keep filesystem, hashing, SQLite, and Git work off the GPUI thread. UI code uses typed client operations. Recovery must finish before another durable mutation. Do not bypass source, revision, journal, or Git isolation checks to make a test pass.

Packaging does not authorize signing, publishing, or merging. Perform those actions only when the task explicitly permits them.
