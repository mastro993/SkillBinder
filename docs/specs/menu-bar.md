# Menu bar menu

ADRs: [0020](../adr/0020-stay-running-in-the-menu-bar.md) (behavior: staying resident), [0021](../adr/0021-tray-through-tray-icon-and-ksni.md) (model, backends, running work). It builds on [0007](../adr/0007-pages-list-active-skills.md) (blocking changes), [0011](../adr/0011-git-is-optional.md) (Git), and [0017](../adr/0017-app-updates-download-first-apply-at-restart.md) (app updates). Glossary: [domain.md](../agents/domain.md) (preset, update) and [domain-app.md](../agents/domain-app.md) (menu bar menu, app update).

Two companion files go with this spec: [Design](menu-bar/design.md) (architecture, interfaces, risks) and [Acceptance and slices](menu-bar/acceptance.md). The approved prototype is [docs/design/menu-bar](../design/menu-bar/prototype.html), layout A.

## Problem

To see whether skills have an update, run Update all, switch the active preset, or check for an app update, the user opens the window and goes to Library, the sidebar, or Settings. The app also quits when its window closes, so none of this is reachable without opening the app again. Users who switch presets several times a day pay that every time.

## Outcome

From the menu bar on macOS, or the system tray on Windows and Linux, the user can do any of these in two clicks or fewer, with or without the window open: see that skills have an update and update them all, switch the active preset, check for an app update or restart into one, open SkillBinder, and quit it. The icon shows a dot when something waits for them.

## Scope

In:
- the icon and its menu on macOS, Windows, and Linux with a StatusNotifier host;
- staying running when the window closes, and the Settings switch that turns that off;
- hiding the Dock icon while no window is open (macOS);
- hourly skill update checks while the app runs;
- moving Update all, Sync, and the blocking change out of the window's view so the menu can see them.

Out:
- turning single skills on or off, workspaces, Sync, Explore, and import from the menu;
- open at login;
- system notifications;
- preset colors and icons in the menu, which native menus cannot show beside a check;
- a tray on Linux desktops without a StatusNotifier host.

## Decisions

These are the owner's decisions from the 2026-11-21 design session. Ids are stable.

- **D1** Closing the window keeps SkillBinder running behind its icon. Settings › Application › **Show in menu bar** (macOS) or **Show in system tray** (Windows, Linux), on by default, turns the icon off. Off brings back quit-on-close.
- **D2** macOS: while no window is open, the app has no Dock icon and is not in ⌘-Tab. Opening the window brings both back.
- **D3** Clicking a preset switches it in place without opening the window. The menu shows the switch's progress. Copies left waiting add an item that opens the window's review list.
- **D4** No open at login.
- **D5** Layout A: presets listed flat under a Presets header, the active one checked.
- **D6** The icon shows a dot when skills have an update, an app update is ready or download-only, or copies wait for the user.
- **D7** No hint the first time the window closes.
- **D8** Windows: a left click on the icon opens the window and a right click shows the menu. On macOS and Linux every click shows the menu.

## Behavior

### Staying resident

1. With the switch on and a tray available, closing the window leaves the app running. With the switch off, or on Linux with no StatusNotifier host, closing the window quits, as before (D1).
2. Quitting from the menu, with ⌘Q or Ctrl+Q, or at logoff takes today's quit path: the engine drains, and a ready app update installs.
3. Open SkillBinder, the Dock icon on macOS, a Windows left click (D8), and launching SkillBinder again each open the window if none is open, or bring the open one forward. A newly opened window starts on Library.
4. macOS: when the last window closes and the app keeps running, it switches to the Accessory policy (no Dock icon). Opening a window switches it back to Regular and activates it (D2).
5. Turning the switch off while the window is open removes the icon at once. Turning it on adds it.
6. Closing the window during a blocking change, an Update all, a Sync, or an install does not stop it. The work finishes behind the icon. A blocking change whose window closed ends with no Done pill, and copies left waiting show only as rule 7.1's item. Once it lands, the menu's presets are enabled again. A reopened window refuses what the menu refuses, Update all included.

