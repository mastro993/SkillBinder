//! Windows: the setup program installs `bin\skillbinder.exe` beside the app, and Install adds that
//! `bin` folder once to the user's PATH in `HKCU\Environment`, then tells running programs the
//! environment changed, so terminals opened afterwards find the command.

use crate::app_update::AppPlacement;
use skillbinder_proto::{CannotInstallCommand, CommandLineState, CommandLineStatus};
use std::{io, path::PathBuf};

pub(super) struct UserPath {
    /// `{app}\bin`.
    bin: PathBuf,
    /// Whether the setup program installed this copy.
    set_up: bool,
}

impl UserPath {
    pub(super) fn platform(placement: &AppPlacement) -> Self {
        let app = placement
            .executable
            .as_deref()
            .and_then(|executable| executable.parent())
            .map(PathBuf::from)
            .unwrap_or_default();
        Self {
            bin: app.join("bin"),
            set_up: placement.set_up_executable().is_some(),
        }
    }

    pub(super) fn status(&self) -> CommandLineStatus {
        let state = if !self.set_up {
            CommandLineState::CannotInstall(CannotInstallCommand::NotSetUp)
        } else if registry::read().is_ok_and(|path| contains(&path, &self.bin())) {
            CommandLineState::Installed
        } else {
            CommandLineState::NotInstalled
        };
        CommandLineStatus {
            state,
            path: self.bin.join("skillbinder.exe"),
        }
    }

    pub(super) fn install(&self) -> io::Result<()> {
        let path = registry::read()?;
        if !contains(&path, &self.bin()) {
            registry::write(with(&path, &self.bin()))?;
        }
        Ok(())
    }

    pub(super) fn uninstall(&self) -> io::Result<()> {
        registry::write(without(&registry::read()?, &self.bin()))
    }

    fn bin(&self) -> String {
        self.bin.display().to_string()
    }
}

/// Whether a PATH entry names `bin`, ignoring case and a trailing separator.
fn same(entry: &str, bin: &str) -> bool {
    let trim = |path: &str| path.trim().trim_end_matches(['\\', '/']).to_lowercase();
    trim(entry) == trim(bin)
}

pub(super) fn contains(path: &str, bin: &str) -> bool {
    path.split(';').any(|entry| same(entry, bin))
}

/// `path` with `bin` added at its end.
pub(super) fn with(path: &str, bin: &str) -> String {
    let path = path.trim_end_matches(';');
    if path.is_empty() {
        bin.to_owned()
    } else {
        format!("{path};{bin}")
    }
}

/// `path` without any entry naming `bin`.
pub(super) fn without(path: &str, bin: &str) -> String {
    path.split(';')
        .filter(|entry| !same(entry, bin))
        .collect::<Vec<_>>()
        .join(";")
}

/// The user's PATH, `HKCU\Environment\Path`, an expandable string.
#[cfg(windows)]
mod registry {
    use std::{io, os::windows::process::CommandExt, process::Command};
    use winsafe::{self as w, co};

    fn environment(access: co::KEY) -> io::Result<w::guard::RegCloseKeyGuard> {
        w::HKEY::CURRENT_USER
            .RegOpenKeyEx(Some("Environment"), co::REG_OPTION::default(), access)
            .map_err(error)
    }

    fn error(code: co::ERROR) -> io::Error {
        io::Error::from_raw_os_error(code.raw() as i32)
    }

    /// The user's PATH; none is an empty one.
    pub(super) fn read() -> io::Result<String> {
        match environment(co::KEY::READ)?.RegQueryValueEx(Some("Path")) {
            Ok(w::RegistryValue::Sz(path) | w::RegistryValue::ExpandSz(path)) => Ok(path),
            Ok(_) => Err(io::Error::other("the user's PATH isn't text")),
            Err(co::ERROR::FILE_NOT_FOUND) => Ok(String::new()),
            Err(code) => Err(error(code)),
        }
    }

    /// Saves the user's PATH, then tells running programs the environment changed.
    pub(super) fn write(path: String) -> io::Result<()> {
        environment(co::KEY::SET_VALUE)?
            .RegSetValueEx(Some("Path"), w::RegistryValue::ExpandSz(path))
            .map_err(error)?;
        broadcast();
        Ok(())
    }

    /// Broadcasts `WM_SETTINGCHANGE` with "Environment", so Explorer, and the terminals it starts
    /// afterwards, read the new PATH. The workspace forbids the `unsafe` call that sends it, so
    /// .NET sends it: removing a user variable, here one that never exists, always broadcasts.
    fn broadcast() {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        let powershell =
            std::path::Path::new(&root).join("System32\\WindowsPowerShell\\v1.0\\powershell.exe");
        let _ = Command::new(powershell)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[Environment]::SetEnvironmentVariable('SKILLBINDER_PATH_CHANGED', $null, 'User')",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
}

#[cfg(not(windows))]
mod registry {
    use std::io;

    pub(super) fn read() -> io::Result<String> {
        Err(io::ErrorKind::Unsupported.into())
    }

    pub(super) fn write(_: String) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }
}
