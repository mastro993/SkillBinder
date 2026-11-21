//! Packages a release build into `dist/<target>/`: a macOS app and DMG, a Windows setup program,
//! or a Linux tarball, named after the workspace version (docs/release.md).
mod macos;
mod windows;

use anyhow::{Context, bail};
use std::{fs, path::Path, process::Command};

pub(crate) fn package(root: &Path, target: Option<&str>) -> anyhow::Result<()> {
    let triple = target.map(str::to_owned).unwrap_or_else(host_triple);
    let executable = if cfg!(windows) {
        "skillbinder.exe"
    } else {
        "skillbinder"
    };
    let binary = target.map_or_else(
        || root.join("target/release").join(executable),
        |target| {
            root.join("target")
                .join(target)
                .join("release")
                .join(executable)
        },
    );
    if !binary.is_file() {
        bail!("Build the locked release application for {triple} before packaging.");
    }
    let output = root.join("dist").join(&triple);
    fs::create_dir_all(&output)?;
    let version = crate::version::read(root)?;
    let name = format!("skillbinder-{version}-{triple}");
    if cfg!(target_os = "macos") {
        return macos::package(root, &binary, &output, &name, &version);
    }
    if cfg!(windows) {
        return windows::package(root, &binary, &output, &name, &version);
    }
    let directory = output.join("SkillBinder");
    fs::create_dir_all(&directory)?;
    fs::copy(binary, directory.join(executable))?;
    licenses(root, &directory)?;
    fs::copy(
        root.join("assets/icon.png"),
        directory.join("skillbinder.png"),
    )?;
    fs::write(
        directory.join("skillbinder.desktop"),
        "[Desktop Entry]\nType=Application\nName=SkillBinder\nComment=Manage a local library of agent skills\nExec=skillbinder\nIcon=skillbinder\nStartupWMClass=skillbinder\nTerminal=false\nCategories=Development;Utility;\n",
    )?;
    fs::write(
        directory.join("INSTALL.txt"),
        "Place skillbinder on PATH. Copy skillbinder.desktop to ~/.local/share/applications/ and skillbinder.png to ~/.local/share/icons/. The application needs a working X11 or Wayland desktop and a directory-selection portal.\n",
    )?;
    let archive = output.join(format!("{name}.tar.gz"));
    run(Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(&output)
        .arg("SkillBinder"))?;
    println!("Packaged {}", archive.display());
    Ok(())
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
