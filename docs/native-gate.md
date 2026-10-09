# Verify the native application

The workspace implements the shipped product journeys with pinned GPUI Kit. Functional coverage is documented in [native features](features/native.md). Visual fidelity and polish are deferred by the user's latest instruction.

## Prerequisites

Use Rust 1.96.0 from `rust-toolchain.toml` and Git 2.39 or newer.

- macOS: Xcode and command-line tools, including Metal support.
- Windows: Visual Studio MSVC C++ tools, Windows SDK 10.0.20348.0 or newer, and CMake.
- Linux: C/C++ toolchain, Clang, CMake, Ninja, `pkg-config`, Fontconfig, Wayland, X11/XCB, XKBCommon, OpenSSL, and Vulkan libraries. Native directory selection requires a working desktop portal. The CI workflow lists Ubuntu package names.

GPUI Kit pins GPUI Pre 0.3.7 and enables the platform's `font-kit`, `wayland`, `x11`, and `runtime_shaders` features. Feature unification also enables X11 and Wayland in the shared GPUI backend. Linux startup requires an available desktop session and reports a clear error if neither display is available.

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

The smoke test initializes only the supplied test directory and renders all four product screens and a GPUI Kit dialog across actual native frames. It checks Unicode text, input focus, modal focus capture and restoration, and refusal of implicit confirmation. Renderer error logs fail the check, including missing icon mappings. Errors from the platform backends (`gpui_linux`, `gpui_macos`, `gpui_windows`) describe host display and input conditions and are printed without failing the check; for example, Xvfb exposes no pointer device. The test then drains the engine and exits. Success prints `NATIVE_SMOKE_OK`. Test-only stage and renderer logs identify startup failures; Linux CI collects thread backtraces if a smoke process stalls. It does not simulate physical keyboard input, IME composition, or directory selection. Ordinary builds contain no test controls or bridge.

### Run the opt-in MCP bridge

Install the pinned upstream server separately:

```sh
cargo install --git https://github.com/themixednuts/gpui-mcp \
  --rev 9dda8e5cb49990261e3fdaa26abe38112d30dafb \
  --locked gpui-mcp-server
```

Start the test build with a disposable data directory. MCP mode keeps onboarding in its normal state and leaves the window open until you quit the application.

```sh
cargo run -p skillbinder --features native-test --locked -- \
  --mcp --data-dir /tmp/skillbinder-mcp-fixture
```

Configure an MCP client to start the server:

```json
{"mcpServers":{"gpui":{"command":"gpui-mcp","args":["--app-id","skillbinder"]}}}
```

For Codex, run `codex mcp add gpui -- gpui-mcp --app-id skillbinder`. The server discovers the running window; the client configuration needs no endpoint path or token. The bridge starts only when both `native-test` and `--mcp` are present. `--mcp` requires `--data-dir`, conflicts with `--smoke-test`, and refuses the platform data directory or the real home as an explicit `--home` override. The GPUI patch applies to every build, while the optional MCP bridge dependency stays outside the ordinary app graph. The quit hook stops the bridge before draining the engine. MCP automation proves native rendered interaction, not visual parity or packaged-app behavior.

For a repeatable local interaction check, build the native test app and run the pinned server through `xtask`:

```sh
cargo build -p skillbinder --features native-test --locked
cargo run -p xtask --locked -- mcp-smoke \
  target/debug/skillbinder ~/.cargo/bin/gpui-mcp
```

The task creates an isolated fixture, selects the launched process by PID, completes onboarding, opens Sync, types and replaces text in the GPUI Kit Remote URL input, quits through the keyboard, and checks that the process leaves MCP discovery. The task uses Select All followed by `type_text` to exercise keyboard editing rather than replacing the input through a programmatic setter. Kit redirects semantic input focus to its inner editor, so the bridge can report the outer input's `focused` flag as false while typing succeeds. The check requires the replacement to appear in the intended input; the renderer smoke separately checks the editing state's real focus handle. The task keeps child processes bounded and stops them on failure. Run this check in a desktop session. MCP screenshot capture and visual parity remain separate checks. During the earlier macOS integration run, screen capture preflight returned false and screenshot capture returned `application window was not available for capture`.

