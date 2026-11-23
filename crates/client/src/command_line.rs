//! The command line tool's row in Settings (ADR 0024): read when Settings opens and after each
//! action, never published in snapshots.

use crate::Client;
use skillbinder_proto::*;

impl Client {
    /// Where `skillbinder` is and what is there.
    pub async fn command_line_status(&self) -> AppResult<CommandLineStatus> {
        self.shared.engine.command_line_status().await
    }
    /// Install and Repair; the result is the state afterwards.
    pub async fn install_command_line(&self) -> AppResult<CommandLineStatus> {
        self.shared.engine.install_command_line().await
    }
    /// Removes what Install wrote; the result is the state afterwards.
    pub async fn uninstall_command_line(&self) -> AppResult<CommandLineStatus> {
        self.shared.engine.uninstall_command_line().await
    }
}
