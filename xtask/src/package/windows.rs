//! The per-user Windows setup program (ADR 0019), which Inno Setup 6 compiles from
//! `packaging/windows/skillbinder.iss` into `<name>-setup.exe`. Nothing is signed, so SmartScreen
//! warns once at install. The command installs as `{app}\bin\skillbinder.exe`, a console
//! program beside the app's folder.
use super::{Binaries, licenses, run};
use crate::version::Version;
use anyhow::bail;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

const COMPILER: &str = "ISCC.exe";

pub(super) fn package(
    root: &Path,
    binaries: Binaries<'_>,
    output: &Path,
    name: &str,
    version: &Version,
) -> anyhow::Result<()> {
    let program = output.join("SkillBinder");
    // A stale file from an earlier run would otherwise end up in the setup program.
    if program.exists() {
        fs::remove_dir_all(&program)?;
    }
    fs::create_dir_all(program.join("bin"))?;
    fs::copy(binaries.app, program.join("skillbinder.exe"))?;
    fs::copy(binaries.cli, program.join("bin/skillbinder.exe"))?;
    // The executable carries no icon resource, so setup and its shortcuts use this file.
    fs::copy(
        root.join("assets/icon.ico"),
        program.join("skillbinder.ico"),
    )?;
    licenses(root, &program)?;
    let [major, minor, patch] = version.core;
    let setup = format!("{name}-setup");
    run(Command::new(compiler()?)
        .arg("/Q")
        .arg(format!("/DAppVersion={version}"))
        .arg(format!("/DNumericVersion={major}.{minor}.{patch}.0"))
        .arg(format!("/DSourceDir={}", program.display()))
        .arg(format!("/O{}", output.display()))
        .arg(format!("/F{setup}"))
        .arg(root.join("packaging/windows/skillbinder.iss")))?;
    println!("Packaged {}", output.join(format!("{setup}.exe")).display());
    Ok(())
}

/// Inno Setup's compiler: on `PATH`, or in Inno Setup 6's default folder.
fn compiler() -> anyhow::Result<PathBuf> {
    let on_path = env::var_os("PATH")
        .into_iter()
        .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>());
    let installed =
        env::var_os("ProgramFiles(x86)").map(|folder| PathBuf::from(folder).join("Inno Setup 6"));
    match on_path
        .chain(installed)
        .map(|folder| folder.join(COMPILER))
        .find(|compiler| compiler.is_file())
    {
        Some(compiler) => Ok(compiler),
        None => bail!("{COMPILER} is not on PATH: install Inno Setup 6 (docs/release.md)"),
    }
}
