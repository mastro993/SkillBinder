//! Linux: an AppImage's files exist only while it runs, so Install copies its
//! `usr/bin/skillbinder-cli` to `~/.local/bin/skillbinder` and records the copy's digest in
//! application data. A copy with the recorded digest is SkillBinder's; any other file there is
//! someone else's, and never replaced.

use super::link::COMMAND;
use crate::lifecycle::EngineConfig;
use sha2::{Digest, Sha256};
use skillbinder_proto::{CannotInstallCommand, CommandLineState, CommandLineStatus};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(super) struct Copy {
    /// The running AppImage's command; `None` when the app doesn't run as an AppImage.
    pub(super) command: Option<PathBuf>,
    /// `~/.local/bin/skillbinder`.
    pub(super) copy: PathBuf,
    /// The digest of the copy Install wrote, in application data.
    pub(super) record: PathBuf,
    /// Whether the copy's folder is on the PATH the app runs with.
    pub(super) on_path: bool,
}

impl Copy {
    pub(super) fn platform(config: &EngineConfig) -> Self {
        let folder = config.home.join(".local/bin");
        let on_path = config
            .environment
            .get("PATH")
            .is_some_and(|path| std::env::split_paths(path).any(|entry| entry == folder));
        Self {
            command: config
                .placement
                .appimage
                .as_ref()
                .and_then(|_| std::env::current_exe().ok())
                .map(|executable| executable.with_file_name(COMMAND)),
            copy: folder.join("skillbinder"),
            record: config.data_dir.join("command-line.sha256"),
            on_path,
        }
    }

    pub(super) fn status(&self) -> CommandLineStatus {
        let state = match fs::symlink_metadata(&self.copy) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => match self.command {
                Some(_) => CommandLineState::NotInstalled,
                None => CommandLineState::CannotInstall(CannotInstallCommand::NotAppImage),
            },
            Ok(metadata) if metadata.is_file() => self.recorded(),
            _ => CommandLineState::InTheWay,
        };
        CommandLineStatus {
            state,
            path: self.copy.clone(),
        }
    }

    /// Writes the copy through a temporary file renamed over the old one, then records it.
    pub(super) fn install(&self) -> io::Result<()> {
        let command = self
            .command
            .as_deref()
            .ok_or_else(|| io::Error::other("SkillBinder isn't running as an AppImage"))?;
        let folder = self
            .copy
            .parent()
            .ok_or_else(|| io::Error::other("the copy has no folder"))?;
        fs::create_dir_all(folder)?;
        let temporary = folder.join(format!(".skillbinder-{}", uuid::Uuid::new_v4()));
        let written = fs::copy(command, &temporary)
            .and_then(|_| executable(&temporary))
            .and_then(|()| digest(&temporary))
            .and_then(|digest| {
                fs::rename(&temporary, &self.copy)?;
                Ok(digest)
            });
        let digest = written.inspect_err(|_| {
            let _ = fs::remove_file(&temporary);
        })?;
        // An unrecorded copy would read as someone else's, so it goes when the record fails.
        fs::write(&self.record, digest).inspect_err(|_| {
            let _ = fs::remove_file(&self.copy);
        })
    }

    pub(super) fn uninstall(&self) -> io::Result<()> {
        fs::remove_file(&self.copy)?;
        match fs::remove_file(&self.record) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }

    /// The copy's state once it is a file: someone else's unless its digest is recorded, then
    /// leading to another SkillBinder while it differs from the running AppImage's.
    fn recorded(&self) -> CommandLineState {
        let recorded = fs::read_to_string(&self.record).ok();
        let copy = digest(&self.copy).ok();
        match (copy, recorded) {
            (Some(copy), Some(recorded)) if copy == recorded.trim() => {
                let running = self.command.as_deref().map(digest);
                if running.is_some_and(|running| running.ok() != Some(copy)) {
                    CommandLineState::OtherApp
                } else if self.on_path {
                    CommandLineState::Installed
                } else {
                    CommandLineState::NotOnPath
                }
            }
            _ => CommandLineState::InTheWay,
        }
    }
}

/// The file's SHA-256 in hex.
fn digest(path: &Path) -> io::Result<String> {
    let mut hasher = Sha256::new();
    io::copy(&mut fs::File::open(path)?, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(unix)]
fn executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
fn executable(_: &Path) -> io::Result<()> {
    Ok(())
}
