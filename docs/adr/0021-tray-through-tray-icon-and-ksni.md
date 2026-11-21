# The menu bar menu is one typed model drawn by tray-icon, or by ksni on Linux

GPUI has no tray or status item API: neither upstream GPUI nor the `gpui-mcp` fork's `gpui-pre` 0.3.12 has one. The fork does provide what a resident app needs around it: `QuitMode::Explicit`, `set_activation_policy` (Regular or Accessory, macOS only), `on_reopen`, and `on_window_closed`. The menu has to be drawn natively on three platforms, kept current as the snapshot changes, and stay usable with no window open (ADR 0020).

**One model.** `crates/ui` derives the menu from the client's `AppSnapshot` and from the running blocking change, as a typed `TrayMenu`: ordered items, each with an action id, a label, enabled, and checked, plus whether the icon shows its dot. This is a pure function with unit tests. The backends only draw it. When the next model has the same item ids, they update labels, enabled, and checked in place; otherwise they rebuild the menu. A menu that stays open while a switch runs keeps its progress current.

**Backends.**

- **macOS and Windows:** `tray-icon` 0.26 with `muda` 0.21, maintained by Tauri. Both are created on the main thread inside `Application::run`, after launch, which macOS requires. GPUI's Win32 loop pumps every window on that thread, the tray's hidden window included.
- **Linux:** `ksni` 0.3, a pure-Rust StatusNotifierItem over D-Bus with no GTK. Spawning it fails with `WontShow` when no tray host exists, and `watcher_offline` reports a host that goes away. Either one marks the tray unavailable, so closing the window quits (ADR 0020).

The dependencies are target-gated, so Linux never builds `tray-icon` and the other platforms never build `ksni`.

**Events.** Menu clicks arrive on the backend's own callback thread. They are sent through a channel that a GPUI foreground task awaits, with no polling and no re-entry into `App` from inside an AppKit or Win32 callback. An action runs where the window runs it: through the open window's view, so the strip, the optimistic active preset, and the toast all behave as they do for the sidebar, or through the client when no window is open.

**Running work moves out of the view.** The menu must show work that the window may not be open to own. The running blocking change becomes a GPUI global in `crates/ui`. The view's `block` sets it and so does the menu, and the strip, `blocked()`, and the menu all read it, so a window opened partway through a switch shows the strip. Update all's progress and last outcome, and the running Sync or Push, move into the client and are published on the snapshot. The Library header, Restart's blockers, and the menu then read one truth. Update all's per-skill loop runs on the client's runtime, so closing the window never stops it.

**Lifecycle in `apps/desktop`.** The app runs with `QuitMode::Explicit`. Opening the window is one function, used at launch, by Open SkillBinder, by `on_reopen`, and by the instance lock's second-launch signal. It sets the Regular policy and activates the app. When the last window closes, the app sets the Accessory policy if the icon is shown, and quits otherwise.

## Considered Options

- `domenkozar/gpui-tray` was rejected. It is the closest prior art, with its own AppKit, Win32, and zbus backends, but it is young (12 stars), it pins `gpui-pre` 0.3.4 against the fork's 0.3.12, it has no radio or checked items without an extra feature, and adopting it means owning a fork when it falls behind.
- `tray-icon` with its `ksni` feature on Linux too was rejected. It hides whether a tray host exists, which ADR 0020 needs, to avoid running where the user cannot see the app.
- `tray-icon`'s default Linux backend was rejected. It needs GTK 3, libappindicator, and a GTK main loop, which GPUI's calloop loop does not run.
- Writing AppKit, Win32, and D-Bus code in the project was rejected. `unsafe_code` is forbidden in the workspace, and the two crates already carry that code with wide use.
- Keeping Update all's progress in the view and routing every menu action through a hidden window was rejected. GPUI has no hidden-window API, and a view closed partway through Update all would stop its loop.
- A GTK thread for the tray on Linux was rejected. Every update would have to cross into glib, and it adds GTK to an app that has none.

## Consequences

- New dependencies of `crates/ui`, target-gated: `tray-icon` and `muda` on macOS and Windows, and `ksni` on Linux. `xtask/src/graph.rs` and the third-party notices record them.
- The icon is the app's own mark as a monochrome template glyph, with a dot variant, under `assets/tray/`. AGENTS.md's icon rule gains it as a recorded exception beside the agent logos.
- Native menus cannot show preset colors or icons beside a check, so presets show by name.
- On Linux, GNOME without the AppIndicator extension shows no icon, and the app keeps quit-on-close there.
