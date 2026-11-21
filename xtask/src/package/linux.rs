//! The Linux AppImage (ADR 0019), `skillbinder-<version>-<arch>.AppImage`, which `appimagetool`
//! builds from an AppDir with the static type 2 runtime named in `SKILLBINDER_APPIMAGE_RUNTIME`,
//! so it needs no `libfuse2`. The app offers to add itself to the applications menu at its first
//! launch.
use super::licenses;
use crate::version::Version;
use anyhow::{Context as _, ensure};
use std::{env, fs, path::Path, process::Command};

const RUNTIME: &str = "SKILLBINDER_APPIMAGE_RUNTIME";
/// The AppDir's entry, which `appimagetool` requires; the app writes its own, pointing at the
/// AppImage, when the user adds it to the menu.
const DESKTOP_ENTRY: &str = "[Desktop Entry]\nType=Application\nName=SkillBinder\nComment=Manage a local library of agent skills\nExec=skillbinder\nIcon=skillbinder\nStartupWMClass=skillbinder\nTerminal=false\nCategories=Development;Utility;\n";

pub(super) fn package(
    root: &Path,
    binary: &Path,
    output: &Path,
    triple: &str,
    version: &Version,
) -> anyhow::Result<()> {
    let runtime = env::var_os(RUNTIME).with_context(|| {
        format!("{RUNTIME} must name the pinned type 2 runtime (docs/release.md)")
    })?;
    let app_dir = output.join("SkillBinder.AppDir");
    // A stale file from an earlier run would otherwise end up in the AppImage.
    if app_dir.exists() {
        fs::remove_dir_all(&app_dir)?;
    }
    let docs = app_dir.join("usr/share/doc/skillbinder");
    fs::create_dir_all(&docs)?;
    licenses(root, &docs)?;
    // The runtime starts `AppRun`, the binary itself; the window names its own app id.
    fs::copy(binary, app_dir.join("AppRun"))?;
    for icon in ["skillbinder.png", ".DirIcon"] {
        fs::copy(root.join("assets/icon.png"), app_dir.join(icon))?;
    }
    fs::write(app_dir.join("skillbinder.desktop"), DESKTOP_ENTRY)?;
    let architecture = triple.split('-').next().unwrap_or(triple);
    let appimage = output.join(format!("skillbinder-{version}-{architecture}.AppImage"));
    let status = Command::new("appimagetool")
        .env("ARCH", architecture)
        .arg("--no-appstream")
        .arg("--runtime-file")
        .arg(runtime)
        .arg(&app_dir)
        .arg(&appimage)
        .status()
        .context("appimagetool is not on PATH: install the pinned release (docs/release.md)")?;
    ensure!(status.success(), "appimagetool failed with {status}.");
    println!("Packaged {}", appimage.display());
    Ok(())
}
