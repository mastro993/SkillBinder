# Menu bar menu: acceptance and slices

Behavior: [menu-bar.md](../menu-bar.md). Design: [design.md](design.md).

Evidence per check is labeled as AGENTS.md requires: **unit** (Rust tests), **native** (interactive check on the running app, recorded), and **packaged** (smoke test of a release build).

## Slices

Implement the slices in order. Each one ships alone.

### S0 · Resident app with a minimal menu

The icon appears with `Open SkillBinder` and `Quit SkillBinder`. Closing the window keeps the app running. The Settings switch turns it off. Depends on nothing.

- C0.1 **native, all three OS:** with the switch on, closing the window leaves the icon, and the app keeps running. Open SkillBinder opens a window on Library.
- C0.2 **native macOS:** with the window closed there is no Dock icon and no ⌘-Tab entry. Open SkillBinder brings both back, and the window comes forward.
- C0.3 **native:** Quit SkillBinder from the menu drains and exits. A staged fixture app update installs, through the same path as ⌘Q (ADR 0018 test install location).
- C0.4 **native:** Settings › Application › Show in menu bar (macOS), or Show in system tray, off: the icon goes away. Closing the window then quits. On again: the icon returns. The value survives a relaunch.
- C0.5 **native:** launching SkillBinder a second time while it runs without a window opens the window.
- C0.6 **native Linux:** with no StatusNotifier host (no AppIndicator extension on GNOME), there is no icon, and closing the window quits. With one (KDE, or GNOME with the extension), C0.1 holds.
- C0.7 **native Windows:** a left click on the icon opens the window, and a right click shows the menu.
- C0.8 **unit:** `Preferences` without `show_in_tray` deserializes to `true`.
- C0.9 **packaged:** the macOS app bundle, the Windows installer, and the AppImage each show the icon on first launch after onboarding.

Docs: `docs/architecture/native.md`, where "last-window closure" in the quit paragraph becomes the resident lifecycle and the dependency boundary gains the tray crates. A new `docs/features/native/menu-bar.md`. The AGENTS.md icon exception. `THIRD_PARTY_NOTICES.md`.

### S1 · Presets in the menu

Depends on S0.

- C1.1 **unit:** the model lists presets in sidebar order, checks only the active one, and shows `Presets` as the header. Before onboarding finishes, it shows only Open and Quit.
- C1.2 **unit:** while the `Gate`'s change is `Switching { preset: "Review" }` and the activation progress is 40%, the header reads `Switching to Review… 40%`, and every preset is disabled.
- C1.3 **unit:** with copies waiting, the first item reads `⚠ 2 skill copies need attention…` (`1 skill copy needs attention…`). The count is the strip's count.
- C1.4 **native, window closed:** clicking an inactive preset switches it without opening the window. Reopening the menu during the switch shows the progress, and afterwards the new preset is checked. Agent copies match the preset.
- C1.5 **native, window open:** clicking a preset in the menu shows the window's strip and toast, exactly as clicking it in the sidebar does.
- C1.6 **native:** opening the window while a menu-started switch runs shows the strip until the switch lands.
- C1.7 **native:** a switch that leaves a conflict shows the attention item. Clicking it opens the window on the review list.
- C1.8 **unit:** `blocked()` is `gate.busy || gate.change.is_some() || snapshot.updating.is_some()`. The model disables presets and Update all while the Done pill shows, while the view is `busy`, and while an Update all runs, whether it started from the window or from the menu. Dispatch refuses a second switch while one runs.
- C1.9 **native:** closing the window during a window-started switch leaves the presets disabled until the switch lands, then enabled, with no Done pill. Closing it under the Done pill enables them at once. Copies left waiting show as the attention item.
- C1.10 **native:** after an Update all starts from the menu, closing and reopening the window still refuses a preset switch in the window and in the menu until the run ends.

### S2 · Skill updates in the menu, and the dot

Depends on S0. It is independent of S1.

- C2.1 **unit:** the status line follows rule 9's order for each case: no Git, updating, failed outcome, n updates (plural and singular), checking, up to date.
- C2.2 **unit:** `Client::update_all` publishes `updating` after each skill, scans once at the end, records the outcome, and refuses to start a second run while one runs (engine fixture with a local bare remote).
- C2.3 **native, window closed:** with fixture skills that have updates, Update all from the menu updates them. The menu counts progress. Library shows the updated skills on reopen.
- C2.4 **native, window open:** Update all from the menu shows Library's header ring, and closing the window midway does not stop the run.
- C2.5 **unit:** the hourly check timer calls `run_check` with the one-hour interval (paused tokio clock).
- C2.6 **unit:** `dot` is true for skill updates, for waiting copies, and for an app update that is ready or download-only, and false otherwise.
- C2.7 **native:** the icon shows the dot variant while skills have updates, and the plain icon once they are updated.
- C2.8 **unit:** Restart's blockers read `snapshot.syncing` and `snapshot.updating`. Library's header reads the snapshot. Nothing in `NativeView` keeps `updating` or `syncing`. An Update all from the menu with the window open shows its outcome on the page, and the outcome clears on navigation, as today.

Docs: `docs/features/native/library-and-organization.md` (Update all runs in the client), and `docs/architecture/native.md` (running work published on the snapshot).

### S3 · App updates in the menu

Depends on S0. The dot rule for app updates comes from S2.

- C3.1 **unit:** the app update group shows each of rule 13's states with its exact label and enabled state, Off included.
- C3.2 **native:** Check for app updates against a fixture manifest with a newer version shows Checking, then Downloading with its percentage, then `Restart to install SkillBinder <v>`. Restart installs the fixture and relaunches it.
- C3.3 **native:** during an Update all, the Restart item is disabled.
- C3.4 **native:** download-only (the app run from outside Applications) shows `Download SkillBinder <v>…`, which opens the release page.

Docs: `docs/features/native/app-updates.md` (the menu entry points).

## Done

All four slices have merged, every check has its evidence recorded in `docs/features/native/acceptance-evidence*.md`, and `docs/native-gate.md` lists the tray's verification per platform.
