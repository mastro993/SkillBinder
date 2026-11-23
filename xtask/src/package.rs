//! Packages a release build into `dist/<target>/`: a macOS app and DMG, a Windows setup program,
//! or a Linux AppImage, named after the workspace version (docs/release.md). Each holds the app
//! and the `skillbinder` command beside it (ADR 0022).
mod linux;
mod macos;
mod windows;

use anyhow::{Context, bail};
use std::{fs, path::Path, process::Command};

pub(crate) fn package(root: &Path, target: Option<&str>) -> anyhow::Result<()> {
    let triple = target.map(str::to_owned).unwrap_or_else(host_triple);
    let release = target.map_or_else(
        || root.join("target/release"),
        |target| root.join("target").join(target).join("release"),
    );
    let suffix = std::env::consts::EXE_SUFFIX;
    let binary = release.join(format!("skillbinder{suffix}"));
    let cli = release.join(format!("skillbinder-cli{suffix}"));
    if !binary.is_file() || !cli.is_file() {
        bail!(
            "Build the locked release application and command (-p skillbinder -p skillbinder-cli) for {triple} before packaging."
        );
    }
    let binaries = Binaries {
        app: &binary,
        cli: &cli,
    };
    let output = root.join("dist").join(&triple);
    fs::create_dir_all(&output)?;
    let version = crate::version::read(root)?;
    let name = format!("skillbinder-{version}-{triple}");
    if cfg!(target_os = "macos") {
        return macos::package(root, binaries, &output, &name, &version);
    }
    if cfg!(windows) {
        return windows::package(root, binaries, &output, &name, &version);
    }
    linux::package(root, binaries, &output, &triple, &version)
}
/// The release executables a package holds.
#[derive(Clone, Copy)]
struct Binaries<'a> {
    /// The app.
    app: &'a Path,
    /// The `skillbinder` command.
    cli: &'a Path,
}
fn licenses(root: &Path, destination: &Path) -> anyhow::Result<()> {
    for name in ["LICENSE", "THIRD_PARTY_NOTICES.md"] {
        fs::copy(root.join(name), destination.join(name))?;
    }
    Ok(())
}
fn run(command: &mut Command) -> anyhow::Result<()> {
    let status = command
        .status()
        .context("Could not start the platform packaging tool")?;
    if !status.success() {
        bail!("Platform packaging failed with {status}.");
    }
    Ok(())
}
fn host_triple() -> String {
    let architecture = std::env::consts::ARCH;
    let platform = if cfg!(target_os = "macos") {
        "apple-darwin"
    } else if cfg!(windows) {
        "pc-windows-msvc"
    } else {
        "unknown-linux-gnu"
    };
    format!("{architecture}-{platform}")
}
