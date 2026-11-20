//! The macOS app bundle and its DMG. With `SKILLBINDER_SIGNING_IDENTITY` naming a Developer ID
//! Application certificate in the keychain, both are signed for notarization; otherwise neither is.
use super::{licenses, run};
use crate::version::Version;
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
        root.join("assets/icon.png"),
        contents.join("Resources/icon.png"),
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
    run(Command::new("hdiutil")
        .args([
            "create",
            "-ov",
            "-format",
            "UDZO",
            "-volname",
            "SkillBinder",
            "-srcfolder",
        ])
        .arg(&app)
        .arg(&image))?;
    if let Some(identity) = &identity {
        run(Command::new("codesign")
            .args(["--force", "--timestamp", "--sign"])
            .arg(identity)
            .arg(&image))?;
    }
    println!("Packaged {} and {}", app.display(), image.display());
    Ok(())
}

/// Bundle versions are plain `X.Y.Z`: a beta carries its prerelease only in the image's name.
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
<key>CFBundleShortVersionString</key><string>{major}.{minor}.{patch}</string>
<key>CFBundleVersion</key><string>{major}.{minor}.{patch}</string>
<key>LSMinimumSystemVersion</key><string>26.0</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
"#
    )
}
