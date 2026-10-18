# Prerequisites and checks

## Prerequisites

Use Rust 1.96.0 from `rust-toolchain.toml` and Git 2.39 or newer.

- macOS: macOS 26 or newer, with Xcode 26 or newer and its command-line tools, including Metal support. `.cargo/config.toml` sets `MACOSX_DEPLOYMENT_TARGET` to 26.0, and the packaged app declares the same `LSMinimumSystemVersion`.
- Windows: Visual Studio MSVC C++ tools, Windows SDK 10.0.20348.0 or newer, and CMake.
- Linux: C/C++ toolchain, Clang, CMake, Ninja, `pkg-config`, Fontconfig, Wayland, X11/XCB, XKBCommon, OpenSSL, and Vulkan libraries. Native directory selection requires a working desktop portal. The CI workflow lists Ubuntu package names.

GPUI Kit pins GPUI Pre 0.3.7 and enables the platform's `font-kit`, `wayland`, `x11`, and `runtime_shaders` features. Feature unification also enables X11 and Wayland in the shared GPUI backend. Linux startup requires an available desktop session and reports a clear error if neither display is available.

## Automated checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cccc .
cargo test --workspace --all-targets --locked
cargo run -p xtask --locked -- verify
cargo build -p skillbinder --release --locked
cargo run -p xtask --locked -- package
```

Domain and integration tests use isolated storage and local bare Git remotes. They cover payload policy, source preservation, expiring grants and plans, scan retention, agent-folder refusal, install replay, organization revisions, organization staying machine-local, same-slug install refusal, concurrent writers, interruption recovery, reconciliation of skill directories after launch and Pull, remote adoption, divergence, unrelated-file protection, Git attribute and ignore rules, log redaction, and client publication ordering.

`xtask verify` checks the retained 79-agent registry, licensed assets, generated registry documentation, 88 canonical baseline images, and the workspace crate graph: members, allowed dependencies and their origins, feature maps that keep native test controls behind non-default `native-test`, that every member inherits workspace lints that forbid unsafe code and allowing the I/O policy back, that the workspace `clippy.toml` matches the list in `xtask/src/clippy_policy.rs`, that the only dependency override is the pinned `gpui-pre` backend patch, that `.cargo/config.toml` adds no compiler flags, compiler wrappers, subcommand aliases, test runners, or Clippy configuration, and that only `crates/engine`, `apps/desktop`, and `xtask` replace the workspace `clippy.toml` (no `.clippy.toml` anywhere). It does not establish native interaction or visual parity.

CI `checks.yml` runs formatting, `cccc .` with `cccc-cli` 1.8.3 built by Rust 1.99.1 (cached across runs), Clippy with warnings denied (the workspace `clippy.toml` denies filesystem, process, network, and environment APIs outside the crates that own them), workspace tests, and `xtask verify`. Complexity limits stay in `cccc.toml`; do not raise them.

The native test feature provides isolated startup and a renderer smoke test:

```sh
cargo run -p skillbinder --features native-test --locked -- \
  --data-dir /tmp/skillbinder-native-test --home /tmp/skillbinder-native-home
cargo run -p skillbinder --features native-test --locked -- \
  --smoke-test --data-dir /tmp/skillbinder-native-smoke
```

The smoke test initializes only the supplied test directory and renders all four product screens and a GPUI Kit dialog across actual native frames. It checks Unicode text, input focus, modal focus capture and restoration, and refusal of implicit confirmation. Renderer error logs fail the check, including missing icon mappings. Errors from the platform backends (`gpui_linux`, `gpui_macos`, `gpui_windows`) and from wgpu's EGL adapter probe (`wgpu_hal::gles::egl`) describe host display, input, and driver conditions. They are printed without failing the check. For example, Xvfb exposes no pointer device, and nested Weston has no GL driver while Vulkan renders. The test then drains the engine and exits. Success prints `NATIVE_SMOKE_OK`. Test-only stage and renderer logs identify startup failures; Linux CI collects thread backtraces if a smoke process stalls. It does not simulate physical keyboard input, IME composition, or directory selection. Ordinary builds contain no test controls or bridge.

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

The task creates an isolated fixture, selects the launched process by PID, completes onboarding, drags the sidebar resize handle and requires the sidebar to widen, hides and shows the sidebar from the header toggle, switches the theme from the sidebar toggle beside Settings and back, opens Settings from its sidebar row, opens Sync, types and replaces text in the GPUI Kit Remote URL input, quits through the keyboard, and checks that the process leaves MCP discovery. The task uses Select All followed by `type_text` to exercise keyboard editing rather than replacing the input through a programmatic setter. Kit redirects semantic input focus to its inner editor, so the bridge can report the outer input's `focused` flag as false while typing succeeds. The check requires the replacement to appear in the intended input; the renderer smoke separately checks the editing state's real focus handle. The task keeps child processes bounded and stops them on failure. Run this check in a desktop session. MCP screenshot capture and visual parity remain separate checks. During the earlier macOS integration run, screen capture preflight returned false and screenshot capture returned `application window was not available for capture`.

The `Checks` workflow gates pull requests and also runs on `main`. One Linux x64 job checks formatting and retained evidence, then runs strict Clippy and workspace tests with all features. It runs no platform-specific checks and builds no application.

The `Native builds` workflow runs only after changes reach `main`, or on manual dispatch. It does not gate pull requests; a failure signals that `main` no longer builds on a supported platform. On macOS ARM64, macOS Intel, Windows x64, and Linux x64, it runs strict Clippy and workspace tests, builds the native test application, runs the renderer smoke, and builds the locked release application. Both macOS jobs run on `macos-26`, because GitHub offers no Intel macOS 26 runner. The Intel job cross-compiles `x86_64-apple-darwin`, then runs its tests and renderer smoke under Rosetta. Rosetta does not replace testing on Intel hardware. Each macOS release binary must declare a minimum system version of 26.0. Linux renderer checks run separately under X11 and Wayland. Wayland uses Weston nested in Xvfb so the compositor supplies an input seat; Weston's headless backend supplies no input and cannot exercise this desktop application. No workflow produces packages yet; run `cargo run -p xtask --locked -- package` locally after a release build. These checks remain distinct from interactive desktop and packaged-window testing.

## Native interaction checklist

Use disposable source directories, library data, and a local bare remote.

1. Complete all four setup steps; quit and reopen between steps to verify resumption.
2. Select a project directory through the native picker, register it, rename, reorder, and remove it, then verify direct agent skill discovery. Pick the home folder and confirm that only the projects in it are added, or that the pick is refused when it holds none.
3. Scan, select candidates, acknowledge invalid metadata when applicable, review and apply an install. Navigate during work; retain the result until dismissed. Confirm original source bytes remain unchanged.
4. Search Library, inspect a file, create, rename, recolor, and reorder a preset, and delete it through its revision-checked preview. Confirm that installing a different skill with a slug the library holds is refused.
5. Connect a disposable remote. Confirm Refresh does not commit, Push explicitly commits only `skills/`, preset and folder changes never show as changes, Pull is fast-forward only, and Disconnect retains local files.
6. Type, select, copy/paste, and compose through an IME. Check Tab/Shift+Tab, dialog focus, Escape dismissal, pending dismissal locks, directory-picker cancellation, appearance, and reduced motion.
7. Resize, hide, and show the sidebar with the pointer, header toggle, and Cmd+B or Ctrl+B. Resize the window, restart, confirm the sidebar width and hidden state persist, launch a second instance, and close the final window. Verify native activation, persisted data, and process exit.
