# Releases

SkillBinder ships through this repository's GitHub Releases: a notarized macOS disk image, an unsigned per-user Windows setup program, and a Linux AppImage, described by a signed update manifest that the Catalog workflow mirrors to GitHub Pages (ADR 0018). On macOS, the [Homebrew casks](#homebrew) install the same disk image.

## Versions

The version is `[workspace.package] version` in the root `Cargo.toml`; every workspace member inherits it. `cargo run -p xtask -- version` prints it, `version next <major|minor|patch|X.Y.Z>` prints the release after it, and `version set <version>` writes it to `Cargo.toml` and to the members' entries in `Cargo.lock`, exactly the lines Cargo would change.

| Channel | Version | Tag | GitHub release | Commit on `main` |
|---|---|---|---|---|
| Release | `X.Y.Z`, bumped by hand | `vX.Y.Z` on the bump commit | latest release | `chore(release): vX.Y.Z` |
| Nightly | next patch + `-beta.<run>`, e.g. `0.1.1-beta.12` | `v0.1.1-beta.12` on the built commit | prerelease | none |

`main` therefore always holds the last stable version, and a beta sorts before the patch release that follows it. The macOS bundle's `CFBundleShortVersionString` and `CFBundleVersion` hold only `X.Y.Z`; a beta's suffix is in the archive names.

## Cutting a release

Actions → **Release** → Run workflow, on `main`, with:

- `bump`: `patch`, `minor`, or `major`; or
- `version`: an exact `X.Y.Z` newer than the current one (it wins over `bump`).

[`release.yml`](../.github/workflows/release.yml) then:

