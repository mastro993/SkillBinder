//! macOS: `/usr/local/bin/skillbinder`, a link to `Contents/MacOS/skillbinder-cli` in the app's
//! bundle, which is on every shell's PATH through `/etc/paths`. `/usr/local` belongs to root, so
//! the link is written through the administrator prompt when the user can't write the folder.
//! The Homebrew cask links the command itself; its link is known by the cask's `Caskroom` entry
//! under the prefix that holds it, and left to `brew`.

use crate::{
    app_update::{AppPlacement, bundle_of},
    persistence,
};
use skillbinder_proto::{CannotInstallCommand, CommandLineState, CommandLineStatus};
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

/// The command's name inside the bundle.
pub(super) const COMMAND: &str = "skillbinder-cli";
/// The casks whose `binary` stanza links the command.
const CASKS: [&str; 2] = ["skillbinder", "skillbinder@beta"];
/// What osascript reports when the user cancels the administrator prompt.
const CANCELLED: &str = "(-128)";

pub(super) struct Link {
    /// This bundle's command; `None` while the app runs outside Applications, where a link would
    /// break, or outside a bundle.
    pub(super) command: Option<PathBuf>,
    /// `/usr/local/bin/skillbinder`.
    pub(super) link: PathBuf,
    /// The Homebrew prefixes, `/opt/homebrew` and `/usr/local`.
    pub(super) prefixes: Vec<PathBuf>,
}

impl Link {
    /// The link and prefixes under `root`, which is `/` outside a fixture.
    pub(super) fn platform(placement: &AppPlacement, root: &Path) -> Self {
        Self {
            command: placement
                .bundle_in_applications()
                .map(|bundle| bundle.join("Contents/MacOS").join(COMMAND)),
            link: root.join("usr/local/bin/skillbinder"),
            prefixes: vec![root.join("opt/homebrew"), root.join("usr/local")],
        }
    }

    pub(super) fn status(&self) -> CommandLineStatus {
        if let Some(path) = self.homebrew() {
            return CommandLineStatus {
                state: CommandLineState::Homebrew,
                path,
            };
        }
        let state = match fs::symlink_metadata(&self.link) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => CommandLineState::NotInstalled,
            Ok(metadata) if metadata.file_type().is_symlink() => self.leads(),
            _ => CommandLineState::InTheWay,
        };
        let state = match (state, &self.command) {
            (CommandLineState::NotInstalled | CommandLineState::OtherApp, None) => {
                CommandLineState::CannotInstall(CannotInstallCommand::OutsideApplications)
            }
            (state, _) => state,
        };
        CommandLineStatus {
            state,
            path: self.link.clone(),
        }
    }

    /// Writes the link as the user when the folder can be written, else through the
    /// administrator prompt, which also creates the folder.
    pub(super) fn install(&self) -> io::Result<()> {
        let command = self
            .command
            .as_deref()
            .ok_or_else(|| io::Error::other("SkillBinder isn't in Applications"))?;
        let folder = self.folder()?;
        if !persistence::writable(folder) {
            return as_administrator(
                r#""/bin/mkdir -p -m 755 " & quoted form of item 1 of argv & " && /bin/ln -sfn " & quoted form of item 2 of argv & " " & quoted form of item 3 of argv"#,
                &[folder, command, &self.link],
            );
        }
        // A link beside it, renamed over the old one, replaces it at once.
        let temporary = folder.join(format!(".skillbinder-{}", uuid::Uuid::new_v4()));
        symlink(command, &temporary)?;
        fs::rename(&temporary, &self.link).inspect_err(|_| {
            let _ = fs::remove_file(&temporary);
        })
    }

    pub(super) fn uninstall(&self) -> io::Result<()> {
        if persistence::writable(self.folder()?) {
            fs::remove_file(&self.link)
        } else {
            as_administrator(
                r#""/bin/rm -f " & quoted form of item 1 of argv"#,
                &[&self.link],
            )
        }
    }

    fn folder(&self) -> io::Result<&Path> {
        self.link
            .parent()
            .ok_or_else(|| io::Error::other("the link has no folder"))
    }

    /// The link a cask's `binary` stanza wrote under a prefix that holds the cask.
    fn homebrew(&self) -> Option<PathBuf> {
        self.prefixes
            .iter()
            .filter(|prefix| {
                CASKS
                    .iter()
                    .any(|cask| prefix.join("Caskroom").join(cask).exists())
            })
            .map(|prefix| prefix.join("bin/skillbinder"))
            .find(|link| is_link(link))
    }

    /// Whether the link leads to this app's command, another SkillBinder's, or something else.
    fn leads(&self) -> CommandLineState {
        let Ok(target) = fs::read_link(&self.link) else {
            return CommandLineState::InTheWay;
        };
        // A relative target is read from the link's folder; joining an absolute one keeps it.
        let target = self
            .link
            .parent()
            .map_or_else(|| target.clone(), |folder| folder.join(&target));
        let ours = self
            .command
            .as_deref()
            .and_then(|command| dunce::canonicalize(command).ok());
        if ours.is_some() && dunce::canonicalize(&target).ok() == ours {
            CommandLineState::Installed
        } else if target.file_name().is_some_and(|name| name == COMMAND)
            && bundle_of(&target).is_some()
        {
            CommandLineState::OtherApp
        } else {
            CommandLineState::InTheWay
        }
    }
}

fn is_link(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink())
}

#[cfg(unix)]
fn symlink(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn symlink(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::ErrorKind::Unsupported.into())
}

/// Runs `script`, an AppleScript expression of the shell command reading `arguments` as `argv`,
/// with administrator privileges. The paths travel as arguments, quoted by `quoted form of`, so no
/// path is ever parsed as AppleScript or shell. Cancelling the prompt is no failure: nothing
/// changed.
fn as_administrator(script: &str, arguments: &[&Path]) -> io::Result<()> {
    let output = Command::new("/usr/bin/osascript")
        .args([
            "-e",
            "on run argv",
            "-e",
            &format!("do shell script ({script}) with administrator privileges"),
            "-e",
            "end run",
        ])
        .args(arguments)
        .stdin(Stdio::null())
        .output()?;
    let error = String::from_utf8_lossy(&output.stderr);
    if output.status.success() || error.contains(CANCELLED) {
        Ok(())
    } else {
        Err(io::Error::other(error.trim().to_owned()))
    }
}
