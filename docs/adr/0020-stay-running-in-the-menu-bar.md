# SkillBinder stays running in the menu bar when its window closes

Today the app quits when its last window closes, and finding skill updates, switching the active preset, or checking for an app update all mean opening the window and going to Library, the sidebar, or Settings. Users who switch presets several times a day pay that cost every time. So SkillBinder gets a menu bar menu: an icon in the macOS menu bar, the Windows notification area, or a Linux StatusNotifier tray, whose menu shows skill updates with Update all, lists the presets to switch between, checks for an app update, opens the window, and quits ([spec](../specs/menu-bar.md)).

A menu is only useful while the app runs, so closing the window no longer quits. SkillBinder keeps running behind the icon. Quit lives in the menu and in the usual shortcut. On macOS the Dock icon goes away while no window is open, and opening the window brings it back. A Settings switch, **Show in menu bar** (**Show in system tray** on Windows and Linux), is on by default. Turned off, there is no icon, closing the window quits, and the Dock icon stays, as before this change. Where Linux has no tray to show the icon in, such as stock GNOME without the AppIndicator extension, the app behaves as if the switch were off, so it never runs where the user cannot see it.

Every action in the menu runs the operation the window already runs, under the same rules. Clicking a preset switches it in place without opening the window. While the switch runs, the menu shows its progress. If the window is open, it blocks as ADR 0007 describes. A switch that leaves copies waiting for the user adds a menu item that opens the window on the review list. Update all runs without the window, and an open window shows the same run.

## Considered Options

- Keeping quit-on-close and showing the icon only while the window is open was rejected by the user. The menu would be unreachable exactly when it saves the most steps.
- Staying resident with no way back to quit-on-close was rejected by the user. A Linux desktop with no tray would hide a running app, and some users want closing the window to quit.
- Keeping the Dock icon while the window is closed was rejected by the user. The app would show two icons whose clicks do different things.
- Opening the window for every preset switch from the menu was rejected by the user. A two-second switch would pop the window up every time, and the menu shows the switch's progress itself.
- A one-time notification on the first close, saying the app is still running, was rejected by the user. The icon is visible, the Settings switch explains it, and a notification would add a dependency and, on macOS, a permission prompt.
- Open at login was left out by the user. It is a separate feature with its own work on each platform.

## Consequences

- The quit path is unchanged. The menu's Quit, the shortcut, logging off, and turning the switch off with no window open all drain the engine, and a ready app update installs at that quit (ADR 0017).
- Launching SkillBinder again while it runs without a window opens the window. The instance lock already forwards the second launch.
- Skill update checks can no longer rely on window focus to start the scan they follow, so the app also checks sources hourly while it runs.
- On Windows a new notification-area icon starts in the overflow until the user pins it.
