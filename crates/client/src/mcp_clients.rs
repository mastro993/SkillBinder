//! Settings › Agents › MCP server (ADR 0026): read when Settings opens, when the window regains
//! focus on it, and after each action, never published in snapshots.

use crate::Client;
use skillbinder_proto::*;

impl Client {
    /// Every agent Connect knows that is on this computer, and whether it is connected.
    pub async fn mcp_clients(&self) -> AppResult<Vec<McpClientStatus>> {
        self.shared.engine.mcp_clients().await
    }
    /// Connects `client`, installing the command first when it is missing.
    pub async fn connect_mcp(&self, client: McpClient) -> AppResult<McpConnect> {
        self.shared.engine.connect_mcp(client).await
    }
    /// Removes `client`'s connection.
    pub async fn disconnect_mcp(&self, client: McpClient) -> AppResult<()> {
        self.shared.engine.disconnect_mcp(client).await
    }
}