1. fails unless every release secret is set, computes the version, fails if its tag already exists, and stamps it into `Cargo.toml` and `Cargo.lock` (the `stamp` artifact);
2. builds and packages every platform from the dispatched commit with that stamp ([`build.yml`](../.github/workflows/build.yml)), signing and notarizing the macOS image, and alongside runs every check `main` runs on that commit again: [`checks.yml`](../.github/workflows/checks.yml) and [`native.yml`](../.github/workflows/native.yml);
3. only if every check passed and all three platforms built: writes `SHA256SUMS` and the signed [update manifest](#update-manifests), then commits the stamp on top of the dispatched commit, tags it, and pushes both to `main` atomically. A signing key that does not open fails the run before the push. If `main` moved during the build the push is rejected and nothing is published; dispatch again;
4. publishes the GitHub release with the archives, `SHA256SUMS`, `update.json`, `update.json.minisig`, and notes that open with the install steps in [`packaging/release-notes.md`](../packaging/release-notes.md) followed by GitHub's generated notes, all in one call;
5. dispatches the Catalog workflow, which mirrors the manifest to Pages;
6. points the `skillbinder` [Homebrew cask](#homebrew) at the new disk image, in a job of its own.

If step 4 fails after the push, `main` holds the bump commit and its tag without a release. Dispatching again would release the next version and leave this one unreleased, so finish it by hand: download the run's `dist-*` artifacts into `dist/`, run `sha256sum -- * > SHA256SUMS` inside it, sign with `UPDATE_SIGNING_KEY` and `UPDATE_SIGNING_KEY_PASSWORD` set: `cargo run -p xtask -- update-manifest --version X.Y.Z --dist dist`, then `gh release create vX.Y.Z --verify-tag --latest --notes-file packaging/release-notes.md --generate-notes dist/*` and `gh workflow run catalog.yml --ref main`.

If step 6 fails, the release is out but Homebrew still installs the previous one. Fix the cause, then re-run the failed `homebrew` job from the run's page ([`homebrew.yml`](../.github/workflows/homebrew.yml)); it skips the commit when the tap already points at this version.

## Nightly betas

[`nightly.yml`](../.github/workflows/nightly.yml) runs every day at 03:23 UTC (GMT), and by hand from the Actions tab. The odd minute avoids the top of the hour, when GitHub delays scheduled runs most; a scheduled run can still start hours late. It skips the night unless a pull request was merged into `main` since the last release or beta (the nearest `v*` tag), by anyone but `github-actions[bot]`; the release bump commit is a direct push, so it never counts. Without any tag yet, it builds. A run queued behind a release that moved `main` skips too. Otherwise it fails unless the signing secrets and the [tap deploy key](#homebrew-tap-deploy-key-release-and-nightly) are set, builds `main` as the next patch's beta while running `checks.yml` and `native.yml` again on that commit, and, only once they all pass, writes and signs the update manifest, tags the commit through the release API (no commit, no deploy key), publishes a prerelease with the manifest among its assets, dispatches the Catalog workflow, and then points the `skillbinder@beta` [Homebrew cask](#homebrew) at the new disk image, in a job of its own that can be re-run alone like Release's. Only the five newest betas are kept; older ones and their tags are deleted. Stable releases are never touched.

## What a release contains

Every archive holds the locked release build without native test controls (`cargo build -p skillbinder -p skillbinder-cli --release --locked`): the app and the `skillbinder` command, at `SkillBinder.app/Contents/MacOS/skillbinder-cli` (signed and notarized with the bundle), `{app}\bin\skillbinder.exe` (a console program), and `usr/bin/skillbinder-cli` in the AppImage ([Native command line](architecture/native-command-line.md)); `LICENSE`, `THIRD_PARTY_NOTICES.md`, and the app icon: `icon.icns` in the macOS bundle, `skillbinder.ico` beside the Windows executable for setup and its shortcuts, and a 256-pixel PNG in the AppImage. The icon files in `assets/` and the disk image background are rendered by `docs/design/brand/source/export-brand.sh`. `cargo run -p xtask -- package <target>` assembles them under `dist/<target>/`.

| Platform | Archive | Built on |
|---|---|---|
| macOS 26+ | `skillbinder-<version>-universal-apple-darwin.dmg`: `SkillBinder.app`, Developer ID signed with the hardened runtime, notarized, ticket stapled to the image. The image opens on SkillBinder, an arrow, and an `Applications` shortcut to drag it onto (ADR 0019). | `macos-26`, `aarch64` and `x86_64` combined with `lipo` |
| Windows | `skillbinder-<version>-x86_64-pc-windows-msvc-setup.exe`: an unsigned Inno Setup program that installs `skillbinder.exe` for the current user, with no administrator prompt, into `%LOCALAPPDATA%\Programs\SkillBinder`, with a Start menu entry and an uninstaller that keeps the user's data (ADR 0019) | `windows-2025` (MSVC) |
| Linux | `skillbinder-<version>-x86_64.AppImage`: one file holding the binary, its `.desktop` entry, and the icon, built on the static type 2 runtime, so it needs no `libfuse2`. At its first launch the app offers to add itself to the applications menu (ADR 0019). | `ubuntu-22.04` (glibc 2.35) |

The disk image is built with [`dmgbuild`](https://github.com/dmgbuild/dmgbuild) (MIT), which writes the window layout without driving Finder, so it works on a headless runner. Its settings are `packaging/macos/dmgbuild-settings.py`, its window background is the brand's `packaging/macos/dmg-background.png` with its `@2x` sibling, and the mounted volume wears the app icon. `dmgbuild` and its dependencies are pinned with their wheels' hashes in `packaging/macos/dmgbuild-requirements.txt`; `build.yml` installs them into a virtual environment, and a local `xtask package` needs the same on `PATH`:

```sh
python3 -m venv .venv-dmgbuild
.venv-dmgbuild/bin/pip install --require-hashes --only-binary :all: -r packaging/macos/dmgbuild-requirements.txt
PATH="$PWD/.venv-dmgbuild/bin:$PATH" cargo run -p xtask -- package universal-apple-darwin
```

The bundle's `Info.plist` names the app `dev.fedemas.skillbinder` ([ADR 0030](adr/0030-bundle-identifier-dev-fedemas-skillbinder.md)); the Homebrew casks quit that identifier on uninstall and zap both its preference file and the one left by betas built as `com.skillbinder.desktop`. It also records the full version, a beta's included, as `SkillBinderVersion`; Move to Applications reads it to tell whether a SkillBinder already there is newer.

The Windows setup program is compiled by [Inno Setup](https://jrsoftware.org/isinfo.php) 6 from `packaging/windows/skillbinder.iss`. `build.yml` downloads Inno Setup 6.7.4's installer from its GitHub release, checks its SHA-256, and installs it for the runner's user; a local `xtask package` finds `ISCC.exe` on `PATH` or in `%ProgramFiles(x86)%\Inno Setup 6`. Setup's version is the full one, a beta's included; its file version holds only `X.Y.Z.0`. The script's `AppId` must never change, since the uninstaller and every update find the installation by it. The same program is the Windows app update, which the app runs silently at quit ([app updates](features/native/app-updates.md#windows)).

The AppImage is built by [`appimagetool`](https://github.com/AppImage/appimagetool) (MIT) from an AppDir whose `AppRun` is the binary, with the [type 2 runtime](https://github.com/AppImage/type2-runtime) (MIT) passed as `--runtime-file`, so a newer upstream runtime never slips in. `build.yml` downloads appimagetool 1.9.1 and runtime 20251108 from their GitHub releases, checks both SHA-256s, and runs the tool with `APPIMAGE_EXTRACT_AND_RUN=1`, as the runner has no FUSE. A local `xtask package` needs `appimagetool` on `PATH` and `SKILLBINDER_APPIMAGE_RUNTIME` naming the runtime:

```sh
curl -fLO https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage
curl -fLO https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64
chmod +x appimagetool-x86_64.AppImage && mkdir -p bin && mv appimagetool-x86_64.AppImage bin/appimagetool
PATH="$PWD/bin:$PATH" SKILLBINDER_APPIMAGE_RUNTIME="$PWD/runtime-x86_64" cargo run -p xtask -- package x86_64-unknown-linux-gnu
```

The runtime needs no `libfuse2`, but it still mounts the AppImage through the FUSE kernel module and `fusermount` (the `fuse3` package on most distributions). Where they are missing, as in some containers, `./skillbinder-<version>-x86_64.AppImage --appimage-extract-and-run` unpacks it to a temporary folder and runs it from there; the release notes say so. A browser drops the executable bit, so the notes also say to `chmod +x` it first.

`SHA256SUMS` beside them lists every archive's checksum, and `update.json` with `update.json.minisig` describes them for the app's updates. A pull request that changes the packaging or these workflows runs `build.yml` unstamped and unsigned, so a broken platform shows up before a release needs it.

On Windows, SmartScreen shows "Windows protected your PC" once, for the unsigned setup program the user downloaded: **More info** → **Run anyway**. App updates run without it, since a setup program the app downloads itself carries no Mark of the Web.

## Homebrew

Releases are also casks in the [`mastro993/homebrew-tap`](https://github.com/mastro993/homebrew-tap) tap, one per channel:

```sh
brew install --cask mastro993/tap/skillbinder        # stable releases
brew install --cask mastro993/tap/skillbinder@beta   # nightly betas
```

Each cask also links the `skillbinder` command into Homebrew's `bin` with its `binary` stanza, which Settings › General then reads as installed by Homebrew (ADR 0024).

Each cask is a template in [`packaging/homebrew/`](../packaging/homebrew/), `skillbinder.rb` and `skillbinder@beta.rb`, with `@VERSION@` and `@SHA256@` standing for the release and its disk image's checksum. [`homebrew.yml`](../.github/workflows/homebrew.yml), called by Release with `skillbinder` and by Nightly with `skillbinder@beta` once the release is published, fills them in from the release's `SHA256SUMS`, writes the result to the tap's `Casks/<cask>.rb`, and pushes it to the tap's `main` as `<cask> X.Y.Z`. Both install the universal disk image's `SkillBinder.app` into `/Applications`, so each conflicts with the other: switching means uninstalling one first. They need macOS 26 (`depends_on macos: :tahoe`) and declare `auto_updates true`, since the app updates itself in place (ADR 0017), so `brew upgrade` leaves them alone unless run with `--greedy`. A beta starts on the Beta update channel, so the beta cask's app keeps updating to betas, and a release's on Stable. Each livecheck reads the version from its channel's update manifest on Pages. `brew uninstall` quits a running SkillBinder first, and `brew uninstall --zap` also deletes its data directory and preferences.

Nightly prunes old betas, so the beta cask's URL breaks only if its tap push fails five nights in a row while betas keep publishing; installed apps are unaffected.

To check a change to a cask, fill in a published release (a beta for `skillbinder@beta`) and audit it from a local tap:

```sh
brew tap-new --no-git local/skillbinder
sha256=$(gh release download vX.Y.Z --pattern SHA256SUMS --output - | awk '$2 ~ /universal-apple-darwin.dmg$/ { print $1 }')
sed -e "s/@VERSION@/X.Y.Z/" -e "s/@SHA256@/$sha256/" packaging/homebrew/skillbinder.rb > "$(brew --repo local/skillbinder)/Casks/skillbinder.rb"
brew trust local/skillbinder && brew audit --cask --strict --online local/skillbinder/skillbinder
brew untap local/skillbinder
```

The audit flags a beta as a GitHub pre-release, which only matters in Homebrew's own taps.

## Update manifests

`cargo run -p xtask -- update-manifest --version <version> --dist <dir>` writes `<dir>/update.json` and signs it with minisign into `<dir>/update.json.minisig`. Every file in `<dir>` other than `SHA256SUMS` and the pair must be a `skillbinder-<version>-<target>.<ext>` archive, the Windows `skillbinder-<version>-<target>-setup.exe`, or the Linux `skillbinder-<version>-<arch>.AppImage`, keyed as `<arch>-unknown-linux-gnu`, at most one per target:

```json
{
  "version": "0.2.0",
  "notes_url": "https://github.com/mastro993/SkillBinder/releases/tag/v0.2.0",
  "published_at": "2026-12-01T10:00:00Z",
  "assets": {
    "universal-apple-darwin": { "url": "https://github.com/mastro993/SkillBinder/releases/download/v0.2.0/skillbinder-0.2.0-universal-apple-darwin.dmg", "sha256": "…", "size": 41234567 }
  }
}
```

Assets are keyed by target triple, so a platform can change its archive format without changing the shape. The app ignores unknown fields; a breaking change publishes `updates/v2/`.

The [Catalog workflow](../.github/workflows/catalog.yml) (`catalog updates --out site`) copies the pairs byte for byte into the Pages site: the latest release's to `updates/v1/stable.json` and `stable.json.minisig`, and the newer of that release and the newest prerelease's to `updates/v1/beta.json` and `beta.json.minisig`. A release published before manifests existed has none, and its channel's files are left out. A Pages deploy replaces the whole site, so every Catalog run, the nightly cron and pushes to `catalog/` included, mirrors them again. When the releases cannot be read, the run carries the live site's `updates/v1/` forward, deploys, and then fails; when the live site cannot be read either, it fails before deploying and the previous site stays up. A new release reaches the app only once the dispatched Catalog run has deployed, a few minutes after it is published.

## Setup

The workflows read these repository secrets (Settings → Secrets and variables → Actions).

### Deploy key (Release only)

The `main` ruleset requires a pull request and lets only deploy keys bypass it. The release job pushes the bump commit with the private half of a write-enabled deploy key stored as `RELEASE_DEPLOY_KEY`. To set it up or rotate it:

```console
ssh-keygen -t ed25519 -N '' -C 'release workflow' -f release_key
gh repo deploy-key add release_key.pub --allow-write --title 'Release workflow'
gh secret set RELEASE_DEPLOY_KEY < release_key
rm release_key release_key.pub
```

When rotating, delete the old key under Settings → Deploy keys.

### Homebrew tap deploy key (Release and Nightly)

The `homebrew` job pushes to the tap with the private half of a write-enabled deploy key on `mastro993/homebrew-tap`, stored here as `HOMEBREW_TAP_DEPLOY_KEY`. To set it up or rotate it:

```console
ssh-keygen -t ed25519 -N '' -C 'SkillBinder release workflow' -f tap_key
gh repo deploy-key add tap_key.pub --repo mastro993/homebrew-tap --allow-write --title 'SkillBinder Release workflow'
gh secret set HOMEBREW_TAP_DEPLOY_KEY < tap_key
rm tap_key tap_key.pub
```

When rotating, delete the old key under the tap's Settings → Deploy keys.

### macOS signing and notarization (Release and Nightly)

| Secret | Value |
|---|---|
| `MACOS_CERTIFICATE` | The **Developer ID Application** certificate with its private key, exported from Keychain Access as `.p12`, base64-encoded |
| `MACOS_CERTIFICATE_PASSWORD` | The `.p12` export password |
| `APPLE_API_KEY` | An App Store Connect API key (`AuthKey_<id>.p8`), the file's contents |
| `APPLE_API_KEY_ID` | That key's ID |
| `APPLE_API_ISSUER` | The issuer ID shown above the keys list |

```console
base64 -i DeveloperIDApplication.p12 | gh secret set MACOS_CERTIFICATE
gh secret set MACOS_CERTIFICATE_PASSWORD
gh secret set APPLE_API_KEY < AuthKey_XXXXXXXXXX.p8
gh secret set APPLE_API_KEY_ID --body XXXXXXXXXX
gh secret set APPLE_API_ISSUER --body xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
```

Create the API key under App Store Connect → Users and Access → Integrations → Team Keys with the Developer role. The build job imports the certificate into a temporary keychain before it compiles, sets `SKILLBINDER_TEAM_ID` to the team ID in the identity's name so the app checks that its updates carry that team's signature, passes the identity to `xtask package` as `SKILLBINDER_SIGNING_IDENTITY`, submits the image with `xcrun notarytool`, prints the notarization log when Apple rejects it, staples the ticket, and checks the image with `spctl`. Locally, `SKILLBINDER_SIGNING_IDENTITY="Developer ID Application: …" cargo run -p xtask -- package` signs the app and the image the same way; without it neither is signed.

### Update signing (Release and Nightly)

| Secret | Value |
|---|---|
| `UPDATE_SIGNING_KEY` | The minisign secret key file's contents |
| `UPDATE_SIGNING_KEY_PASSWORD` | The password that encrypts it |

```console
minisign -G -p skillbinder-update.pub -s skillbinder-update.key
gh secret set UPDATE_SIGNING_KEY < skillbinder-update.key
gh secret set UPDATE_SIGNING_KEY_PASSWORD
```

The app compiles in the public key, `skillbinder-update.pub`, and accepts only manifests it signed. Keep the secret key file and its password backed up outside GitHub, in a password manager or offline: a lost key strands every installed app on its version, since no later release can be signed for it. To rotate the key, publish one release signed with the old key whose app carries the new public key, then replace both secrets; apps that skipped that release cannot update past it.
