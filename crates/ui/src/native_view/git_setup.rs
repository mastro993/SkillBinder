//! Installing Git (ADR 0011, design T1 to T7, S3), shared by onboarding's Git step and Settings'
//! Git row: the action this computer offers, the one status line, and what the app waits on
//! after an action, while it checks for Git every 3 seconds and on focus.

mod flow;
#[cfg(test)]
mod tests;
mod view;

use skillbinder_proto::{Bootstrap, GitInstall};

/// Git's download page.
pub(super) const DOWNLOAD_URL: &str = "https://git-scm.com/downloads";
/// How often the app checks for Git while it waits on it.
pub(super) const CHECK_EVERY: std::time::Duration = std::time::Duration::from_secs(3);
/// How long "Git <version> is ready" shows before the step moves on (T6).
pub(super) const READY_FOR: std::time::Duration = std::time::Duration::from_secs(1);

/// What the user did to get Git.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Action {
    Install,
    Download,
    Copy,
}

/// A button that gets Git; Copy lives in the command box.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum GitButton {
    Install,
    Download,
}

impl GitButton {
    fn action(self) -> Action {
        match self {
            Self::Install => Action::Install,
            Self::Download => Action::Download,
        }
    }
}

/// What the app waits on after an action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Waiting {
    /// The system's installer runs.
    Installer,
    /// Only the app's own check, after Download Git or Copy.
    Check,
}

/// The flow's state the engine does not hold.
#[derive(Debug, Default)]
pub(super) struct GitSetup {
    pub(super) waiting: Option<Waiting>,
}

impl GitSetup {
    /// Install Git always waits on the installer, which the engine starts again unless it still
    /// runs, so a cancelled one can be restarted;
    /// Download Git and Copy wait on the check unless something is waited on already.
    pub(super) fn act(&mut self, action: Action) {
        match action {
            Action::Install => self.waiting = Some(Waiting::Installer),
            Action::Download | Action::Copy => {
                self.waiting.get_or_insert(Waiting::Check);
            }
        }
    }
}

/// The one status line under the lead.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum Status {
    Waiting(&'static str),
    Failed,
    Ready(String),
    TooOld(String),
}

impl Status {
    pub(super) fn text(&self) -> String {
        match self {
            Self::Waiting(line) => (*line).to_owned(),
            Self::Failed => "Couldn't install with winget.".to_owned(),
            Self::Ready(version) => format!("Git {version} is ready"),
            Self::TooOld(version) => {
                format!("Git {version} is too old. SkillBinder needs 2.39 or newer.")
            }
        }
    }
}

/// The buttons and the command box for this computer's offer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Actions<'a> {
    /// Install Git, the primary.
    pub(super) install: bool,
    /// The package manager's command, shown with Copy.
    pub(super) command: Option<&'a str>,
    /// Download Git is the primary, with nothing to install.
    pub(super) download_primary: bool,
}

/// What the snapshot says about Git.
#[derive(Clone, Copy, Debug)]
pub(super) struct GitFacts<'a> {
    pub(super) found: Option<&'a str>,
    pub(super) outdated: Option<&'a str>,
    pub(super) install: &'a GitInstall,
    pub(super) failed: bool,
}

impl<'a> GitFacts<'a> {
    pub(super) fn of(boot: &'a Bootstrap) -> Self {
        Self {
            found: boot.git_version.as_deref(),
            outdated: boot.outdated_git.as_deref(),
            install: &boot.git_install,
            failed: boot.git_install_failed,
        }
    }

    /// "Update Git" while the Git found is too old (T7).
    pub(super) fn title(self) -> &'static str {
        if self.outdated.is_some() {
            "Update Git"
        } else {
            "Install Git"
        }
    }

    /// Git found; else the check waited on after Download Git or Copy; else winget's failure,
    /// which ends its wait; else the installer waited on; else a Git too old (T2, T3, T6, T7).
    pub(super) fn status(self, waiting: Option<Waiting>) -> Option<Status> {
        if let Some(version) = self.found {
            return Some(Status::Ready(version.to_owned()));
        }
        match waiting {
            Some(Waiting::Check) => Some(Status::Waiting("Checking for Git…")),
            _ if self.failed => Some(Status::Failed),
            Some(Waiting::Installer) => Some(Status::Waiting(match self.install {
                GitInstall::AppleInstaller => "Finish in Apple's installer.",
                _ => "Installing Git…",
            })),
            None => self
                .outdated
                .map(|version| Status::TooOld(version.to_owned())),
        }
    }

    /// Install Git while the system's installer can run; else the command with Copy on Linux;
    /// else Download Git as the primary (T2 to T5).
    pub(super) fn actions(self) -> Actions<'a> {
        let install = matches!(
            self.install,
            GitInstall::AppleInstaller | GitInstall::Winget
        ) && !self.failed;
        let command = match self.install {
            GitInstall::Command(command) => Some(command.as_str()),
            _ => None,
        };
        Actions {
            install,
            command,
            download_primary: !install && command.is_none(),
        }
    }
}
