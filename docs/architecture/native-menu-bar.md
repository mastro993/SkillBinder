# Native menu bar

The app stays resident behind a menu bar or system tray icon once its window closes ([spec](../specs/menu-bar.md), [design](../specs/menu-bar/design.md), ADR 0020, ADR 0021; [feature](../features/native/menu-bar.md)). Slice S0 (#183) adds the resident lifecycle and a menu that opens the window and quits.

## Ownership

| Where | What |
| --- | --- |
| `crates/proto/src/lifecycle.rs` | `Preferences.show_in_tray`, `true` through `serde(default)` for preferences saved before it. |
| `crates/ui/src/tray.rs` | The `Tray` GPUI global. `start_tray` subscribes to the client's snapshots and adds or removes the icon as `show_in_tray` changes, and quits when it removes the icon with no window open, as when the switch is turned off and the window closed before the saved preference arrives; `tray_shown` tells whether it shows. Its state is `Hidden`, `Starting` while a backend registers the icon, `Shown` with the appearance its glyph was drawn for, or `Unavailable` until the preference is turned off and on again. |
| `crates/ui/src/tray/menu.rs` | The pure `TrayMenu` model: ordered `TrayItem`s, each an action with a label and an optional shortcut key, or a separator. `TrayAction` ids are stable strings. |
| `crates/ui/src/tray/icon.rs` | Decodes the PNGs in `assets/tray/` with `png` into RGBA: the template on macOS, the 32 px glyph on Windows, and every size on Linux, black or white by `App::window_appearance`. |
| `crates/ui/src/tray/native.rs` (not Linux) | Builds the icon with `tray-icon` and its menu with `muda` on the main thread, inside `Application::run`, as macOS requires. The template is set with `with_icon_templated`. Menu clicks arrive through `MenuEvent::set_event_handler`, and on Windows a left click through `TrayIconEvent::set_event_handler`; both handlers are process-wide, set once by `start_tray`. |
| `crates/ui/src/tray/linux.rs` | Serves a `ksni` StatusNotifierItem (blocking API on `async-io`), spawned on GPUI's background executor because D-Bus may be slow to answer. Any `spawn` error leaves the tray `Unavailable`. `watcher_offline` reports the host gone and stops the service. `MENU_ON_ACTIVATE` makes every click open the menu. |
| `apps/desktop/src/window.rs` | The `MainWindow` global: the client, the open window's handle, whether a window ever opened, and the native-test hooks. `open_main_window` brings the open window forward, or sets the Regular activation policy, opens a window on Library, and activates the app. `window_closed` forgets the handle, and when no window is left sets the Accessory policy if `tray_shown`, or quits. |
| `apps/desktop/src/main.rs` | Runs with `QuitMode::Explicit`. Launch, the tray's Open SkillBinder, `on_reopen`, and the instance lock's second-launch signal all call `open_main_window`. The quit hook is unchanged. |

## Events

A backend never touches `App`. Its callbacks send a `TrayEvent` into an unbounded `async-channel`, which a foreground task spawned by `start_tray` awaits: Open SkillBinder calls the `open_window` closure `apps/desktop` passed in, Quit SkillBinder calls `App::quit`, and a lost Linux host marks the tray `Unavailable` and opens the window when none is open.

## Windows of one launch

`NativeView` is created per window. What the app update notice has shown this launch, `LaunchNotice`, is a GPUI global that the view writes when it is created and when it dismisses the notice or announces the update; a later window resumes it, and offers neither Move to Applications nor the Applications menu, which show in the launch's first window only. Under `native-test`, every window gets its own MCP bridge, which stops when the window closes, so the bridge sees no app while no window is open.

## Dependencies

`tray-icon` 0.26 (default features off, so no GTK) is a `cfg(not(target_os = "linux"))` dependency of `crates/ui`, and `ksni` 0.3 (`blocking`, `async-io`) a `cfg(target_os = "linux")` one; `png` and `async-channel` are plain ones. `xtask/src/graph.rs` allows them for `skillbinder-ui` only. Neither backend needs a denied standard-library API in the project's own code.
