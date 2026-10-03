# Verify the native dependency gate

This stage proves that the exact dependency graph can build and support native interactions. It does not implement the product journeys.

## Install platform prerequisites

On macOS, install Xcode and its command-line tools, including Metal support.

On Windows, use a Visual Studio developer shell with the MSVC C++ tools, Windows SDK 10.0.20348.0 or newer, and CMake.

On Debian or Ubuntu, install the C and C++ toolchain, Clang, CMake, Ninja, `pkg-config`, Fontconfig, Wayland, X11/XCB, XKBCommon, OpenSSL, and Vulkan libraries. Native directory selection also requires a working desktop portal. The CI workflow lists its package names.

Use the Rust 1.96.0 toolchain pinned by `rust-toolchain.toml`. Upstream's repository toolchain is newer, so compile the actual graph before changing this pin.

## Exercise the real application

1. Run `cargo build -p skillbinder --features native-test --locked`.
2. Run `cargo run -p skillbinder --features native-test --locked` in an interactive native desktop session.
3. Confirm the standard SkillBinder titlebar, initial 1180 by 760 outer window, and minimum 860 by 620 outer window. The macOS baseline client area is 1180 by 728, with a minimum of 860 by 588. Record differences on other operating systems.
4. Type, select, copy, paste, and compose text through an installed IME. Confirm that text input retains focus correctly.
5. Use Tab and Shift+Tab between controls. Open the dialog, confirm focus remains inside it, then press Escape and confirm focus returns to the opener.
6. Open the directory picker. Cancel once, then choose an isolated test directory. Confirm the selected path appears in the field. The gate does not write to that directory.
7. Inspect the check, folder, and dialog-close icons. They must use the retained Hugeicons artwork.
8. Change system appearance and confirm the window updates. Capture light and dark images and record the display scale.
9. Close the last window and confirm the process exits.

Repeat these checks on macOS, Windows, Linux X11, and Linux Wayland. Compilation alone does not prove a working native session.

## Verification status

The canonical macOS baseline contains 88 images, their geometry and accessibility records, and separate development interaction evidence. `xtask verify` checks integrity and retained registry data.

On 2026-10-03, the macOS ARM64 graph built with Rust 1.96.0 in debug, native-test, and release configurations. Formatting, all-target and all-feature Clippy, the workspace test command, and `xtask verify` passed. There are no domain tests at this foundation stage; the test command runs zero tests.

A task-local native app bundle exercised real text entry, retained Hugeicons artwork, native directory selection and cancellation, dialog Tab and Shift+Tab focus, Escape dismissal and focus restoration, and last-window process exit. Native captures confirmed 1180 by 760 initial outer dimensions and an enforced 860 by 620 minimum. An initial fractional centered origin added one pixel to the native frame; rounding the origin to logical pixels fixed the measured dimensions.

Additional clipboard round-trip and appearance checks could not complete. A system automation query timed out and the computer-use tool refused access to the foreground system notification app. The prompt was left untouched, the original clipboard was restored and verified, no appearance or input-source settings changed, and the test app was closed. Selection, copy/paste, IME composition, and appearance changes remain unverified.

Windows and Linux native runners are not available on the current host. The CI workflow defines build checks for macOS, Windows, and Linux. Pull request checks record each run's outcome. The all-platform dependency gate remains open until native build and interaction evidence is available for every required platform.

Product feature reconstruction, full UI parity, native IME verification, Linux runtime checks, packaged-app smoke tests, signing, and publication remain incomplete.

The pinned public GPUI and Ely APIs have not established cross-platform backdrop blur matching the existing modal overlays. A dim dialog in the gate does not resolve that visual requirement. Preserve this as a parity blocker until a verified implementation or an explicit requirement change resolves it.