CI checks macOS ARM64, macOS Intel, Windows x64, and Linux x64. Linux renderer checks run separately under X11 and Wayland. Wayland uses Weston nested in Xvfb so the compositor supplies an input seat; Weston's headless backend supplies no input and cannot exercise this desktop application. Each platform produces an unsigned package and checks the packaged executable's version. These checks remain distinct from interactive desktop and packaged-window testing.

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

The GPUI Kit 0.7.0 migration passed macOS ARM64 formatting, strict workspace Clippy, 82 workspace tests, two native launch-argument tests, retained-evidence verification, and a locked release build. The renderer smoke passed Unicode editing, real input focus, modal focus capture and restoration, and refusal of implicit confirmation. Two negative controls failed before their fixes: an unmapped close icon and Enter bypassing the explicit review action. The MCP smoke passed onboarding, Unicode typing and replacement, keyboard quit, and discovery cleanup. A separate native session created a folder, opened its deletion review, kept it open on Enter, traversed only modal controls with Tab and Shift+Tab, cancelled with Escape, then deleted it through its explicit action. Screenshot capture was unavailable. The dependency graph contains one GPUI Pre backend and excludes the MCP bridge from ordinary builds.

On 2026-10-09, a macOS ARM64 MCP session with an isolated data directory and fixture home completed the main journey on the GPUI Kit build. It covered four-step onboarding, a discovery scan of two fixture skills, page selection, reviewed import, Library search and file preview, folder creation, assignment, and deletion through its review dialog, Settings, and Sync. The Sync check connected a `file://` bare remote and pushed one commit containing both skills. After the push, the app was quit and relaunched. The relaunched app retained onboarding completion, the library, and the remote. A second instance then exited after activating the first. A plain filesystem path was rejected as a remote URL, as specified. Directory-picker roots, Pull, and shared-slug resolution were not exercised in this session.

The migration also produced an unsigned macOS app and DMG. The packaged executable returned `SkillBinder 0.1.0` for `--version`; packaged-window interaction was not repeated.

The remaining evidence below describes the original rewrite before the component migration. It does not establish the same interaction checks for GPUI Kit on Windows or Linux.

The baseline revision is `83d55ef789a7cf256021f6620742e0e9021a8312`. It was still current after fetching and rebasing before feature reconstruction.

On 2026-10-04, macOS ARM64 local checks passed for workspace formatting, strict Clippy, 82 domain, integration, and client tests, evidence verification, native debug compilation, and the locked release build. The real renderer smoke test passed, including the native quit hook. The hook drains the worker before AppKit termination; code after the native event loop is not relied on for shutdown. A subprocess-inheritance regression also verifies that shutdown explicitly releases library ownership before reopening. Portable-name policy is tested independently of the host filesystem, which may normalize or reject unsafe fixture names before inspection.

The MCP integration also passed those local checks and two native launch-argument tests. A real MCP session completed onboarding, navigated all four product screens, focused and replaced Unicode text in the previous component input, and quit successfully. Restart and second-instance activation preserved the existing library and published one endpoint; quit removed its descriptor. A native-test launch without `--mcp` published no endpoint, and the ordinary dependency graph excluded the bridge. The new bridge has not yet been verified on Windows or Linux.

Interactive macOS testing exercised four-step onboarding, native project selection, registry discovery, expired-plan refusal, reviewed import, retained results across navigation and restart, file preview, folder creation/assignment/deletion, remote Connect/Refresh/Push against a local bare remote, second-instance activation, and Cmd+Q. Source fixture bytes and the remote commit were independently checked. Sanitized Library and Sync screenshots accompany the rewrite PR. The packaged macOS production app rendered initial onboarding and exited cleanly; the unsigned DMG was created successfully.

The earlier dependency gate also verified directory-picker cancellation, dialog Tab/Shift+Tab and Escape restoration, retained icon artwork, initial 1180×760 outer window geometry, minimum 860×620 outer geometry, and final-window exit on macOS. That evidence does not establish every reconstructed screen's keyboard behavior.

Full cross-platform interactive journeys, IME composition, clipboard round trips, system appearance/reduced-motion changes, and exact visual comparisons remain separate unverified categories. CI results in the rewrite PR are the authority for current platform build, renderer, and package checks. Signing and publication are separate from this rewrite.
