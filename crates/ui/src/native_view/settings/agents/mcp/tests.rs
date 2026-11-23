use super::super::super::catalog::{Section, search};
use super::*;
use std::path::PathBuf;

fn command(state: CommandLineState) -> CommandLineStatus {
    CommandLineStatus {
        state,
        path: PathBuf::from("/usr/local/bin/skillbinder"),
    }
}

#[test]
fn the_group_shows_only_why_where_the_command_cant_be_installed() {
    let cases = [
        (
            CommandLineState::CannotInstall(CannotInstallCommand::OutsideApplications),
            Some("Move SkillBinder to Applications to connect agents."),
        ),
        (
            CommandLineState::CannotInstall(CannotInstallCommand::NotSetUp),
            Some("Install SkillBinder with its setup program to connect agents."),
        ),
        (
            CommandLineState::InTheWay,
            Some("Another skillbinder is in /usr/local/bin."),
        ),
        (CommandLineState::NotInstalled, None),
        (CommandLineState::Installed, None),
        (CommandLineState::Homebrew, None),
    ];
    for (state, reason) in cases {
        assert_eq!(unavailable(&command(state)).as_deref(), reason, "{state:?}");
    }
}

#[test]
fn the_description_says_connecting_installs_the_missing_command() {
    assert_eq!(
        description(&command(CommandLineState::NotInstalled)),
        "Let your agents use SkillBinder's tools. Connecting also installs the skillbinder command."
    );
    assert_eq!(
        description(&command(CommandLineState::Installed)),
        DESCRIPTION
    );
}

#[test]
fn each_state_has_its_line_and_control() {
    let status = |client, state| McpClientStatus { client, state };
    let cases = [
        (
            status(McpClient::ClaudeCode, McpClientState::Connected),
            ("Connected", Control::Disconnect),
        ),
        (
            status(McpClient::Codex, McpClientState::NotConnected),
            ("Not connected", Control::Connect),
        ),
        (
            status(McpClient::VsCode, McpClientState::Connected),
            ("Connected", Control::RemoveIn("VS Code")),
        ),
        (
            status(McpClient::Cursor, McpClientState::Connected),
            ("Connected", Control::RemoveIn("Cursor")),
        ),
        (
            status(McpClient::VsCode, McpClientState::NotFound { tool: "code" }),
            ("code not found", Control::None),
        ),
    ];
    for (status, (line, control)) in cases {
        assert_eq!(row(&status), (line.to_owned(), control), "{status:?}");
    }
    assert_eq!(
        connected_toast(McpClient::ClaudeCode),
        "Claude Code can use SkillBinder. Start a new Claude Code session to load it."
    );
}

#[test]
fn search_finds_the_rows_by_mcp_tools_and_the_agents_name() {
    let rows = || {
        vec![
            SettingRow::new("settings-mcp-claude-code", "Claude Code")
                .detail("Connected")
                .keywords(KEYWORDS),
        ]
    };
    for query in ["mcp", "tools", "claude"] {
        let groups = vec![Group::new(rows())];
        assert_eq!(search(Section::Agents, groups, query).len(), 1, "{query}");
    }
}