### Menu

7. Items, top to bottom, each group shown only when it applies:
   1. `⚠ <n> skill copies need attention…` (`1 skill copy needs attention…`), then a separator, while copies wait for the user, counted as the progress strip counts them. It opens the window on the review list.
   2. The skill update group, then a separator (rules 9–10).
   3. `Presets` as a disabled header, or, while a blocking change runs, the strip's label with its percentage, such as `Switching to Review… 40%`. Then every preset by name in sidebar order, the active one checked. Then a separator.
   4. The app update group (rule 13).
   5. `Open SkillBinder`, a separator, and `Quit SkillBinder` with the platform's quit shortcut shown.
8. Before onboarding finishes, the menu holds only Open SkillBinder and Quit SkillBinder.

### Skill updates

9. The status line is disabled text. The first of these that applies is shown:
   - Git missing: `Skill updates need Git`.
   - Update all running: `Updating <done> of <total> skills…`.
   - The last Update all had failures: `Updated <n> · <m> failed`, enabled. It opens the window on Library with the Updates filter.
   - Skills have updates: `<n> skills have updates` (`1 skill has an update`), followed by an enabled `Update all`.
   - A check running with no update known: `Checking skill updates…`.
   - Otherwise: `Skills are up to date`.
10. Update all from the menu is Library's Update all: it updates each skill with an update, one at a time, then scans once. It is unavailable while a blocking change, Sync, or Push runs, as in the window. An open window shows the same run in Library's header.
11. While the app runs, it checks every source not checked within the hour, every hour, whether or not the window is open (ADR 0020).

### Presets

12. Clicking a preset that is not active switches it (D3):
    - With the window open, the switch runs exactly as clicking the preset in the sidebar does: the strip, the toast, and Done · Review.
    - With no window, the switch runs behind the icon, and the presets are disabled until it lands.
    - If it ends with copies waiting, rule 7.1's item appears.
    - Clicking the active preset does nothing.
    - While any blocking change runs, every preset is disabled.

### App updates

13. The app update group follows the app update state:
    - Off: `App updates are off in this build`, disabled.
    - Idle, after a check that found nothing: `SkillBinder <version> is up to date`, disabled, above `Check for app updates`.
    - Idle, never checked: `Check for app updates`.
    - Idle, after a failed check: `Couldn't check for app updates · Try again`.
    - Checking: `Checking for app updates…`, disabled.
    - Downloading: `Downloading SkillBinder <version>… <percent>%`, disabled.
    - Ready: `Restart to install SkillBinder <version>`. It is disabled while one of Restart's blockers runs, as in Settings.
    - Download-only: `Download SkillBinder <version>…`, which opens the release page.

    Check for app updates is Settings' Check now.

### Icon

14. The icon is SkillBinder's mark as a monochrome glyph: a template image on macOS, and light and dark variants that follow the taskbar or panel theme on Windows and Linux. A dot variant shows while rule D6 applies. Its tooltip is `SkillBinder`.

## UX

The approved prototype is [docs/design/menu-bar/prototype.html](../design/menu-bar/prototype.html), layout A, dot badge. Its state buttons show each rule above.

## Assumptions

Decisions made for the owner, open to overrule:

- **A1** "Check for updates" means the app update check. A bare "update" stays a skill update (glossary).
- **A2** The Windows and Linux switch label says "system tray". macOS says "menu bar".
- **A3** A failed Update all stays in the menu as `Updated n · m failed` until the next Update all or source check completes. The window already shows it in Library.
- **A4** The `is up to date` line appears only after a check that found nothing, so a menu never claims a check it did not make.
- **A5** The attention item shows whenever copies wait, from any change, because it counts what the strip counts.
- **A6** A window opened again starts on Library, as at launch. It does not restore the page it closed on.
- **A7** Menu actions keep the window's rules exactly: what the window refuses during a blocking change, the menu disables.
- **A8** The Show in menu bar preference is machine-local, in `Preferences`, and it never syncs.
