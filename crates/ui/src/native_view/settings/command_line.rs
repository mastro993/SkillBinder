//! Settings › General › Command line tool (spec's Settings table, ADR 0024): whether `skillbinder`
//! works in a new terminal, with Install, Repair, or Uninstall at the row's end. The row reads the
//! state when Settings opens and after each action; Install's toast says how to start using it,
//! and a failure's toast says what failed, leaving the row as it was.

#[cfg(test)]
mod tests;

use super::super::NativeView;
use super::catalog::SettingRow;
use gpui_kit::{Context, IntoElement};
use skillbinder_design::{Button, CodeBlock};
use skillbinder_proto::{
    AppError, AppResult, CannotInstallCommand, CommandLineState, CommandLineStatus,
};
use std::future::Future;

/// The line while nothing has been read, and while nothing is installed.
const INVITE: &str = "Run SkillBinder from a terminal as skillbinder.";
/// The line to add to a shell profile while `~/.local/bin` is off the PATH.
const EXPORT: &str = r#"export PATH="$HOME/.local/bin:$PATH""#;
/// What else a search finds the row by, besides its title and line.
const KEYWORDS: &[&str] = &["terminal", "cli", "path", "shell", "skillbinder"];

/// What the row knows: the state read last, and whether its action runs.
#[derive(Default)]
pub(in crate::native_view) struct CommandLineRow {
    status: Option<CommandLineStatus>,
    working: bool,
}

impl CommandLineRow {
    /// The state read last, if any.
    pub(in crate::native_view) fn status(&self) -> Option<&CommandLineStatus> {
        self.status.as_ref()
    }

    /// Whether Install, Repair, or Uninstall runs.
    pub(in crate::native_view) fn working(&self) -> bool {
        self.working
    }
}

/// The row's control.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Control {
    Install,
    Repair,
    Uninstall,
    /// Muted "Homebrew": `brew` owns the link.
    Homebrew,
    None,
}

/// What the row asked the engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Request {
    Read,
    Install,
    Uninstall,
}

/// The row's line and control for `status`.
fn row(status: &CommandLineStatus) -> (String, Control) {
    let folder = status
        .path
        .parent()
        .map(|folder| folder.display().to_string())
        .unwrap_or_default();
    match status.state {
        CommandLineState::NotInstalled => (INVITE.to_owned(), Control::Install),
        CommandLineState::Installed => (format!("skillbinder in {folder}"), Control::Uninstall),
        CommandLineState::Homebrew => (format!("skillbinder in {folder}"), Control::Homebrew),
        CommandLineState::OtherApp => (
            "skillbinder opens another copy of SkillBinder.".to_owned(),
            Control::Repair,
        ),
        CommandLineState::NotOnPath => (
            "Add ~/.local/bin to your PATH:".to_owned(),
            Control::Uninstall,
        ),
        CommandLineState::InTheWay => (
            format!("Another skillbinder is in {folder}."),
            Control::None,
        ),
        CommandLineState::CannotInstall(reason) => {
            let line = match reason {
                CannotInstallCommand::OutsideApplications => {
                    "Move SkillBinder to Applications to install it."
                }
                CannotInstallCommand::NotSetUp => {
                    "Install SkillBinder with its setup program to add it."
                }
                CannotInstallCommand::NotAppImage => "Run SkillBinder as its AppImage to add it.",
            };
            (line.to_owned(), Control::None)
        }
    }
}

/// Install's toast: on Windows only terminals opened afterwards see the PATH.
fn installed_toast() -> &'static str {
    if cfg!(windows) {
        "Open a new terminal to use skillbinder."
    } else {
        "skillbinder is installed."
    }
}

impl NativeView {
    pub(super) fn command_line_setting(&self, cx: &mut Context<Self>) -> SettingRow {
        let setting =
            SettingRow::new("settings-command-line-row", "Command line tool").keywords(KEYWORDS);
        let Some(status) = &self.settings.command_line.status else {
            return setting.detail(INVITE);
        };
        let (line, control) = row(status);
        let setting = setting.detail(line);
        let setting = if status.state == CommandLineState::NotOnPath {
            setting
                .under([CodeBlock::new("settings-command-line-export", EXPORT).into_any_element()])
        } else {
            setting
        };
        let (id, label, request) = match control {
            Control::Install => ("settings-command-line-install", "Install", Request::Install),
            Control::Repair => ("settings-command-line-repair", "Repair", Request::Install),
            Control::Uninstall => (
                "settings-command-line-uninstall",
                "Uninstall",
                Request::Uninstall,
            ),
            Control::Homebrew => return setting.fact("settings-command-line-homebrew", "Homebrew"),
            Control::None => return setting,
        };
        setting.control(
            Button::new(id)
                .small()
                .label(label)
                .inactive(self.settings.changing_command())
                .on_click(cx.listener(move |view, _, _, cx| view.change_command_line(request, cx))),
        )
    }

    /// Reads the row's state again, as Settings opens.
    pub(in crate::native_view) fn read_command_line(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        self.ask_command_line(
            Request::Read,
            async move { client.command_line_status().await },
            cx,
        );
    }

    fn change_command_line(&mut self, request: Request, cx: &mut Context<Self>) {
        if self.settings.changing_command() {
            return;
        }
        self.settings.command_line.working = true;
        let client = self.client.clone();
        self.ask_command_line(
            request,
            async move {
                match request {
                    Request::Uninstall => client.uninstall_command_line().await,
                    Request::Install | Request::Read => client.install_command_line().await,
                }
            },
            cx,
        );
    }

    fn ask_command_line(
        &mut self,
        request: Request,
        asked: impl Future<Output = AppResult<CommandLineStatus>> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        let handle = self.client.runtime().spawn(asked);
        cx.spawn(async move |this, cx| {
            let result = handle.await.unwrap_or_else(|_| Err(AppError::storage()));
            let _ = this.update(cx, |view, cx| {
                view.command_line_answered(request, result, cx)
            });
        })
        .detach();
        cx.notify();
    }

    /// Shows the state an answer brought. A read while an action runs is older than the action's
    /// answer, so it is dropped.
    fn command_line_answered(
        &mut self,
        request: Request,
        result: AppResult<CommandLineStatus>,
        cx: &mut Context<Self>,
    ) {
        let row = &mut self.settings.command_line;
        if request == Request::Read && row.working {
            return;
        }
        row.working = false;
        match result {
            Ok(status) => {
                // Cancelling the administrator prompt leaves it as it was, with no toast.
                let landed = request == Request::Install
                    && matches!(
                        status.state,
                        CommandLineState::Installed | CommandLineState::NotOnPath
                    );
                row.status = Some(status);
                if landed {
                    self.toast.show(installed_toast().into(), cx);
                }
            }
            Err(error) if request != Request::Read => self.toast.fail(error.message.into(), cx),
            Err(_) => {}
        }
        cx.notify();
    }
}
