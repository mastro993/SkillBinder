# App updates

ADRs: [0017](../adr/0017-app-updates-download-first-apply-at-restart.md) (behavior), [0018](../adr/0018-signed-update-manifests-on-pages.md) (manifests, verification, install), [0019](../adr/0019-classic-install-per-platform.md) (installers per platform); partly supersedes [0014](../adr/0014-explore-catalog-over-https.md) (the HTTP client's destinations and the Pages site). Glossary: [domain-app.md](../agents/domain-app.md) (app update, channel). Releases: [docs/release.md](../release.md).

This spec has two companion files: [Design](app-updates/design.md) (architecture, data, interfaces, risks) and [Acceptance and slices](app-updates/acceptance.md).

## Problem

SkillBinder publishes a notarized macOS disk image, a Windows ZIP, and a Linux tarball for every release and for every nightly beta (`docs/release.md`). A running copy never learns of them. Users stay on whatever build they downloaded until they think to visit the releases page, and beta testers must reinstall every night by hand to test anything.

## Outcome

Within about an hour of a release on the user's channel, the running app has downloaded and verified it and shows "SkillBinder 0.2.0 is ready". One click restarts into it. Ignoring the notice still lands the update at the next quit. Settings shows the version and the channel, and has a Check now button. A user who cannot update in place, such as one running the app from its disk image, is told so and gets a Download link.

## Scope

In:
- checks at launch, hourly, on demand, and on a channel change;
- Stable and Beta channels;
- background download, verification, and staging;
- the ready notice with Restart and Dismiss, and install at quit;
- in-place replacement on macOS, Windows, and Linux, with a download-only fallback;
- the Settings › Updates section and the "Updated to" toast;
- publishing signed manifests from the release workflows;
- classic installation per platform: the macOS disk image window with Move to Applications, a per-user Windows installer, and a Linux AppImage with its applications-menu entry.

Out:
- delta updates;
- installs through package managers (Homebrew, winget, Flatpak, `.deb`, `.rpm`);
- downgrades;
- showing release notes inside the app;
- Linux on other architectures;
- code-signing the Windows installer;
- per-machine or elevated installs.

## Decisions

These are the owner's decisions from the 2026-11-20 design session. Ids are stable.

- **D1** Download first. An app update is downloaded, verified, and staged before the user hears of it. The notice appears only when it is ready, with Restart and Dismiss. Where the app cannot replace itself, the notice offers Download, which opens the release page.
- **D2** In-place replacement on all three platforms, each falling back to Download where its folder cannot be written.
- **D3** Dismiss hides the notice until the next launch. A ready update installs at the next quit.
- **D4** Switching from Beta to Stable keeps the installed beta until Stable passes it. There is no downgrade.
- **D5** Checks run at launch, while the app is open, and on demand (the owner's request).
- **D6** The user chooses Stable or Beta in Settings, and Settings shows the version and Check now (the owner's request).
- **D7** Each platform is packaged with its classic installation flow, so the app always sits where it can update itself (the owner's request).
- **D8** macOS: the disk image opens on the drag-to-Applications window. Launched from anywhere outside Applications, the app asks to move itself there: Move, or Not now, which asks again at the next launch.
- **D9** Windows: a per-user installer with no administrator prompt, a Start menu entry, and an uninstaller. The ZIP is dropped.
- **D10** Linux: an AppImage replaces the tarball, and the first launch offers to add SkillBinder to the applications menu.
- **D11** The user can turn automatic checks off in Settings and still update by hand with Check now (overrules the first draft's A3).

## Behavior

### Checks

1. A stamped release build checks when the user presses Check now. While **Check automatically** is on, which is the default, it also checks at launch, every hour while it runs, and right after the channel changes. Turned off, the app never checks by itself (D11). Turning it back on checks at once. A build without the release stamp never checks, and Settings says "Updates are off in this build".
2. A check reads the channel's signed manifest. It offers the manifest's version only when it is newer than the running version. Versions compare as `X.Y.Z` with an optional `-beta.N`, and `0.1.1-beta.12 < 0.1.1`.
3. Only one check or download runs at a time. Check now while one runs does nothing more, and its button shows the running state.
4. A failed automatic check stays silent and the next one retries. A failed Check now says why in Settings: "Couldn't check for updates. Check your connection." when offline, and "Couldn't check for updates." for any other failure, including a bad signature, which is also logged.

### Download and staging

5. A newer version downloads at once, in the background, with no notice. Settings shows "Downloading 0.2.0… 42%".
6. The download must match the signed SHA-256. On macOS the app inside must also carry SkillBinder's Developer ID signature. A payload that fails either check is discarded and logged, and the next check tries again.
7. A verified payload is staged: next to the installed app on macOS and Linux, and in application data for the Windows installer (ADR 0018). From then on the update is **ready**, and it stays ready across launches until it is installed or a newer one replaces it.
8. A newer version that appears while one is ready downloads in turn and replaces the staged one. The notice then names the newer version.

### Ready notice

9. When an update becomes ready, and at every launch while one is ready, a persistent notice appears at the window's bottom-right, like the new agent notice: "SkillBinder 0.2.0 is ready" in medium weight, "Restart now, or it installs when you quit." muted, then Dismiss and the primary Restart. It has no auto-hide, takes no focus, and stacks with the new agent notice. Like that notice, it hides while the blocking progress strip is up and returns when the strip ends.
10. **Restart** quits through the normal quit path, installs the update, and launches the new version. It is disabled while Sync, Push, Update all, an install, or a blocking change runs, with the tooltip "Available when <Sync> finishes".
11. **Dismiss** hides the notice until the next launch (D3). Settings still shows the ready update with its own Restart.
12. **Quit**, by any route, installs a ready update first (D3). On Windows, a quit caused by logging off or shutting down skips the installer, which Windows could kill halfway; the next quit installs it. If installing fails, the old version stays, the failure is logged, and the next launch shows the notice again.

### Download-only

13. Where the app cannot replace itself (ADR 0018), nothing is downloaded. The notice reads "SkillBinder 0.2.0 is available" with Dismiss and the primary Download, which opens the release page in the browser. The muted line says why:
    - "Move SkillBinder to Applications to update automatically." on macOS anywhere outside `/Applications` and `~/Applications`, including the disk image, Downloads, and the Desktop, after Not now;
    - "SkillBinder can't write to its folder." for an AppImage in a folder the user cannot write, or a macOS Applications folder the user cannot write;
    - "This copy wasn't installed by the SkillBinder installer." for a Windows build not installed by setup.

    Dismiss hides it until the next launch.

### After an update

14. The first launch of a newer version shows the change toast "Updated to SkillBinder 0.2.0" with the action Release notes, which opens the release page. A first launch with no earlier version recorded shows no toast.

### Channels

15. The channel starts as the installed build's: Beta for a `-beta.N` build, Stable otherwise. The user's choice is saved per device and kept across updates.
16. Switching channel, to Beta or to Stable, checks at once while Check automatically is on. A stable build whose channel was Beta can then find a newer stable release. Otherwise the next Check now uses the new channel.
17. Switching to Stable on a beta build keeps the beta (D4). Settings says "You'll move to Stable at its first release after 0.1.1-beta.12." until a stable release newer than the installed beta is installed. A ready update that is newer than the latest stable release is discarded on the switch.

### Settings › Updates

18. Settings gains an "Updates" section header after "Application", over a `KeyValueList`: **Version** (the full version, such as `0.1.1-beta.12`, in tabular figures), **Channel** (a segmented control, Stable | Beta), **Check automatically** (a Kit switch, as the Agents rows use, saved per device), and **Status** (one line, then its action):

| State | Status line | Action |
|---|---|---|
| Off | Updates are off in this build. | none |
| Up to date | Up to date · checked 5 minutes ago | Check now |
| Checking | Checking… (spinner) | Check now, disabled |
| Downloading | Downloading 0.2.0… 42% | Check now, disabled |
| Ready | SkillBinder 0.2.0 is ready. | Restart, primary; What's new link |
| Download-only | SkillBinder 0.2.0 is available. Plus the reason from 13 | Download; What's new link |
| Check failed, offline (manual) | Couldn't check for updates. Check your connection. | Check now |
| Check failed, other (manual) | Couldn't check for updates. | Check now |

19. The Beta→Stable line from 17 sits under the Channel control while it applies. With Check automatically off, the Up to date line reads "Up to date · checked 3 days ago" or "Not checked yet", so the user can see how stale it is.

### Installing SkillBinder

20. **macOS.** Opening the disk image shows SkillBinder, an arrow, and an Applications shortcut.
21. **macOS.** Launched from outside `/Applications` and `~/Applications`, before anything else (onboarding included), a dialog asks "Move SkillBinder to Applications?" with the body "SkillBinder updates itself only from your Applications folder.", Not now, and the primary Move.
    - **Move** copies the app into `/Applications`, or into `~/Applications` when `/Applications` can't be written. It moves the original to the Trash, or ejects the disk image when it ran from one, and relaunches from the new place.
    - If a SkillBinder is already there, Move replaces it, unless that copy is newer, in which case Move opens it instead.
    - A failed move shows "Couldn't move SkillBinder. Drag it to Applications yourself." and continues from where it is.
    - **Not now** continues, and the dialog asks again at the next launch.
22. **Windows.** The setup program installs for the current user only, with no administrator prompt. It installs into `%LOCALAPPDATA%\Programs\SkillBinder`, adds a Start menu entry, and registers an uninstaller under Settings › Apps. SmartScreen's "Windows protected your PC" still appears once (More info → Run anyway). Uninstalling keeps the user's data and library.
23. **Linux.** The release ships `skillbinder-<version>-x86_64.AppImage`. On its first launch as an AppImage, a dialog asks once "Add SkillBinder to your applications menu?" with Not now and the primary Add, and either answer is remembered. Add writes the menu entry and icon under `~/.local/share`, pointing at the AppImage's path. An update replaces the file at that path, so the entry keeps working.

### Updating by hand

24. **Updating by hand.** With Check automatically off, Check now finds, downloads, and stages an update just as an automatic check would. The update is then ready: the notice appears, and Restart or quit installs it (rules 9–12). Automatic checks decide only when the app looks; once the user has asked, the update proceeds.

## UX

The notice reuses the new agent notice's pattern and Kit notification (`docs/features/native/settings-agents.md`). The Settings section reuses `SectionHeader`, `KeyValueList`, Kit's segmented control as Library's segments use it, `Button`, and `Spinner`. No new design-system component is needed. Copy follows the glossary: it always says SkillBinder, never a bare "update", which means a skill update.

## Assumptions

Decided without asking. Each says what changes if it is wrong.

- **A1** Hourly checks, with the first right after launch (Zed, VS Code). A shorter interval needs only a constant.
- **A2** The default channel follows the installed build (15). If everyone should start on Stable, a beta tester's first launch would wait for the next release.
- **A3** Superseded by D11.
- **A13** With automatic checks off, an update found by Check now still downloads at once and installs at the next quit (24). If the owner wants nothing to install without pressing Restart in this mode, quit-install gets a condition on the switch.
- **A4** Restart is disabled while long work runs (10). Allowing it would cancel the work through the quit drain.
- **A5** The "Updated to" toast (14) and the What's new links are the only release notes. In-app notes are out of scope.
- **A6** The Settings layout follows the existing sections (18) without a prototype. A different look goes through ux-brainstorm before slice S1.
- **A7** The download-only copy in 13. It is the only guidance a user who cannot update in place gets.
- **A8** A ready update not on the new channel is discarded on a switch to Stable (17), so switching channels never installs a beta the user just opted out of.
- **A9** Move puts the original in the Trash and falls back to `~/Applications` (21), as LetsMove-style prompts do. Keeping the original would leave a second copy that never updates.
- **A10** The Linux menu offer is asked once, with no Settings row to ask again (23). A Settings row is a small addition if users ask for it.
- **A11** The Windows uninstaller keeps application data and the library (22). The library is the user's, and it may sync with other devices.
- **A12** The copy of the move and menu dialogs (21, 23).
