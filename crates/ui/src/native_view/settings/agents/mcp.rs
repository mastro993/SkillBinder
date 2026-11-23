//! Settings › Agents › MCP server (spec's Settings › Agents › MCP server, ADR 0026, ADR 0027): a
//! row per agent Connect knows that is on this computer, with Connect, Disconnect, or where to
//! remove it, then Other agents with Copy configuration. Connect and Copy configuration install
//! the command first when it is missing; where the command can't be installed, the group shows only why. The rows are
//! read when Settings opens, when the window regains focus on it, and after each action.

#[cfg(test)]
mod tests;

use super::super::super::NativeView;
use super::super::super::controls::agent_logo;
use super::super::catalog::{Group, SettingRow};
use gpui_kit::{ClipboardItem, Context, IntoElement, Styled, px};
use skillbinder_design::tokens::space;
use skillbinder_design::{AvatarStack, Button, Notice};
use skillbinder_proto::{
    AppError, AppResult, CannotInstallCommand, CommandLineState, CommandLineStatus, McpClient,
    McpClientState, McpClientStatus, McpConnect, mcp_configuration,
};
use std::future::Future;
use std::path::Path;

const TITLE: &str = "MCP server";
const DESCRIPTION: &str = "Let your agents use SkillBinder's tools.";
const INSTALLS: &str = "Connecting also installs the skillbinder command.";
/// What else a search finds the rows by, besides their titles, which name the agents.
const KEYWORDS: &[&str] = &["MCP", "tools"];

/// What the group knows: the agents read last, and whether an action runs.
#[derive(Default)]
pub(in crate::native_view) struct McpRows {
    clients: Option<Vec<McpClientStatus>>,
    working: bool,
}

impl McpRows {
    /// Whether Connect, Disconnect, or Copy configuration's install runs.
    pub(in crate::native_view) fn working(&self) -> bool {
        self.working
    }
}

/// An agent row's control.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Control {
    Connect,
    Disconnect,
    /// Muted: only the agent can remove its connection.
    RemoveIn(&'static str),
    None,
}

/// Why agents can't be connected: there is no stable command path to point them at.
fn unavailable(command: &CommandLineStatus) -> Option<String> {
    let reason = match command.state {
        CommandLineState::CannotInstall(CannotInstallCommand::OutsideApplications) => {
            "Move SkillBinder to Applications to connect agents."
        }
        CommandLineState::CannotInstall(CannotInstallCommand::NotSetUp) => {
            "Install SkillBinder with its setup program to connect agents."
        }
        CommandLineState::CannotInstall(CannotInstallCommand::NotAppImage) => {
            "Run SkillBinder as its AppImage to connect agents."
        }
        CommandLineState::InTheWay => {
            let folder = command.path.parent().unwrap_or(&command.path);
            return Some(format!("Another skillbinder is in {}.", folder.display()));
        }
        CommandLineState::NotInstalled
        | CommandLineState::Installed
        | CommandLineState::Homebrew
        | CommandLineState::OtherApp
        | CommandLineState::NotOnPath => return None,
    };
    Some(reason.to_owned())
}

/// The group's description, which says Connect installs the command while it is missing.
fn description(command: &CommandLineStatus) -> String {
    if command.state == CommandLineState::NotInstalled {
        format!("{DESCRIPTION} {INSTALLS}")
    } else {
        DESCRIPTION.to_owned()
    }
}

/// An agent row's line and control.
fn row(status: &McpClientStatus) -> (String, Control) {
    match status.state {
        McpClientState::Connected if status.client.disconnects() => {
            ("Connected".to_owned(), Control::Disconnect)
        }
        McpClientState::Connected => (
            "Connected".to_owned(),
            Control::RemoveIn(status.client.name()),
        ),
        McpClientState::NotConnected => ("Not connected".to_owned(), Control::Connect),
        McpClientState::NotFound { tool } => (format!("{tool} not found"), Control::None),
    }
}

/// Connect's toast.
fn connected_toast(client: McpClient) -> String {
    let name = client.name();
    format!("{name} can use SkillBinder. Start a new {name} session to load it.")
}

impl NativeView {
    /// The MCP server group, once the command's state is read.
    pub(super) fn mcp_group(&self, cx: &mut Context<Self>) -> Option<Group> {
        let command = self.settings.command_line.status()?;
        if let Some(reason) = unavailable(command) {
            return Some(Group {
                empty: Some(
                    Notice::new("settings-mcp-unavailable", reason)
                        .p(px(space::LG))
                        .into_any_element(),
                ),
                ..Group::titled(TITLE, Vec::new())
            });
        }
        let mcp = &self.settings.mcp;
        let mut rows: Vec<SettingRow> = mcp
            .clients
            .iter()
            .flatten()
            .map(|status| self.mcp_client_row(status, cx))
            .collect();
        rows.push(
            SettingRow::new("settings-mcp-other", "Other agents")
                .detail("Add SkillBinder to any agent's MCP settings.")
                .keywords(KEYWORDS)
                .control(
                    Button::new("settings-mcp-copy")
                        .small()
                        .label("Copy configuration")
                        .inactive(self.settings.changing_command())
                        .on_click(cx.listener(|view, _, _, cx| view.copy_mcp_configuration(cx))),
                ),
        );
        Some(Group {
            description: Some(description(command)),
            ..Group::titled(TITLE, rows)
        })
    }

