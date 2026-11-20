# App updates are found through signed channel manifests on the catalog's Pages site

Partly supersedes ADR 0014: the engine's HTTP client also fetches the update manifests and downloads release archives from GitHub Releases, and the Pages site also serves `updates/`.

App updates (ADR 0017) need a way to find the newest build on each channel, to know that the downloaded archive is SkillBinder's, and to replace a running app on three platforms.

**Publishing.** The Release and Nightly workflows run `xtask update-manifest` after they build. It writes `update.json`, which holds the version, the release page URL, and for each target the archive's download URL, SHA-256, and size. It signs the file with minisign, using the `UPDATE_SIGNING_KEY` secret, into `update.json.minisig`. Both are uploaded as assets of the GitHub release, and the workflow then dispatches the Catalog workflow. The Catalog job, which already holds the only GitHub token and is the only workflow that deploys Pages, copies the latest release's pair to `updates/v1/stable.json` and `stable.json.minisig`. It copies the newer of that release and the newest prerelease to `updates/v1/beta.json` and its signature. It copies their bytes unchanged, so the signatures still verify. `actions/deploy-pages` replaces the whole site, so every Catalog run, the nightly cron and pushes to `catalog/` included, rebuilds `updates/` this way; a run that skipped it would delete the manifests. A failed copy keeps the previous site, as a failed catalog deploy does.

**Checking.** The engine fetches its channel's manifest and signature from the catalog's base URL with the HTTP client it already has (`ureq`, rustls), a conditional `If-None-Match` request with a 10-second timeout and a 64 KB cap. It verifies the signature with `minisign-verify` against a public key compiled into the app. It offers the manifest's version only when it is newer than the running one, so a replayed older manifest can do no harm. Versions compare as `X.Y.Z` with an optional `-beta.N`, and a beta sorts before its release.

**Downloading and staging.** Each platform's update is the same file a new user installs (ADR 0019): the disk image, the Windows setup program, or the AppImage. It comes from its GitHub release URL, capped at 512 MB, into `<data>/updates/<version>/`, and must match the signed SHA-256.

- **macOS:** the disk image is opened with `hdiutil attach -nobrowse -readonly` and `SkillBinder.app` is copied out with `ditto`. It must pass `codesign --verify --deep --strict` against a requirement naming SkillBinder's Developer ID team, which the build compiles in. It is then copied next to the installed bundle as `.skillbinder-update-<version>.app`, so the swap is a rename on one volume. That copy also tests whether the folder is writable.
- **Linux:** the AppImage is copied next to the running one, `$APPIMAGE`, as `.skillbinder-update-<version>`, and marked executable.
- **Windows:** the setup program stays in `<data>/updates/<version>/`. The per-user install folder is always writable.

A payload that fails any check is deleted and logged, and the next check tries again.

**Installing.** At quit, after the engine has drained, `apps/desktop` calls the engine's `app_update::install_staged`.

- **macOS and Linux:** it renames the current bundle or AppImage to `.skillbinder-old-*` and the staged one into its place, and renames back if the second rename fails. Both platforms allow renaming a running executable.
- **Windows:** it starts the setup program detached with `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /WAITPID=<pid>`, adding `/RELAUNCH=1` after Restart. Neither switch is built into Inno Setup. The script reads both with `{param:…}`. Its `[Code]` `InitializeSetup` opens that process through `kernel32` (`OpenProcess`, `WaitForSingleObject`) and waits up to 60 seconds for it to exit, giving up with no change if it doesn't. A `[Run]` entry starts SkillBinder when `RELAUNCH` is set. `AppMutex` is not used: under `/SUPPRESSMSGBOXES` it would not wait. A quit caused by logging off or shutting down does not start the installer, because Windows may kill it halfway; the update waits for the next quit. Windows needs no rename trick and no leftovers.

After Restart on macOS and Linux, `apps/desktop` launches the new executable once the instance lock is released, and exits. The next launch deletes leftover `.skillbinder-old-*` items.

**Where the app cannot update itself.** On macOS that is anywhere outside `/Applications` and `~/Applications`: Downloads, the Desktop, a translocated path, or the mounted disk image. Also an Applications folder the user cannot write. The Move to Applications prompt (ADR 0019) fixes the first case. On Linux it is an AppImage in a folder the user cannot write, or a build not run as an AppImage. On Windows it is a build not installed by the setup program, recognized by the missing `unins000.exe` beside the executable, such as a development build or an unpacked old ZIP. In those places the app reports the update as download-only. A build without the release stamp, `SKILLBINDER_RELEASE` set by `build.yml`, never checks.

## Considered Options

- Listing releases through the GitHub REST API was rejected, as ADR 0005 and ADR 0014 rejected it: 60 unauthenticated calls an hour per IP address, shared behind NAT, and no alias for the newest prerelease.
- `releases/latest/download/update.json` alone was rejected. It costs no API quota but serves only the stable release, and Beta needs a second address anyway.
- A rolling release holding `beta.json` was rejected. Its asset would be replaced every night, which GitHub's immutable releases forbid.
- Trusting `SHA256SUMS` from the same release was rejected. Anyone who can upload one asset can rewrite it, and the Windows build has no Authenticode signature to fall back on.
- Sparkle was rejected. It covers macOS only, needs an Objective-C bridge and a second framework to sign and notarize, and its UI is not GPUI's.
- Velopack was rejected. It brings its own installers, packaging tool (which needs the .NET SDK), and feed, and verifies only a hash with no publisher signature.
- The `self_update` crate was rejected. It cannot read a disk image or run an installer, and its GitHub backend lists releases through the REST API. The parts it would supply, an HTTP fetch, a hash, and a rename, are already in the engine.
- Swapping files on Windows by renaming the running executable was rejected once Windows got an installer (ADR 0019). The installer already replaces files, shortcuts, and the uninstall entry's version together.
- Rsyncing the new bundle over the running one, as Zed does, was rejected. A crash halfway leaves a mixed bundle, while two renames are atomic and can be undone.

## Consequences

- New secret: `UPDATE_SIGNING_KEY`, the minisign secret key, with its password as `UPDATE_SIGNING_KEY_PASSWORD`. The public key is a constant in the engine. Rotating the key needs a release signed with the old key that carries the new public key.
- New dependencies: `minisign-verify` in `crates/engine` and `minisign` in `xtask`. `xtask/src/graph.rs` and the third-party notices record both.
- The Release and Nightly workflows need `actions: write` to dispatch Catalog. Until the Catalog job runs, which takes a few minutes, a new release is not offered.
- Isolated and `native-test` configurations override the manifest base URL, the public key, and the install location, so tests verify fixture manifests signed with a test key and swap a fixture app in a temporary folder. CI makes no network requests.
- Privacy: a check sends what the catalog request sends, `User-Agent: SkillBinder/<version>` and the user's IP address. A download also reaches GitHub's release host.
