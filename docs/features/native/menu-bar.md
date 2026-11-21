# Menu bar menu

SkillBinder keeps running behind an icon in the macOS menu bar, the Windows notification area, or a Linux StatusNotifier tray once its window closes ([spec](../../specs/menu-bar.md) rules 1–5, 8, 14; ADR 0020, ADR 0021; prototype `docs/design/menu-bar/prototype.html`, layout A). The architecture is in [Native menu bar](../../architecture/native-menu-bar.md). Slice S0 (#183) ships the resident app with a menu that holds Open SkillBinder and Quit SkillBinder, and S1 (#184) adds the presets and the attention item (rules 7.1, 7.3, 12); skill updates and the dot (S2) and app updates (S3) come later.

## The switch

Settings › General › **Show in menu bar** on macOS, **Show in system tray** on Windows and Linux, is a switch, on by default, with the line "Closing the window keeps SkillBinder running behind its icon." Turning it off removes the icon at once, and turning it on adds it. The value is saved in this device's preferences (`Preferences.show_in_tray`) and never syncs; preferences saved before it existed read as on.

## Staying resident

- With the icon shown, closing the window leaves the app running. With the switch off, or where the icon cannot show, closing the window quits, as before.
- On macOS, the app has no Dock icon and no ⌘-Tab entry while no window is open. Opening the window brings both back and brings the app forward.
- Quit SkillBinder in the menu, ⌘Q or Ctrl+Q, and logging off all quit the same way: the engine drains, and a ready app update installs ([App updates in place](app-updates.md)).
- Open SkillBinder, the Dock icon on macOS, a left click on the icon on Windows, and launching SkillBinder again each open the window when none is open, or bring the open one forward. A new window starts on Library, as at launch.
- Within one launch, a window opened again does not repeat what the first one showed once: the Move to Applications or Applications menu offer, the "Updated to" toast, and an app update notice the user dismissed.

## The menu

Items, top to bottom, each group shown only when it applies:

| Item | Does |
|---|---|
| `⚠ <n> skill copies need attention…` (`⚠ 1 skill copy needs attention…`), then a separator, while copies wait for the user | Opens the window, or brings it forward, on the review list |
| `Presets`, disabled, or while a blocking change runs, the strip's title and percentage, such as `Switching to Review… 40%` | — |
| Every preset by name, in sidebar order, the active one checked, then a separator | Switches to it |
| Open SkillBinder | Opens the window, or brings it forward |
| — | |
| Quit SkillBinder, with ⌘Q on macOS or Ctrl+Q elsewhere shown beside it | Quits |

Before onboarding finishes, the menu holds only Open SkillBinder and Quit SkillBinder.

The attention count is the Done pill's: every copy in the library that waits for the user, whichever change left it ([Activation progress](activation-progress.md#the-end)). The header's percentage is the snapshot's activation progress; it is left out while the client publishes none.

The icon's tooltip is "SkillBinder". On Windows, a left click opens the window and a right click shows the menu; on macOS and Linux every click shows the menu.

## Presets

Clicking a preset that is not active switches to it; clicking the active one does nothing. While anything the window refuses runs, every preset is disabled, in the window and in the menu alike: a blocking change from its start until its strip ends (working, landed but inside its minimum show time, or on the Done pill), and the window's short work, such as a single Update, an optimistic save, or Update all started in the window.

- **Window open:** the switch runs exactly as clicking the preset in the sidebar does, with the strip, the toast, and Done · Review.
- **No window:** the switch runs behind the icon without opening the window. The header shows its progress, and the presets are enabled again once it returns, whether it landed or was refused. There is no Done pill: copies left waiting show only as the attention item.
- **Window opened mid-switch:** the window shows the strip at once, at the switch's progress so far, and ends it as if it had started the switch: a toast, the Done pill, or a close with the error.
- **Window closed mid-switch:** the switch finishes behind the icon, and the presets stay disabled until it returns, with no Done pill. Closing the window under the Done pill, or while the strip waits out its minimum time, enables them at once.

## The icon

The icon is SkillBinder's rune, a stave bound by a diamond with a hook at each end, as a one-color glyph (`assets/tray/`). macOS draws it as a template image, recolored for the menu bar. Windows and Linux show a black glyph while the system appearance is light and a white one while it is dark, each with a faint halo of the other color, so it stays visible on a panel whose shade differs from the system's, such as GNOME's top bar. The glyph follows a change of appearance at the app's next refresh, such as the window regaining focus.

## Per platform

| Platform | Icon |
|---|---|
| macOS | Always available |
| Windows | Always available. A new icon starts in the notification area's overflow until the user pins it. |
| Linux with a StatusNotifier host (KDE, GNOME with the AppIndicator extension) | Shown. If the host goes away, the icon is gone and closing the window quits; with no window open the window opens, so the app never runs unseen. |
| Linux without a host (stock GNOME) | Not shown, and closing the window quits. Turning the switch off and on tries again. |

## Evidence

- **Unit:** `preferences_saved_before_the_tray_show_it` (C0.8); the client's preference round-trip saves `show_in_tray: false`; `tray::menu` tests for the items, labels, shortcut, and action ids, presets in sidebar order with only the active one checked and only Open and Quit before onboarding finishes (C1.1), the running switch's header and disabled presets (C1.2), the attention item's count and wording (C1.3), and presets disabled while the strip is up or the view is busy (C1.8, model side); `gate` tests that short work or any change blocks, that a change with no window unblocks as it returns, and that closing the window keeps only a running change (C1.8, C1.9); `tray::icon` tests that every artwork file decodes to its size and the glyph contrasts with the appearance.
- **Native, macOS, 2026-11-21** (native-test build, synthetic world, driven through the MCP bridge, System Events, and real clicks on the status item): closing the window kept the process and the status item, and turned the app from `Foreground` to `UIElement` (no Dock icon, no ⌘-Tab); Open SkillBinder opened a frontmost window on Library, back to `Foreground`, with the dismissed app update notice still hidden (C0.1, C0.2); the menu read Open SkillBinder, a separator, and Quit SkillBinder ⌘Q; Quit SkillBinder with no window open exited and installed the staged 0.2.0 fixture bundle (C0.3); the switch turned off removed the status item, closing the window then quit, a relaunch kept it off with no icon, and turning it on added the icon back, after which closing kept the app running (C0.4); a second launch with no window open opened the window (C0.5).
- **Not yet run:** Windows (C0.1, C0.7), Linux with and without a StatusNotifier host (C0.1, C0.6), and the packaged bundle, installer, and AppImage (C0.9). For S1, the native checks C1.4–C1.7 and C1.9 on every platform: the status item's menu is reached only through real pointer events. C1.10 and the Update all half of C1.8 wait for S2, which moves Update all into the client and adds the menu's Update all.
