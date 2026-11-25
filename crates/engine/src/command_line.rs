//! The `skillbinder` command on the PATH (ADR 0024): on macOS a link in `/usr/local/bin`, on
//! Windows the app's `bin` folder in the user's PATH, on Linux a copy in `~/.local/bin`. Its state
//! is read from the files each time, never cached. Install never replaces a `skillbinder`
//! SkillBinder didn't write, and Uninstall removes only what Install wrote.

mod copy;
mod link;
#[cfg(test)]
mod tests;
mod user_path;

use crate::{
    lifecycle::EngineConfig,
    logging::{Area, Level, Logger},
};
use copy::Copy;
use link::Link;
use skillbinder_proto::{AppError, AppResult, CommandLineState, CommandLineStatus, ErrorCategory};
use std::{io, sync::Arc};
use user_path::UserPath;

/// Where this platform puts the command.
enum Place {
    Link(Link),
    UserPath(UserPath),
    Copy(Copy),
}

impl Place {
    fn status(&self) -> CommandLineStatus {
        match self {
            Self::Link(link) => link.status(),
            Self::UserPath(user_path) => user_path.status(),
            Self::Copy(copy) => copy.status(),
        }
    }

    fn install(&self) -> io::Result<()> {
        match self {
            Self::Link(link) => link.install(),
            Self::UserPath(user_path) => user_path.install(),
            Self::Copy(copy) => copy.install(),
        }
    }

    fn uninstall(&self) -> io::Result<()> {
        match self {
            Self::Link(link) => link.uninstall(),
            Self::UserPath(user_path) => user_path.uninstall(),
            Self::Copy(copy) => copy.uninstall(),
        }
    }
}

/// The command line tool's installation, shared by the engine's calls.
#[derive(Clone)]
pub(crate) struct CommandLine {
    place: Arc<Place>,
    log: Logger,
}

impl CommandLine {
    /// The running platform's place for the command, from where the app runs.
    pub(crate) fn platform(config: &EngineConfig, log: Logger) -> Self {
        let place = if cfg!(windows) {
            Place::UserPath(UserPath::platform(&config.placement))
        } else if cfg!(target_os = "linux") {
            Place::Copy(Copy::platform(config))
        } else {
            Place::Link(Link::platform(&config.placement, &config.system_root))
        };
        Self {
            place: Arc::new(place),
            log,
        }
    }

    pub(crate) fn status(&self) -> CommandLineStatus {
        self.place.status()
    }

    /// Install and Repair: puts the command in place, or points it at this app, unless something
    /// else is there or this copy can't. Cancelling the administrator prompt changes nothing.
    pub(crate) fn install(&self) -> AppResult<CommandLineStatus> {
        let before = self.status();
        match before.state {
            CommandLineState::NotInstalled | CommandLineState::OtherApp => self
                .place
                .install()
                .map_err(|error| self.failed("install", &error))?,
            CommandLineState::Installed
            | CommandLineState::NotOnPath
            | CommandLineState::Homebrew => return Ok(before),
            CommandLineState::InTheWay | CommandLineState::CannotInstall(_) => {
                return Err(AppError::validation(
                    "SkillBinder can't install `skillbinder` here.",
                ));
            }
        }
        Ok(self.status())
    }

    /// Removes what Install wrote, and nothing else.
    pub(crate) fn uninstall(&self) -> AppResult<CommandLineStatus> {
        let before = self.status();
        match before.state {
            CommandLineState::Installed | CommandLineState::NotOnPath => self
                .place
                .uninstall()
                .map_err(|error| self.failed("uninstall", &error))?,
            CommandLineState::NotInstalled | CommandLineState::CannotInstall(_) => {
                return Ok(before);
            }
            CommandLineState::Homebrew
            | CommandLineState::OtherApp
            | CommandLineState::InTheWay => {
                return Err(AppError::validation(
                    "SkillBinder didn't install this `skillbinder`, so it leaves it.",
                ));
            }
        }
        Ok(self.status())
    }

    /// Linux, at launch, on a thread of its own: [`Self::refresh`].
    pub(crate) fn refresh_in_background(&self) {
        if !matches!(*self.place, Place::Copy(_)) {
            return;
        }
        let command_line = self.clone();
        let _ = std::thread::Builder::new()
            .name("skillbinder-command-line".into())
            .spawn(move || command_line.refresh());
    }

    /// The copy Install wrote follows the running AppImage, so an app update updates it. A copy
    /// anyone else wrote or changed is left alone.
    fn refresh(&self) {
        if self.status().state == CommandLineState::OtherApp
            && let Err(error) = self.install()
        {
            self.log
                .event(Level::Warn, Area::CommandLine, "command refresh failed")
                .error(&error);
        }
    }

    fn failed(&self, action: &'static str, error: &io::Error) -> AppError {
        self.log
            .event(Level::Error, Area::CommandLine, "command change failed")
            .field("change", action)
            .field("cause", error);
        AppError::new(
            ErrorCategory::Storage,
            format!("Couldn't {action} `skillbinder`: {error}"),
            "Try again. See logs for details.",
        )
    }
}
