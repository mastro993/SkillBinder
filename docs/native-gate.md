# Verify the native application

The workspace implements the shipped product journeys with pinned GPUI Kit. Functional coverage is documented in [native features](features/native.md). Visual fidelity and polish are deferred by the user's latest instruction.

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

Domain and integration tests use isolated storage and local bare Git remotes. They cover payload policy, source preservation, expiring grants and plans, scan retention, agent-folder refusal, install replay, organization revisions, shared-slug backups, concurrent writers, interruption recovery, remote adoption, divergence, unrelated-file protection, Git attribute and ignore rules, log redaction, and client publication ordering.

`xtask verify` checks the retained 79-agent registry, licensed assets, generated registry documentation, 88 canonical baseline images, and the workspace crate graph (members, allowed dependencies, and source imports that would skip crate ownership). It does not establish native interaction or visual parity.

CI `checks.yml` runs formatting, `cccc .` with `cccc-cli` 1.8.3, Clippy with warnings denied, workspace tests, and `xtask verify`. Complexity limits stay in `cccc.toml`; do not raise them.

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
4. Search Library, inspect a file, create, rename, recolor, and reorder a preset, and delete it through its revision-checked preview. Resolve two distinct copies sharing a slug and inspect the backup.
5. Connect a disposable remote. Confirm Refresh does not commit, Push explicitly commits, Pull is fast-forward only, and Disconnect retains local files.
6. Type, select, copy/paste, and compose through an IME. Check Tab/Shift+Tab, dialog focus, Escape dismissal, pending dismissal locks, directory-picker cancellation, appearance, and reduced motion.
7. Resize, hide, and show the sidebar with the pointer, header toggle, and Cmd+B or Ctrl+B. Resize the window, restart, confirm the sidebar width and hidden state persist, launch a second instance, and close the final window. Verify native activation, persisted data, and process exit.

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

On 2026-10-10, a macOS ARM64 MCP session with an isolated data directory verified the shell redesign. It checked full-window rendering under the transparent titlebar during onboarding and in the product shell. The sidebar toggle hid and showed the sidebar, as did Cmd+B. Drag resizing reached both clamps, 192 and 400. Width and hidden state each survived a restart. The extended `mcp-smoke` passed. A negative control failed it: the drag listener was reverted to the pre-fix payload type, which never matched the handle's `Rc`-wrapped drag value, so drag resizing had silently never worked. Windows and Linux shells, dark appearance, and window dragging from the header were not exercised. A later revision pinned the toggle beside the traffic lights and animated opening and closing. Unit tests cover its width and header-inset curves, and the renderer smoke passed on it. An MCP session on that revision could not render frames because the test window was not visible on the active desktop, so the motion has not been visually verified.

