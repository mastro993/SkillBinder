//! Settings › Agents › MCP server (ADR 0026): the agents SkillBinder connects to its MCP server
//! through their own installers, and whether each is connected.

use std::path::Path;

/// An agent Connect knows how to connect, in the order Settings lists them.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum McpClient {
    /// Connected with `claude mcp add`.
    ClaudeCode,
    /// Connected with `codex mcp add`.
    Codex,
    /// Connected with `code --add-mcp`.
    VsCode,
    /// Connected through its install link.
    Cursor,
}

impl McpClient {
    /// Every agent, in Settings' order.
    pub const ALL: [Self; 4] = [Self::ClaudeCode, Self::Codex, Self::VsCode, Self::Cursor];

    /// The agent's name as the user knows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex",
            Self::VsCode => "VS Code",
            Self::Cursor => "Cursor",
        }
    }

    /// Its registry id, which names its logo and its element ids.
    pub fn id(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Codex => "codex",
            Self::VsCode => "vscode",
            Self::Cursor => "cursor",
        }
    }

    /// The registry id of the agent whose global skill directory it serves, which onboarding
    /// checks is chosen and whose logo it draws: VS Code runs GitHub Copilot.
    pub fn agent(self) -> &'static str {
        match self {
            Self::VsCode => "github-copilot",
            Self::ClaudeCode | Self::Codex | Self::Cursor => self.id(),
        }
    }

    /// Whether SkillBinder can remove the connection, or only the agent itself can.
    pub fn disconnects(self) -> bool {
        matches!(self, Self::ClaudeCode | Self::Codex)
    }
}

/// Whether an agent on this computer is connected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum McpClientState {
    /// Its settings name SkillBinder's server.
    Connected,
    /// Connect can connect it.
    NotConnected,
    /// The agent is here, but the program Connect runs is not found.
    NotFound {
        /// The program's name, such as `claude`.
        tool: &'static str,
    },
}

/// One agent found on this computer, and its state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct McpClientStatus {
    /// Which agent.
    pub client: McpClient,
    /// Whether it is connected.
    pub state: McpClientState,
}

/// What Connect did.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum McpConnect {
    /// The agent's installer connected it.
    Connected,
    /// The agent asks the user itself: the app opens this install link.
    Open(String),
    /// The user cancelled installing the command first, so nothing changed.
    CommandMissing,
}

/// Copy configuration's JSON, which any agent's MCP settings take.
pub fn mcp_configuration(command: &Path) -> String {
    let configuration = serde_json::json!({
        "mcpServers": {
            "skillbinder": { "command": command.to_string_lossy(), "args": ["mcp"] }
        }
    });
    serde_json::to_string_pretty(&configuration).unwrap_or_default()
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "Fixture failures must abort the test.")]
mod tests {
    use super::*;

    #[test]
    fn the_configuration_names_the_command_with_its_path_escaped() {
        let command = Path::new(r#"/Users/a "b"/bin/skillbinder"#);
        let parsed: serde_json::Value = serde_json::from_str(&mcp_configuration(command)).unwrap();
        assert_eq!(
            parsed,
            serde_json::json!({"mcpServers": {"skillbinder": {
                "command": r#"/Users/a "b"/bin/skillbinder"#, "args": ["mcp"]
            }}})
        );
    }
}
