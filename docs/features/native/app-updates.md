# App updates in place

A newer SkillBinder on the user's channel downloads in the background, waits beside the installed app or, on Windows, in application data, and installs when the app quits ([spec](../../specs/app-updates.md) rules 5–14, 17, 22; ADR 0017, ADR 0018, ADR 0019). Checks, the channel, Settings › Updates, and the download-only notice are in [Settings](settings-agents.md#settings--updates); the architecture is in [Native app updates](../../architecture/native-app-updates.md).

## Per platform

| Platform | What it does now |
|---|---|
| macOS, from `/Applications` or `~/Applications` | Downloads, verifies, stages, and installs in place at quit or Restart. |
| macOS, anywhere else | Download-only: "Move SkillBinder to Applications to update automatically." |
| macOS, Applications folder the user can't write | Download-only: "SkillBinder can't write to its folder." |
| Windows, installed by the setup program | Downloads and verifies the newer setup program, and runs it silently at quit or Restart. |
| Windows, any other copy (no `unins000.exe` beside `skillbinder.exe`) | Download-only: "This copy wasn't installed by the SkillBinder installer." |
| Linux | Download-only: "Download it from the release page." Slice S5 adds in-place updates. |
| Development build (no bundle) | Download-only, "Download it from the release page.", or the Windows line above. An unstamped build never checks. |

## Download and staging (macOS)

A check that finds a newer version for a copy in Applications downloads its disk image at once, with no notice. Settings shows "Downloading 0.2.0… 42%" with Check now disabled. The image streams into `<data>/updates/<version>/archive` while it is hashed; it must be at most 512 MiB and match the manifest's signed size and SHA-256, or it is removed and logged ("App update <v> was discarded: …"), and the next check tries again. A failed download ends the check as failed: an automatic one changes nothing, last checked included, and Check now reads "Couldn't check for updates.". The image is attached read-only with no Finder window, `SkillBinder.app` is copied out with `ditto`, and the copy is kept only when `codesign --verify --deep --strict` accepts it as signed with SkillBinder's Developer ID, the team compiled into the build. The image is detached even when a step fails, and the download folder is removed.

The verified copy becomes `.skillbinder-update-<version>.app` beside `SkillBinder.app`, and the update is **ready**: Settings reads "SkillBinder 0.2.0 is ready." with the primary Restart and the ghost What's new, and the notice appears. It stays ready across launches while that bundle is there and newer than the running version. A newer version found while one is ready downloads quietly, with the ready state still shown, and replaces the staged bundle once it is verified; the notice then names the newer version. A check that finds the staged version again downloads nothing.

Switching to Stable discards a staged beta, removing its bundle, so the app never installs a beta the user just opted out of. A staged release stays ready on either channel.

## The ready notice

"SkillBinder 0.2.0 is ready" in medium weight, "Restart now, or it installs when you quit." muted, then Dismiss and the primary Restart, at the window's bottom-right like the download-only notice, with no auto-hide. It never shows during onboarding and is taken down while the progress strip is up.

- **Restart** quits through the normal quit path, which installs the update and opens the app again. While Sync, Push, Update all, an install from Explore, or a blocking change runs, Restart is disabled, here and in Settings, with the tooltip "Available when Sync finishes", "Available when Push finishes", "Available when Update all finishes", "Available when installing finishes", or "Available when the change finishes".
- **Dismiss** hides the notice until the next launch. Settings still shows the ready update with its own Restart.

## Install at quit

Quitting by any route installs a ready update once the engine has drained its work. `SkillBinder.app` is renamed to `.skillbinder-old-<running version>.app`, then the staged bundle to `SkillBinder.app`; if the second rename fails, the original is renamed back, the running version stays, and the failure is logged, and the next launch offers the update again while its bundle is there. After Restart the app releases its instance lock and runs `open -n` on `SkillBinder.app`, which is the new version, or the old one when the install failed. The next launch removes `.skillbinder-old-*` and any partial staging, and any staged bundle that is not the saved ready version.

## Windows

The setup program (`skillbinder-<version>-x86_64-pc-windows-msvc-setup.exe`, Inno Setup 6, `packaging/windows/skillbinder.iss`) installs for the current user with no administrator prompt into `%LOCALAPPDATA%\Programs\SkillBinder`, adds a SkillBinder entry to the Start menu, and registers an uninstaller under Settings › Apps. Uninstalling removes only what setup installed, so `%LOCALAPPDATA%\SkillBinder`, the user's data and library, stays. An app update is the same program: only a copy with Inno's `unins000.exe` beside its executable updates in place, so a development build, or a copy unpacked by hand, never runs a setup program that would install over another copy.

A check that finds a newer version downloads its setup program into `<data>/updates/<version>/archive` with the same size and SHA-256 checks as macOS; the signed manifest is the only proof of the publisher, since the program is not code-signed. The verified program becomes `<data>/updates/.skillbinder-update-<version>.exe`, and the update is ready, with the same notice, Settings row, Restart, Dismiss, and channel rules as on macOS. At launch, the download folders are removed and the staged program is kept while it is the saved ready version and newer than the running one.

At quit, once the engine drained, the app starts the staged program detached: `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /WAITPID=<pid>`, plus `/RELAUNCH=1` after Restart, and exits. Setup waits up to 60 seconds for that process to exit, installs over the old version, keeping the Start menu entry and the uninstaller, and opens the new version only after Restart. While the app still runs after the wait, setup ends without changing anything, and the next launch offers the update again; the next launch also removes the staged program once its version runs. If the program does not start, the failure is logged, and Restart opens the running version again. A quit caused by logging off or shutting down (`SM_SHUTTINGDOWN`) starts nothing, since Windows could end setup halfway; the update stays ready and installs at the next quit.

## After an update

The first launch of a version newer than the last one that ran shows the change toast "Updated to SkillBinder 0.2.0" with Release notes, which opens the release page the manifest named, or the GitHub release page of that version. It shows once, after onboarding. A first launch with no earlier version recorded shows none.

## Verification

Domain tests run on every platform with a fixture bundle whose archive is its executable and nothing signed (`UpdatePayload::Executable`): a wrong SHA-256 is discarded (AC3.1), quit swaps the bundle, a failed second rename restores it, and the next launch cleans up (AC3.2), a newer manifest replaces the staged bundle and a switch to Stable discards a staged beta (AC3.3). On macOS a test also stages a real disk image and sees `codesign` refuse its unsigned bundle and the image detach. The packaged checks AC3.5 to AC3.7 need two notarized builds and are recorded in the [native gate](../../native-gate.md) when run.

Windows domain tests run on every platform too, with a fixture `skillbinder.exe` and setup programs that are only text (`UpdatePayload::Setup`): a copy without `unins000.exe` is download-only (NotInstalled), a staged setup program waits in application data across launches, the silent command line always waits for the app and relaunches only after Restart, a quit as the session ends starts nothing, and a setup program that does not start reopens the app after Restart (AC4.2). The packaged checks AC4.1 and AC4.3 need a Windows machine and two setup programs, and are recorded in the native gate when run.
