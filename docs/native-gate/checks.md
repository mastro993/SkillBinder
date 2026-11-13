# Prerequisites and checks

## Prerequisites

Use Rust 1.100.0 from `rust-toolchain.toml` and Git 2.39 or newer.

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

Domain and integration tests use isolated storage and local bare Git remotes. They cover payload policy, source preservation, expiring grants and plans, scan retention, agent-folder refusal, import replay, organization revisions, organization staying machine-local, same-slug import refusal, concurrent writers, interruption recovery, reconciliation of skill directories after launch and Pull, remote adoption, divergence, unrelated-file protection, Git attribute and ignore rules, log redaction, and client publication ordering. Import activation tests cover first boot adopting matching copies without writing and filling the Default preset per agent, a different version found elsewhere replaced and listed in the plan, a later import activating in the active preset and in each workspace where found while other presets stay unchanged, a copy changed after review never overwritten, a linked copy left alone, and recovery saving an interrupted import's activations. Adoption tests cover first boot over a library left from before adopting its identical copies into Default, an import that fills the library using up the adoption, a library from before adopting, at launch, only the identical copies your agents have, never widening what the active preset holds and never adopting again, and the Pull that fills an empty library holding them in the active preset while a later Pull leaves what it brings in no preset. Activation tests write managed copies into a fixture home's global skill directories and cover one copy per shared directory, deactivation removing only that copy, adopting a matching copy without writing, a conflicting copy untouched until Replace and not asked again after Keep, an edited copy never removed or overwritten without asking, specific agents keeping their list while all your agents gains an agent detected later, an unwritable directory keeping the choice and applying on retry, a copy that cannot be removed staying managed until a retry removes it, switching and deleting the active preset swapping the copies while a same-slug copy SkillBinder does not manage stays untouched, an agent directory linked into SkillBinder's storage never receiving or losing a copy, a directory that became a link to another one forgotten without emptying its target, Update, Reinstall, and Pull rewriting managed copies and removing a removed skill's, and the active preset moving from stored preferences into the engine. Workspace activation tests, with the project folder outside the fixture home except in the removal and holder journeys, whose folders are inside it, cover adding a workspace adopting the library skills it holds without writing while slugs outside the library stay found and a directory only undetected agents read is left alone, a detected agent's own directory recorded and holding a skill alone, a differing copy found on add left untouched as a conflict until Replace, activation per directory, a detected agent's directory created and joining the workspace's agents while an undetected one is refused, a workspace without agents taking those of its first activation, removal keeping the files unmanaged and adding the workspace again adopting them, a missing workspace folder never recreated while its copies stay managed, also those of a directory linked inside it and when it is removed while missing, keeping a nested missing workspace's records and a global skill directory's linked into it, removal of a workspace whose directory became your agents', explicitly or when the engine opens, of one whose directory a missing nested workspace shares, of the last holder of a shared copy, and of one your agents stopped sharing, leaving their copies, a shared kept copy handed to the other holder, present or missing, a shared copy handed to the workspace still holding it when the other lets go, and an answer for a copy in a missing workspace refused until its folder returns, a directory linked out of the project never touched or swept, a directory a re-pointed or replaced link no longer leads to keeping its copies under home, also when another of the workspace's directories still leads there, an edited workspace copy asking before it is removed, an unwritable workspace directory keeping the choice until a retry, the overlap mark following the agents that read a skill both globally and in the workspace, including through another agent's directory, and following the reviewed order to say whether an agent may load it twice, hides the workspace copy, overrides your agents', or keeps one copy no source ranks, and Pull rewriting workspace copies and removing a removed skill's. Registry tests cover the overlay's directories being scanned with every reader recorded while detection and the shown global skill directory stay the registry's.

`xtask verify` checks the retained 79-agent registry and its skill-directory overlay, licensed assets, generated registry documentation, 88 canonical baseline images, and the workspace crate graph: members, allowed dependencies and their origins, feature maps that keep native test controls behind non-default `native-test`, that every member inherits workspace lints that forbid unsafe code and allowing the I/O policy back, that the workspace `clippy.toml` matches the list in `xtask/src/clippy_policy.rs`, that the only dependency override is the pinned `gpui-pre` backend patch, that `.cargo/config.toml` adds no compiler flags, compiler wrappers, subcommand aliases, test runners, or Clippy configuration, and that only `crates/engine`, `apps/desktop`, and `xtask` replace the workspace `clippy.toml` (no `.clippy.toml` anywhere). It does not establish native interaction or visual parity. The overlay step checks the reviewed bytes of `fixtures/contracts/registry-directories.json` and its three parts, its `registrySha256` against the registry, that each part holds only the agent-name range it declares, over ascending ranges, and that each entry sits in its part once per surface, that it keeps every registry directory and repeats no path after expansion, that workspace paths stay relative and inside the workspace, that global paths are templates or absolute, that only the three shared directories are `shared`, that `unconfirmed` marks a registry directory, that `order` ranks exactly the listed paths where a source gives an order and is absent otherwise, and that each source pins the full commit SHA a repository URL names or the date a website was retrieved.

