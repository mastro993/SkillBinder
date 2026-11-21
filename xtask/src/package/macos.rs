//! The macOS app bundle and its DMG, which `dmgbuild` lays out as SkillBinder, an arrow, and an
//! Applications shortcut over the brand's background, with the app icon on the volume (ADR 0019). With `SKILLBINDER_SIGNING_IDENTITY` naming a Developer ID
//! Application certificate in the keychain, both are signed for notarization; otherwise neither is.
use super::{licenses, run};
use crate::version::Version;
use anyhow::Context as _;
use std::{fs, path::Path, process::Command};

const SIGNING_IDENTITY: &str = "SKILLBINDER_SIGNING_IDENTITY";

pub(super) fn package(
    root: &Path,
    binary: &Path,
    output: &Path,
    name: &str,
    version: &Version,
) -> anyhow::Result<()> {
    let app = output.join("SkillBinder.app");
    // A stale bundle from an earlier run would otherwise end up in the image.
    if app.exists() {
        fs::remove_dir_all(&app)?;
    }
    let contents = app.join("Contents");
    fs::create_dir_all(contents.join("MacOS"))?;
    fs::create_dir_all(contents.join("Resources"))?;
    fs::copy(binary, contents.join("MacOS/skillbinder"))?;
    fs::copy(
        root.join("assets/icon.icns"),
        contents.join("Resources/icon.icns"),
    )?;
    fs::write(contents.join("Info.plist"), info_plist(version))?;
    licenses(root, &contents.join("Resources"))?;
    let identity = std::env::var(SIGNING_IDENTITY)
        .ok()
        .filter(|identity| !identity.is_empty());
    if let Some(identity) = &identity {
        // Notarization requires the hardened runtime and a secure timestamp.
        run(Command::new("codesign")
            .args(["--force", "--options", "runtime", "--timestamp", "--sign"])
            .arg(identity)
            .arg(&app))?;
        run(Command::new("codesign")
            .args(["--verify", "--strict", "--verbose=2"])
            .arg(&app))?;
    }
    let image = output.join(format!("{name}.dmg"));
    // An estimate from the bundle alone can leave too little room for the universal binary ("No
    // space left on device"), so size the volume from it with headroom.
    let megabytes = size(&app)? / (1024 * 1024) * 3 / 2 + 32;
    let mut dmgbuild = Command::new("dmgbuild");
    dmgbuild
        .arg("-s")
        .arg(root.join("packaging/macos/dmgbuild-settings.py"))
        .arg("-D")
        .arg(format!("app={}", app.display()))
        .arg("-D")
        .arg(format!("size={megabytes}m"))
        .arg("-D")
        .arg(format!(
            "background={}",
            root.join("packaging/macos/dmg-background.png").display()
        ))
        .arg("SkillBinder")
        .arg(&image);
    let status = dmgbuild.status().context(
        "dmgbuild is not on PATH: install packaging/macos/dmgbuild-requirements.txt (docs/release.md)",
    )?;
    anyhow::ensure!(status.success(), "dmgbuild failed with {status}.");
    if let Some(identity) = &identity {
        run(Command::new("codesign")
            .args(["--force", "--timestamp", "--sign"])
            .arg(identity)
            .arg(&image))?;
    }
    println!("Packaged {} and {}", app.display(), image.display());
    Ok(())
}

/// The total size in bytes of the files under `path`.
fn size(path: &Path) -> anyhow::Result<u64> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() {
        return Ok(metadata.len());
    }
    fs::read_dir(path)?.try_fold(0, |total, entry| Ok(total + size(&entry?.path())?))
}

/// Bundle versions are plain `X.Y.Z`: a beta carries its prerelease in the image's name and in
/// `SkillBinderVersion`, which Move to Applications compares with a copy already installed.
fn info_plist(version: &Version) -> String {
    let [major, minor, patch] = version.core;
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>SkillBinder</string>
<key>CFBundleDisplayName</key><string>SkillBinder</string>
<key>CFBundleExecutable</key><string>skillbinder</string>
<key>CFBundleIdentifier</key><string>com.skillbinder.desktop</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleIconFile</key><string>icon</string>
<key>CFBundleShortVersionString</key><string>{major}.{minor}.{patch}</string>
<key>CFBundleVersion</key><string>{major}.{minor}.{patch}</string>
<key>SkillBinderVersion</key><string>{version}</string>
<key>LSMinimumSystemVersion</key><string>26.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
"#
    )
}