    fn mcp_client_row(&self, status: &McpClientStatus, cx: &mut Context<Self>) -> SettingRow {
        let client = status.client;
        let id = format!("settings-mcp-{}", client.id());
        let (line, control) = row(status);
        let setting = SettingRow::new(id.clone(), client.name())
            .leading(AvatarStack::new([agent_logo(client.id())]))
            .detail(line)
            .keywords(KEYWORDS);
        let working = self.settings.changing_command();
        match control {
            Control::Connect => setting.control(
                Button::new(format!("{id}-connect"))
                    .small()
                    .label("Connect")
                    .inactive(working)
                    .on_click(cx.listener(move |view, _, _, cx| view.connect_mcp(client, cx))),
            ),
            Control::Disconnect => setting.control(
                Button::new(format!("{id}-disconnect"))
                    .small()
                    .label("Disconnect")
                    .inactive(working)
                    .on_click(cx.listener(move |view, _, _, cx| view.disconnect_mcp(client, cx))),
            ),
            Control::RemoveIn(name) => {
                setting.fact(format!("{id}-remove"), format!("Remove it in {name}"))
            }
            Control::None => setting,
        }
    }

    /// Reads the agents again, as Settings opens or the window regains focus on it. A read while
    /// an action runs is dropped: the action reads them once it ends.
    pub(in crate::native_view) fn read_mcp_clients(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        self.ask_mcp(
            async move { client.mcp_clients().await },
            |view, clients, cx| {
                if let (false, Ok(clients)) = (view.settings.mcp.working, clients) {
                    view.settings.mcp.clients = Some(clients);
                    cx.notify();
                }
            },
            cx,
        );
    }

    /// Connects `mcp_client`, installing the command first when it is missing. Cursor's install
    /// link opens Cursor, which asks the user; its row reads Connected once the window regains
    /// focus after they accept.
    fn connect_mcp(&mut self, mcp_client: McpClient, cx: &mut Context<Self>) {
        if self.settings.changing_command() {
            return;
        }
        self.settings.mcp.working = true;
        let client = self.client.clone();
        self.ask_mcp(
            async move { client.connect_mcp(mcp_client).await },
            move |view, connected, cx| {
                match connected {
                    Ok(McpConnect::Connected) => {
                        view.toast.show(connected_toast(mcp_client).into(), cx);
                    }
                    Ok(McpConnect::Open(link)) => cx.open_url(&link),
                    // Cancelling the administrator prompt leaves it as it was, with no toast.
                    Ok(McpConnect::CommandMissing) => {}
                    Err(error) => view.toast.fail(error.message.into(), cx),
                }
                view.mcp_action_ended(cx);
                // Connect may have installed the command.
                view.read_command_line(cx);
            },
            cx,
        );
    }

    fn disconnect_mcp(&mut self, mcp_client: McpClient, cx: &mut Context<Self>) {
        if self.settings.changing_command() {
            return;
        }
        self.settings.mcp.working = true;
        let client = self.client.clone();
        self.ask_mcp(
            async move { client.disconnect_mcp(mcp_client).await },
            |view, disconnected, cx| {
                if let Err(error) = disconnected {
                    view.toast.fail(error.message.into(), cx);
                }
                view.mcp_action_ended(cx);
            },
            cx,
        );
    }

    /// Copies the configuration that runs the installed command, installing it first when it is
    /// missing, as Connect does, so the copied path exists.
    fn copy_mcp_configuration(&mut self, cx: &mut Context<Self>) {
        let Some(command) = self.settings.command_line.status().cloned() else {
            return;
        };
        if self.settings.changing_command() {
            return;
        }
        if command.state != CommandLineState::NotInstalled {
            self.copied(&command.path, cx);
            return;
        }
        self.settings.mcp.working = true;
        let client = self.client.clone();
        self.ask_mcp(
            async move { client.install_command_line().await },
            |view, installed, cx| {
                match installed {
                    Ok(status)
                        if matches!(
                            status.state,
                            CommandLineState::Installed | CommandLineState::NotOnPath
                        ) =>
                    {
                        view.copied(&status.path, cx);
                    }
                    // Cancelling the administrator prompt leaves it as it was, with no toast.
                    Ok(_) => {}
                    Err(error) => view.toast.fail(error.message.into(), cx),
                }
                view.mcp_action_ended(cx);
                view.read_command_line(cx);
            },
            cx,
        );
    }

    fn copied(&mut self, command: &Path, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(mcp_configuration(command)));
        self.toast.show("Configuration copied".into(), cx);
    }

    fn mcp_action_ended(&mut self, cx: &mut Context<Self>) {
        self.settings.mcp.working = false;
        self.read_mcp_clients(cx);
    }

    /// Asks the engine off the window, then hands the answer to `answered`.
    fn ask_mcp<T: Send + 'static>(
        &mut self,
        asked: impl Future<Output = AppResult<T>> + Send + 'static,
        answered: impl FnOnce(&mut Self, AppResult<T>, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) {
        let handle = self.client.runtime().spawn(asked);
        cx.spawn(async move |this, cx| {
            let result = handle.await.unwrap_or_else(|_| Err(AppError::storage()));
            let _ = this.update(cx, |view, cx| answered(view, result, cx));
        })
        .detach();
        cx.notify();
    }
}