On 2026-10-11, a macOS ARM64 MCP session with an isolated data directory verified the sidebar redesign. Onboarding, scan, and import of six fixture skills populated the counts. The session created folders inline, recolored one from its context menu, moved it with Move Down and by dragging, and canceled an inline rename with Escape. It checked that long names truncate, that a folder row opens its filtered Library, that Cmd+[ returned to the previous folder route, and that hiding the sidebar removes Back and Forward. It also checked light and dark themes. Pixel measurements of a 2× dark capture matched the reference screenshot's surface and selection colors, row pitch, icon and label offsets, section spacing, label size, and trailing column within one point. Sidebar width differs because it is a user preference. The renderer smoke passed. A debug build first aborted when a folder row was right-clicked. GPUI's accessibility tree reported two focused nodes in one frame: the focused row, and the context menu that takes focus while being drawn. Right-click now leaves focus on the view root, and the menu opened without error. Windows and Linux were not exercised.

On 2026-10-12, a macOS ARM64 MCP session with an isolated data directory verified animated folder reordering. Native pointer drags were captured mid-gesture and in a screen recording. The dragged folder floated under the pointer with a shadow, its slot stayed empty, and the rows it passed slid aside. The release saved the shown order to `.skillbinder.json`, and a plain click on a folder still opened its filtered Library. Reduced motion was not exercised. The same session checked the folder context menu in both themes: every item showed its glyph, Color choices showed tinted folders with the check mark on the right, Delete… used the danger color and opened the deletion preview, and Move Up moved the folder one slot.

Full cross-platform interactive journeys, IME composition, clipboard round trips, system appearance/reduced-motion changes, and exact visual comparisons remain separate unverified categories. CI results in the rewrite PR are the authority for current platform build, renderer, and package checks. Signing and publication are separate from this rewrite.

On 2026-10-12, a macOS ARM64 MCP session with an isolated data directory and fixture home verified Global (then named Root and listed first under Projects) and projects. After onboarding and an import of two fixture skills, clicking that row selected it and its rescan recorded both as deployments. The Projects add button opened the native directory picker, driven through System Events to a fixture folder. A skeleton row and the scan modal appeared while a 115,000-folder fixture project was scanned, and Cancel closed both and saved nothing. A second project finished with "1 in library, 1 importable" and was saved; a third was added the same way. Picking an already added folder was refused with a message naming the project. Dragging a project row and Move Down each saved a new order, and an inline rename saved with Enter. Moving a project folder away showed the warning glyph and kept its deployment. Remove… showed the confirmation with the deployment count, removed the project, and left its files byte-identical. Skill details listed the targets that hold the skill. After a relaunch, the launch rescan recorded a copy added while the app was closed. The renderer smoke passed. Windows and Linux were not exercised.

On 2026-10-12, a macOS ARM64 MCP session with an isolated data directory and fixture home verified Discovery aggregation. The fixture held an identical `review` skill in the home's `.claude/skills` and in a project's `.agents/skills`, and a `lint` skill with different content in each. Before this change Discovery listed four entries, with `review` twice. After it, Discovery listed three: one `review` entry naming both locations with each location's readers, and two separate `lint` entries. Selecting `review` and opening Review import showed one source; importing it added one library skill, and Scan again hid the entry and reported "1 already in library" while both `lint` entries stayed. No deployment was recorded, and the source copies and the library copy were byte-identical. MCP screenshot capture timed out after 15 seconds, so the screenshots were taken with `screencapture` of the window. Windows and Linux were not exercised.

On 2026-10-12, a macOS ARM64 MCP session with an isolated data directory and fake home under `/tmp` verified the move of Root, renamed Global, out of the Projects section. The fixture was seeded through the engine API instead of the native picker: a completed onboarding, `review` and `lint` in the home's `.claude/skills`, and the projects `web` and `api`. The sidebar listed Discovery, Library, and Global, in that order, and the Projects section listed only `web` and `api`. Global showed a muted `~` as its trailing detail, and hovering it showed the canonical fixture home path as a tooltip. Clicking Global selected its row and opened its page, titled Global, with Claude Code's global skill directory and both deployments. After the home's `review` copy was edited, clicking Global again rescanned it; the row showed the warning dot, its label read "Global, 1 deployment drifted", and the page offered Restore from library and Accept into library. The MCP `screenshot` timed out after 15 seconds, so the screenshots come from macOS `screencapture` of the app window. Windows and Linux were not exercised, and visual parity is still deferred.

On 2026-10-14, two macOS ARM64 MCP sessions with isolated data directories and fixture homes under `/tmp` verified the removal of the System workspace. Onboarding showed the new Boundaries text. The Workspaces section had no System row and showed "No workspaces"; the Agents section listed Universal, Claude Code, and Cursor with one found skill each. Discovery showed "Skills found in your agents and workspaces that are not installed." and "2 found · 0 already installed". Review install opened the "Review install" dialog with Back and Install, and Install reported "Install finished: 1 copied, 0 already installed, 0 name conflicts" and then "1 found · 1 already installed". Library showed `review` with "Installed just now · Modified just now" and the Universal and Claude Code chips; the Universal chip opened Universal's page, which listed `review` as Installed with Show in Library. Removing the fixture's `~/.cursor` folder while Cursor's page was open and then refocusing the window removed Cursor's row and opened Discovery. In the second session `~/.claude/skills/review` was a link to `~/.agents/skills/review`; Library showed both the Universal and Claude Code chips, and Claude Code's page listed `review` as installed. Adding a workspace through the native picker, including the home-folder refusal, was not exercised here; engine tests cover it. MCP screenshot capture timed out after 15 seconds in the second session, which was checked through the semantic tree. Windows and Linux were not exercised, and visual parity is still deferred.

On 2026-10-14, a macOS ARM64 MCP session with an isolated data directory and fixture home under `/tmp` verified the rename of folders to presets. After onboarding and an install of two fixture skills, the sidebar showed the Presets section with its Add preset control, the inline field read "Preset name", and Research and Writing were created. The library was then rewritten as schema 2, with a green Research folder holding `review`, and a stored preference that folded `folders`. On relaunch the Presets section was folded; unfolded, it showed Research in green with one skill, and Library › Research listed `review`. Reading left the file byte-identical. The first write, Move Up, saved schema 3 with `presetIds`, the color, and the order, and without `folders` or `folderId`. With `review` in both presets and `notes` in Writing, the rows showed Writing 2 and Research 1. Deleting Writing showed "Delete Writing? Its 2 skills stay in the library." with Cancel and Delete preset; confirming opened Library with both skills, and Research still held `review`. Preset rows keep the Hugeicons folder glyph until the polish pass. The session does not establish visual parity.

On 2026-10-15, a macOS ARM64 MCP session with an isolated data directory and a fixture home under `/private/tmp` verified the skill page. After onboarding and the install of one fixture skill, clicking its Library row opened the page with the breadcrumb Library › pdf-tools while the Library sidebar row stayed selected. The page rendered the skill's `SKILL.md`: its frontmatter as a table, then headings, lists, a code block, links, and a table. Clicking the Library row returned to the list. Window captures were taken with `screencapture -l`, because MCP screenshot capture exceeded its deadline; while the window was in the background, GPUI did not render new frames and MCP frame waits timed out. The agent page's Show in Library link to the skill page was not exercised natively.

On 2026-10-15, a macOS ARM64 MCP session with an isolated data directory and a fixture home under `/tmp` verified library folders. After onboarding and the install of five fixture skills, Library showed the header tabs All skills, a divider, and Add folder in place of the breadcrumb, with All skills chosen. Two folders were created inline with Enter and saved to `libraryFolders` in creation order. Right-clicking the `review` row opened Move to folder, whose submenu checked None; choosing Code review saved `libraryFolderId`, and under All skills the row's dates line then began with the folder glyph and "Code review". Choosing the Code review tab listed only `review`, with the count at one. Double-clicking the Writing tab chose it and opened its name field without zooming the window; renaming it to Docs saved, and the empty tab read "No skills in Docs". Deleting the chosen Code review tab from its menu showed "Delete Code review? Its 1 skills stay in the library."; confirming removed it, left `review` unfiled, and returned the page to All skills. Window captures were taken with `screencapture -l`, because MCP screenshot capture exceeded its deadline, and MCP frame waits timed out until the window was brought to the front. Tab overflow scrolling, a Sync removing the chosen folder, and the light theme were not exercised. The session does not establish visual parity.
