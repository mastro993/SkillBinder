# App updates: acceptance and slices

Companion to [the spec](../app-updates.md). The numbers in parentheses are its Behavior rules. Each check is tagged by how it is proven, as AGENTS.md requires: **[domain]** engine and client tests with fixture manifests and a test key, **[native]** an interactive check through `native-test`, **[packaged]** a smoke test on a real signed build.

## Slices

In implementation order. Each ships alone.

### S0 Publish signed manifests

The Release and Nightly workflows publish `update.json` and its `.minisig`, and the Catalog job mirrors them to `updates/v1/`. No app change. Depends on nothing. Updates `docs/release.md` (the new secrets, the dispatch, the manifest).

- **AC0.1 [domain]** `xtask update-manifest` writes the documented shape for the three archives in a fixture `dist/`, with their SHA-256s and sizes, and a signature that `minisign-verify` accepts under the matching public key.
- **AC0.2 [packaged]** After a nightly run, `updates/v1/beta.json` on Pages names that beta and its signature verifies. After a release, `stable.json` names it, and so does `beta.json` when no newer beta exists.
- **AC0.3** A Catalog run whose copy fails keeps the previous `updates/` on the site.

### S1 Version, channel, and checks with Download

Settings › Updates shows Version, Channel, and Status. Checks run at launch, hourly, on Check now, and after a channel change. A newer version shows the download-only notice, whose Download opens the release page on every platform. This is useful alone: everyone learns of releases. Depends on S0 for real use; fixtures cover the tests.

- **AC1.1 [domain]** `AppVersion` parses `0.1.0` and `0.1.1-beta.12`, rejects anything else, and orders `0.1.0 < 0.1.1-beta.3 < 0.1.1-beta.12 < 0.1.1` (2).
- **AC1.2 [domain]** A check offers a version only when the signature verifies and the version is newer than the running one. An older or equal manifest, a bad signature, or a missing asset for the target reads as up to date. A bad signature is logged (2, 4).
- **AC1.3 [domain]** A second ETag check that gets 304 makes no change. Offline maps to `CheckFailure::Offline` for a manual check, and a failed automatic check leaves `Idle` with no failure (4).
- **AC1.4 [domain]** An unstamped build's state is `Off`, and it never requests anything (1).
- **AC1.5 [domain]** The channel defaults to the build's channel, persists, and switching to Beta triggers a check (15, 16).
- **AC1.6 [native]** Settings shows the Updates section with every status row of the table in 18 for the matching fixture state, and the Beta→Stable line on a beta build set to Stable (17, 19).
- **AC1.7 [native]** With a newer fixture manifest, the notice "SkillBinder 0.2.0 is available" appears bottom-right, stacks with the new agent notice, hides during the progress strip, and Dismiss hides it until relaunch (9, 13).

### S2 macOS install flow

The disk image opens on the drag-to-Applications window, and the app asks to move itself into Applications. It needs no update code and helps every new user. Depends on nothing; it follows S1 so the move also unlocks updates. Updates `docs/release.md` and the setup feature docs.

- **AC2.1 [packaged]** Opening the disk image shows SkillBinder, the arrow background, and the Applications shortcut, laid out by `dmgbuild` on CI (20).
- **AC2.2 [domain]** `location` reports Translocated, DiskImage, NotWritable, and inside Applications for fixture paths (21).
- **AC2.3 [domain]** `relocate` copies a fixture bundle into a fixture Applications folder. It falls back to the user folder when the first can't be written, replaces an older copy, and refuses to replace a newer one (21).
- **AC2.4 [native]** Outside Applications the dialog shows before onboarding with its copy. Not now continues and the dialog returns at relaunch (21).
- **AC2.5 [packaged]** Launched from the mounted image: Move copies to `/Applications`, ejects the image, and relaunches with no Gatekeeper prompt. Launched from Downloads: the original goes to the Trash (21).

### S3 macOS in place

Background download, verification, staging, the ready notice, Restart, install at quit, and the "Updated to" toast, on macOS. Depends on S1 and S2.

- **AC3.1 [domain]** A fixture download whose SHA-256 differs is discarded and not staged (6).
- **AC3.2 [domain]** `install_staged` swaps a fixture bundle by rename, restores the original when the second rename fails, and the next launch removes `.skillbinder-old-*` (12).
- **AC3.3 [domain]** A newer manifest while one is ready replaces the staged payload (8). Switching to Stable discards a ready beta newer than the latest stable (17).
- **AC3.4 [native]** The ready notice shows the version. Restart is disabled with its tooltip while Sync runs. Dismiss hides it until relaunch (9, 10, 11).
- **AC3.5 [packaged]** From an older notarized build in `/Applications`, the newer build downloads, the notice appears, and Restart relaunches into it with no Gatekeeper prompt. "Updated to SkillBinder <v>" shows once (10, 14).
- **AC3.6 [packaged]** The same with Dismiss, then Quit: the next launch is the new version (12). Run on a non-admin account in `~/Applications` too, to confirm macOS 26's App Management permission does not block it.
- **AC3.7 [packaged]** A bundle re-signed with another team is refused before staging (6). After Not now, the download-only notice says to move SkillBinder to Applications (13).

### S4 Windows installer and updates

The per-user Inno Setup installer replaces the ZIP. Updates run it silently at quit. Depends on S1.

- **AC4.1 [packaged]** Setup installs into `%LOCALAPPDATA%\Programs\SkillBinder` with no UAC prompt, adds the Start menu entry and the uninstaller, and uninstalling keeps `%LOCALAPPDATA%\SkillBinder` data (22).
- **AC4.2 [domain]** `location` reports NotInstalled without the `HKCU` uninstall entry, and `install_staged` builds the silent command line with `/RELAUNCH` only after Restart (12, 13).
- **AC4.3 [packaged]** An older installed build updates through Restart and through Dismiss then Quit. The installer waits for the app to exit, and the Start menu entry opens the new version (10, 12).

### S5 Linux AppImage and updates

The AppImage replaces the tarball, with the menu offer and in-place updates. Depends on S3, with which it shares staging and the rename swap.

- **AC5.1 [packaged]** The AppImage runs on Ubuntu 22.04 and 24.04 with no `libfuse2` installed (23).
- **AC5.2 [domain]** `desktop_entry` writes the entry and icon under a fixture `~/.local/share`, pointing at the AppImage path. The answer is remembered and never asked again (23).
- **AC5.3 [domain]** `location` reports NotInstalled without `$APPIMAGE`, and NotWritable for a read-only folder (13).
- **AC5.4 [packaged]** An older AppImage under `~/Applications` updates through Restart, and the menu entry opens the new version (10, 23).

## Docs each slice updates

- S0: `docs/release.md`, and `xtask/src/graph.rs` with the third-party notices for `minisign`.
- S1: `docs/features/native/settings-agents.md` (Settings › Updates), `docs/architecture/native.md` (the ownership row, HTTP destinations), and `xtask/src/graph.rs` with the third-party notices for `minisign-verify`.
- S2, S4, S5: `docs/release.md` (archives, build tools, SmartScreen, FUSE) and `docs/features/native/setup-and-shell.md` (the move and menu dialogs).
- S3–S5: `docs/features/native/` gains `app-updates.md` describing what is implemented per platform, and `docs/native-gate.md` records the packaged evidence.
