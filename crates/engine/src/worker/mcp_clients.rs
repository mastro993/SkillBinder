//! The engine's side of Settings › Agents › MCP server (ADR 0026): which agents are connected,
//! and Connect and Disconnect, each on a thread of its own, since the agents' programs take a
//! moment and installing the command first may wait on the administrator prompt (ADR 0027).

use super::{Engine, command_line::on_own_thread};
use crate::logging::Area;
use skillbinder_proto::*;

impl Engine {
    /// Every agent Connect knows that is on this computer, and whether it is connected.
    pub async fn mcp_clients(&self) -> AppResult<Vec<McpClientStatus>> {
        let clients = self.owner.mcp_clients.clone();
        on_own_thread("skillbinder-mcp-clients", move || Ok(clients.statuses())).await
    }
    /// Connects `client` to the installed command, installing the command first when it is
    /// missing.
    pub async fn connect_mcp(&self, client: McpClient) -> AppResult<McpConnect> {
        let (command_line, clients) = (
            self.owner.command_line.clone(),
            self.owner.mcp_clients.clone(),
        );
        let connected = on_own_thread("skillbinder-mcp-clients", move || {
            let mut command = command_line.status();
            if command.state == CommandLineState::NotInstalled {
                command = command_line.install()?;
            }
            match command.state {
                CommandLineState::NotInstalled => return Ok(McpConnect::CommandMissing),
                CommandLineState::InTheWay | CommandLineState::CannotInstall(_) => {
                    return Err(AppError::validation(
                        "SkillBinder can't install `skillbinder` here.",
                    ));
                }
                CommandLineState::Installed
                | CommandLineState::NotOnPath
                | CommandLineState::Homebrew
                | CommandLineState::OtherApp => {}
            }
            Ok(match clients.connect(client, &command.path)? {
                Some(link) => McpConnect::Open(link),
                None => McpConnect::Connected,
            })
        })
        .await;
        self.reported(
            Area::Mcp,
            ("agent connect failed", "agent connect stopped at quit"),
            connected,
        )
    }
    /// Removes `client`'s connection, for the agents whose program can.
    pub async fn disconnect_mcp(&self, client: McpClient) -> AppResult<()> {
        let clients = self.owner.mcp_clients.clone();
        let disconnected = on_own_thread("skillbinder-mcp-clients", move || {
            clients.disconnect(client)
        })
        .await;
        self.reported(
            Area::Mcp,
            (
                "agent disconnect failed",
                "agent disconnect stopped at quit",
            ),
            disconnected,
        )
    }
}