CI `checks.yml` runs formatting, `cccc .` with `cccc-cli` 1.8.3 built by the pinned toolchain (cached across runs), Clippy with warnings denied (the workspace `clippy.toml` denies filesystem, process, network, and environment APIs outside the crates that own them), workspace tests, and `xtask verify`. Complexity limits stay in `cccc.toml`; do not raise them.

The native test feature provides isolated startup and a renderer smoke test. A `native-test` build never opens the user's own library or home: every mode requires `--data-dir`, which may not be the platform data directory, and `--home` may not be the real home; without `--home`, the home is `<data-dir>/fixture-home`. In an isolated run, the engine's Git commands read only the fixture home's Git configuration, never the user's or the system's, so a fixture never reaches the user's credential helpers, URL rewrites, or identity. SSH still reads the account's own `~/.ssh`, so fixtures use `file://` remotes.

```sh
cargo run -p skillbinder --features native-test --locked -- \
  --data-dir /tmp/skillbinder-native-test --home /tmp/skillbinder-native-home
cargo run -p skillbinder --features native-test --locked -- \
  --smoke-test --data-dir /tmp/skillbinder-native-smoke
```

The smoke test initializes only the supplied test directory and renders all four product screens and a GPUI Kit dialog across actual native frames. It checks Unicode text, input focus, modal focus capture and restoration, and refusal of implicit confirmation. Renderer error logs fail the check, including missing icon mappings. Errors from the platform backends (`gpui_linux`, `gpui_macos`, `gpui_windows`) and from wgpu's EGL adapter probe (`wgpu_hal::gles::egl`) describe host display, input, and driver conditions. They are printed without failing the check. For example, Xvfb exposes no pointer device, and nested Weston has no GL driver while Vulkan renders. The test then drains the engine and exits. Success prints `NATIVE_SMOKE_OK`. Test-only stage and renderer logs identify startup failures; Linux CI collects thread backtraces if a smoke process stalls. It does not simulate physical keyboard input, IME composition, or directory selection. Ordinary builds contain no test controls or bridge.

### Launch on the synthetic world

`fixtures/synthetic/` holds a small fictional machine: agents' global skill directories, projects, a skill source with a newer version, and a Sync remote, all with made-up content. Build it at a new directory and launch on it:

```sh
cargo run -p xtask --locked -- fixture /tmp/skillbinder-world
cargo run -p skillbinder --features native-test --locked -- \
  --data-dir /tmp/skillbinder-world/data --home /tmp/skillbinder-world/home
```

The task refuses a directory that is not empty and prints the launch command, the projects folder to add as workspaces, and the Sync remote's `file://` URL. Its Git commits carry fixed dates and author, so every build gives the same commits. `fixtures/synthetic/README.md` lists what the world holds and which states each part exercises. Add `--mcp` to drive it through the bridge below. The prototype-parity fixture in `docs/design/scoped-pages/fixture/` is separate: it reproduces the prototypes' data for visual comparison.

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

For Codex, run `codex mcp add gpui -- gpui-mcp --app-id skillbinder`. The server discovers the running window; the client configuration needs no endpoint path or token. The bridge starts only when both `native-test` and `--mcp` are present. `--mcp` conflicts with `--smoke-test`. The GPUI patch applies to every build, while the optional MCP bridge dependency stays outside the ordinary app graph. The quit hook stops the bridge before draining the engine. MCP automation proves native rendered interaction, not visual parity or packaged-app behavior.

The bridge advertises two app commands. `hold_activation` holds the progress strip or its end toast (see [activation progress](../features/native/activation-progress.md)). `add_workspace {"path": "<absolute folder>"}` adds a folder's projects as workspaces, as picking it in the system folder panel does, which the bridge cannot drive; it answers `Busy` while a change blocks the window, and a refusal shows in the window as an error.

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

The [native interaction checklist](interaction-checklist.md) lists the checks done by hand in the running app.
