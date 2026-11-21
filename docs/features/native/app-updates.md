# App updates in place

A newer SkillBinder on the user's channel downloads in the background, waits beside the installed app, and installs when the app quits ([spec](../../specs/app-updates.md) rules 5–14, 17; ADR 0017, ADR 0018). Checks, the channel, Settings › Updates, and the download-only notice are in [Settings](settings-agents.md#settings--updates); the architecture is in [Native app updates](../../architecture/native-app-updates.md).

## Per platform

| Platform | What it does now |
|---|---|
| macOS, from `/Applications` or `~/Applications` | Downloads, verifies, stages, and installs in place at quit or Restart. |
| macOS, anywhere else | Download-only: "Move SkillBinder to Applications to update automatically." |
| macOS, Applications folder the user can't write | Download-only: "SkillBinder can't write to its folder." |
| Windows and Linux | Download-only: "Download it from the release page." Slices S4 and S5 add in-place updates. |
| Development build (no bundle) | Download-only, like Windows and Linux. An unstamped build never checks. |

## Download and staging (macOS)

A check that finds a newer version for a copy in Applications downloads its disk image at once, with no notice. Settings shows "Downloading 0.2.0… 42%" with Check now disabled. The image streams into `<data>/updates/<version>/archive` while it is hashed; it must be at most 512 MiB and match the manifest's signed size and SHA-256, or it is removed and logged ("App update <v> was discarded: …"), and the next check tries again. The image is attached read-only with no Finder window, `SkillBinder.app` is copied out with `ditto`, and the copy is kept only when `codesign --verify --deep --strict` accepts it as signed with SkillBinder's Developer ID, the team compiled into the build. The image is detached even when a step fails, and the download folder is removed.

The verified copy becomes `.skillbinder-update-<version>.app` beside `SkillBinder.app`, and the update is **ready**: Settings reads "SkillBinder 0.2.0 is ready." with the primary Restart and the ghost What's new, and the notice appears. It stays ready across launches while that bundle is there and newer than the running version. A newer version found while one is ready downloads quietly, with the ready state still shown, and replaces the staged bundle once it is verified; the notice then names the newer version. A check that finds the staged version again downloads nothing.

Switching to Stable discards a staged beta, removing its bundle, so the app never installs a beta the user just opted out of. A staged release stays ready on either channel.

## The ready notice

"SkillBinder 0.2.0 is ready" in medium weight, "Restart now, or it installs when you quit." muted, then Dismiss and the primary Restart, at the window's bottom-right like the download-only notice, with no auto-hide. It never shows during onboarding and is taken down while the progress strip is up.

- **Restart** quits through the normal quit path, which installs the update and opens the app again. While Sync, Push, Update all, an install from Explore, or a blocking change runs, Restart is disabled, here and in Settings, with the tooltip "Available when Sync finishes", "Available when Push finishes", "Available when Update all finishes", "Available when installing finishes", or "Available when the change finishes".
- **Dismiss** hides the notice until the next launch. Settings still shows the ready update with its own Restart.

## Install at quit

Quitting by any route installs a ready update once the engine has drained its work. `SkillBinder.app` is renamed to `.skillbinder-old-<running version>.app`, then the staged bundle to `SkillBinder.app`; if the second rename fails, the original is renamed back, the running version stays, and the failure is logged, and the next launch offers the update again while its bundle is there. After Restart the app releases its instance lock and runs `open -n` on `SkillBinder.app`, which is the new version, or the old one when the install failed. The next launch removes `.skillbinder-old-*` and any partial staging, and any staged bundle that is not the saved ready version.

## After an update

The first launch of a version newer than the last one that ran shows the change toast "Updated to SkillBinder 0.2.0" with Release notes, which opens the release page the manifest named, or the GitHub release page of that version. It shows once, after onboarding. A first launch with no earlier version recorded shows none.

## Verification

Domain tests run on every platform with a fixture bundle whose archive is its executable and nothing signed (`UpdatePayload::Executable`): a wrong SHA-256 is discarded (AC3.1), quit swaps the bundle, a failed second rename restores it, and the next launch cleans up (AC3.2), a newer manifest replaces the staged bundle and a switch to Stable discards a staged beta (AC3.3). On macOS a test also stages a real disk image and sees `codesign` refuse its unsigned bundle and the image detach. The packaged checks AC3.5 to AC3.7 need two notarized builds and are recorded in the [native gate](../../native-gate.md) when run.
