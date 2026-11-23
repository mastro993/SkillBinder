//! The engine's side of the command line tool (ADR 0024): its state and Install, Repair, and
//! Uninstall, each on a thread of its own, since the macOS administrator prompt waits on the user.

use super::Engine;
use crate::command_line::CommandLine;
use skillbinder_proto::*;

impl Engine {
    /// Where `skillbinder` is and what is there, read from the files.
    pub async fn command_line_status(&self) -> AppResult<CommandLineStatus> {
        self.on_command_line(|command_line| Ok(command_line.status()))
            .await
    }
    /// Install and Repair; the result is the state afterwards, unchanged when the user cancels
    /// the administrator prompt.
    pub async fn install_command_line(&self) -> AppResult<CommandLineStatus> {
        self.on_command_line(CommandLine::install).await
    }
    /// Removes what Install wrote.
    pub async fn uninstall_command_line(&self) -> AppResult<CommandLineStatus> {
        self.on_command_line(CommandLine::uninstall).await
    }
    async fn on_command_line(
        &self,
        operation: impl FnOnce(&CommandLine) -> AppResult<CommandLineStatus> + Send + 'static,
    ) -> AppResult<CommandLineStatus> {
        let command_line = self.owner.command_line.clone();
        let (done, finished) = tokio::sync::oneshot::channel();
        std::thread::Builder::new()
            .name("skillbinder-command-line".into())
            .spawn(move || {
                let _ = done.send(operation(&command_line));
            })
            .map_err(|_| AppError::storage())?;
        finished.await.map_err(|_| AppError::storage())?
    }
}
