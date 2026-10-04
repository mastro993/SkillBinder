# Verify the native application

The workspace implements the shipped product journeys with pinned GPUI and unmodified Ely. Functional coverage is documented in [native features](features/native.md). Visual fidelity and polish are deferred by the user's latest instruction.

## Prerequisites

Use Rust 1.96.0 from `rust-toolchain.toml` and Git 2.39 or newer.

- macOS: Xcode and command-line tools, including Metal support.
- Windows: Visual Studio MSVC C++ tools, Windows SDK 10.0.20348.0 or newer, and CMake.
- Linux: C/C++ toolchain, Clang, CMake, Ninja, `pkg-config`, Fontconfig, Wayland, X11/XCB, XKBCommon, OpenSSL, and Vulkan libraries. Native directory selection requires a working desktop portal. The CI workflow lists Ubuntu package names.

Both `gpui` and `gpui_platform` enable the `wayland` and `x11` features. At the pinned revision, the platform's X11 feature does not enable core X11 display detection; omitting the core feature selects a headless event loop under X11. Linux startup requires an available desktop session and reports a clear error if neither display is available.

## Automated checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo run -p xtask --locked -- verify
cargo build -p skillbinder --release --locked
cargo run -p xtask --locked -- package
```

Domain and integration tests use isolated storage and local bare Git remotes. They cover payload policy, source preservation, expiring grants and plans, scan retention, import replay, organization revisions, shared-slug backups, concurrent writers, interruption recovery, remote adoption, divergence, unrelated-file protection, Git attribute and ignore rules, log redaction, and client publication ordering.

`xtask verify` checks the retained 79-agent registry, licensed assets, generated registry documentation, and 88 canonical baseline images. It does not establish native interaction or visual parity.

The native test feature provides isolated startup and a renderer smoke test:

```sh
cargo run -p skillbinder --features native-test --locked -- \
  --data-dir /tmp/skillbinder-native-test --home /tmp/skillbinder-native-home
cargo run -p skillbinder --features native-test --locked -- \
  --smoke-test --data-dir /tmp/skillbinder-native-smoke
```

The smoke test initializes only the supplied test directory, renders all four product screens and an Ely dialog across actual native frames, checks Unicode text and focus, then drains the engine and exits. Success prints `NATIVE_SMOKE_OK`. Test-only stage and renderer logs identify startup failures; Linux CI collects thread backtraces if a smoke process stalls. It does not simulate physical keyboard input, IME composition, or directory selection. Ordinary builds contain no test controls or debug server.

CI checks macOS ARM64, macOS Intel, Windows x64, and Linux x64. Linux renderer checks run separately under X11 and Wayland. Each platform produces an unsigned package and checks the packaged executable's version. These checks remain distinct from interactive desktop and packaged-window testing.

## Native interaction checklist

Use disposable source directories, library data, and a local bare remote.

1. Complete all four setup steps; quit and reopen between steps to verify resumption.
2. Select a project directory through the native picker, register it, rename and enable/disable it, then verify direct agent skill discovery.
3. Scan, select candidates, acknowledge invalid metadata when applicable, review and apply an import. Navigate during work; retain the result until dismissed. Confirm original source bytes remain unchanged.
4. Search Library, inspect a file, create/rename a folder, assign skills, and delete the folder through its revision-checked preview. Resolve two distinct copies sharing a slug and inspect the backup.
5. Connect a disposable remote. Confirm Refresh does not commit, Push explicitly commits, Pull is fast-forward only, and Disconnect retains local files.
6. Type, select, copy/paste, and compose through an IME. Check Tab/Shift+Tab, dialog focus, Escape dismissal, pending dismissal locks, directory-picker cancellation, appearance, and reduced motion.
7. Resize the sidebar and window, restart, launch a second instance, and close the final window. Verify native activation, persisted data, and process exit.

## Recorded evidence

The baseline revision is `83d55ef789a7cf256021f6620742e0e9021a8312`. It was still current after fetching and rebasing before feature reconstruction.

On 2026-10-04, macOS ARM64 local checks passed for workspace formatting, strict Clippy, 82 domain, integration, and client tests, evidence verification, native debug compilation, and the locked release build. The real renderer smoke test passed, including the native quit hook. The hook drains the worker before AppKit termination; code after the native event loop is not relied on for shutdown. A subprocess-inheritance regression also verifies that shutdown explicitly releases library ownership before reopening. Portable-name policy is tested independently of the host filesystem, which may normalize or reject unsafe fixture names before inspection.

Interactive macOS testing exercised four-step onboarding, native project selection, registry discovery, expired-plan refusal, reviewed import, retained results across navigation and restart, file preview, folder creation/assignment/deletion, remote Connect/Refresh/Push against a local bare remote, second-instance activation, and Cmd+Q. Source fixture bytes and the remote commit were independently checked. Sanitized Library and Sync screenshots accompany the rewrite PR. The packaged macOS production app rendered initial onboarding and exited cleanly; the unsigned DMG was created successfully.

The earlier dependency gate also verified directory-picker cancellation, dialog Tab/Shift+Tab and Escape restoration, retained icon artwork, initial 1180×760 outer window geometry, minimum 860×620 outer geometry, and final-window exit on macOS. That evidence does not establish every reconstructed screen's keyboard behavior.

Full cross-platform interactive journeys, IME composition, clipboard round trips, system appearance/reduced-motion changes, and exact visual comparisons remain separate unverified categories. CI results in the rewrite PR are the authority for current platform build, renderer, and package checks. Signing and publication are separate from this rewrite.
