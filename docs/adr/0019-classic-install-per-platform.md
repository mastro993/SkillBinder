# Each platform installs the classic way

Changes the archives [docs/release.md](../release.md) lists: the Windows ZIP and the Linux tarball are replaced.

Until now SkillBinder shipped a plain disk image with no Applications shortcut, a ZIP, and a tarball whose `INSTALL.txt` asked the user to copy files by hand. The app could end up anywhere: run from the disk image, from Downloads, or from a folder it cannot write. App updates (ADR 0017) can only replace an app that sits in a folder it can write. So each platform now installs the way its users expect, and the app makes sure it lives in the right place.

**macOS.** The disk image opens on the classic window: the app, an arrow on the background, and an `Applications` shortcut to drag onto. It is built with `dmgbuild`, which writes the window layout without driving Finder, so it works on a headless CI runner. When the app is launched from anywhere other than `/Applications` or `~/Applications`, it first asks "Move SkillBinder to Applications?" with Move and Not now. Move does four things. It copies the bundle with `ditto` into `/Applications`, or into `~/Applications` when the user cannot write `/Applications`. It moves the original to the Trash, unless it is on the disk image, which it ejects instead. It relaunches from the new place. When a SkillBinder is already there, it replaces it, unless that one is newer, which it opens instead. Not now asks again at the next launch, and until then app updates are download-only.

**Windows.** A per-user setup program, built with Inno Setup 6 and `PrivilegesRequired=lowest`, asks for no administrator rights. It installs into `%LOCALAPPDATA%\Programs\SkillBinder`, adds a Start menu entry, and registers an uninstaller under Settings › Apps. Uninstalling removes the program and keeps the user's data and library. The setup program is the update too (ADR 0018), run silently at quit. The ZIP is dropped. Nothing is code-signed, so SmartScreen still warns once, at install.

**Linux.** One AppImage, `skillbinder-<version>-x86_64.AppImage`, built with a pinned `appimagetool` and its static runtime, which needs no `libfuse2`. It replaces the tarball. On first launch as an AppImage, the app asks once "Add SkillBinder to your applications menu?" with Add and Not now. Add writes `~/.local/share/applications/skillbinder.desktop` pointing at the AppImage, and its icon under `~/.local/share/icons`. Either answer is remembered. An update replaces the AppImage file at its path, so the menu entry keeps working.

## Considered Options

- On macOS, the disk image window without a move prompt was rejected by the user. A user who runs the app from Downloads would never get updates in place.
- On Windows, a per-machine installer in Program Files was rejected by the user. It asks for administrator rights at install and at every update.
- Keeping the ZIP beside the installer was rejected by the user. It means two formats to build and test, and a ZIP copy often lives where it cannot update itself.
- On Linux, keeping the tarball with an install script was rejected by the user in favor of one file that runs on most distributions.
- `.deb` and `.rpm` packages were rejected. The package manager owns root-owned files, so the app could never update itself without an apt or dnf repository to host.
- Flatpak was rejected. Flathub forbids self-updating apps, and its sandbox would need portals for every agent and workspace directory SkillBinder writes.
- `create-dmg` was rejected for the disk image. It scripts Finder through AppleScript, which is flaky on CI runners. `dmgbuild` writes the layout directly.
- WiX MSI and NSIS were rejected for Windows. Inno Setup does a per-user install, a silent update that waits for the app to exit, and a relaunch in one script, and Zed uses it the same way.

## Consequences

- `xtask package` gains three outputs and their tools: `dmgbuild` (Python, pinned), Inno Setup 6, and `appimagetool` (pinned release). `build.yml` installs them, and `docs/release.md` changes its archive table.
- An AppImage needs the FUSE kernel module and `fusermount`. Where they are missing it runs with `--appimage-extract-and-run`, which `INSTALL.txt`'s replacement, the release notes, explain.
- The move prompt is the first thing a new macOS user sees outside Applications, before onboarding.
- Builds from before this change, the ZIP and the tarball, had no updater, so no installs need migrating.
