//! The engine's side of the command line tool (ADR 0024): its state and Install, Repair, and
//! Uninstall, each on a thread of its own, since the macOS administrator prompt waits on the user.

use super::{Engine, action::Action};
use skillbinder_proto::*;

impl Engine {
    /// Where `skillbinder` is and what is there, read from the files.
    pub async fn command_line_status(&self) -> AppResult<CommandLineStatus> {
        let command_line = self.owner.command_line.clone();
        self.action(Action::CommandLineStatus, Vec::new(), |_| {
            on_own_thread(
                "skillbinder-command-line",
                move || Ok(command_line.status()),
            )
        })
        .await
    }
    /// Install and Repair; the result is the state afterwards, unchanged when the user cancels
    /// the administrator prompt.
    pub async fn install_command_line(&self) -> AppResult<CommandLineStatus> {
        let command_line = self.owner.command_line.clone();
        self.action(Action::InstallCommandLine, Vec::new(), |_| {
            on_own_thread("skillbinder-command-line", move || command_line.install())
        })
        .await
    }
    /// Removes what Install wrote.
    pub async fn uninstall_command_line(&self) -> AppResult<CommandLineStatus> {
        let command_line = self.owner.command_line.clone();
        self.action(Action::UninstallCommandLine, Vec::new(), |_| {
            on_own_thread("skillbinder-command-line", move || command_line.uninstall())
        })
        .await
    }
}

/// Runs `operation` off the worker on a thread called `name`, since it may wait on the user or on
/// another program.
pub(super) async fn on_own_thread<T: Send + 'static>(
    name: &str,
    operation: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    let (done, finished) = tokio::sync::oneshot::channel();
    std::thread::Builder::new()
        .name(name.into())
        .spawn(move || {
            let _ = done.send(operation());
        })
        .map_err(|error| {
            AppError::storage().because(format_args!("starting the {name} thread: {error}"))
        })?;
    finished
        .await
        .map_err(|error| AppError::storage().because(format_args!("waiting for {name}: {error}")))?
}
