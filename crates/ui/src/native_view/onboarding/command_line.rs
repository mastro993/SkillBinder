//! The Use SkillBinder from your terminal step (spec's command step, ADR 0027): the lead, an
//! example command, where Install puts it, and one status line, then Skip and Install at the
//! right, as the prototype draws them. Installed, it reads "skillbinder is ready" and moves on by
//! itself; on Linux with `~/.local/bin` off the PATH it shows the line to add and waits for
//! Continue.

#[cfg(test)]
mod tests;

use super::super::git_setup::READY_FOR;
use super::super::settings::command_line::EXPORT;
use super::super::{Effect, NativeView};
use super::offers::Install;
use super::{Place, StepView};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div, px};
use skillbinder_design::tokens::{radius, space, text};
use skillbinder_design::{Button, CodeBlock, Notice, NoticeSize, Status, Text, TextTone};
use skillbinder_proto::{AppResult, CommandLineState, CommandLineStatus};

const TITLE: &str = "Use SkillBinder from your terminal";
const LEAD: &str = "Install the skillbinder command to manage your skills from a terminal, a \
                    script, or an agent. You can do this later in Settings.";
const EXAMPLE: &str = "skillbinder preset activate Work";

/// Where Install puts the command on this platform.
fn installs_to() -> &'static str {
    if cfg!(windows) {
        "Installs to your PATH."
    } else if cfg!(target_os = "linux") {
        "Installs to ~/.local/bin."
    } else {
        "Installs to /usr/local/bin."
    }
}

/// The status line, with its tone, for what Install did and the command's state.
fn line(install: &Install, state: Option<CommandLineState>) -> Option<(String, Option<Status>)> {
    match (install, state) {
        (Install::Running, _) if cfg!(target_os = "macos") => {
            Some(("Enter your password in the macOS prompt.".to_owned(), None))
        }
        (Install::Running, _) => Some(("Installing…".to_owned(), None)),
        (Install::Failed(reason), _) => Some((reason.clone(), Some(Status::Danger))),
        (Install::Idle, Some(CommandLineState::Installed)) => {
            let ready = if cfg!(windows) {
                "skillbinder is ready. Open a new terminal to use it."
            } else {
                "skillbinder is ready"
            };
            Some((ready.to_owned(), Some(Status::Success)))
        }
        (Install::Idle, Some(CommandLineState::NotOnPath)) => {
            Some(("Add ~/.local/bin to your PATH:".to_owned(), None))
        }
        (Install::Idle, _) => None,
    }
}

/// Install put the command in place, though on Linux perhaps off the PATH.
fn landed(status: &CommandLineStatus) -> bool {
    matches!(
        status.state,
        CommandLineState::Installed | CommandLineState::NotOnPath
    )
}

impl NativeView {
    /// The step at `place`; gone back to once settled, it offers no Skip, and the frame's
    /// Continue is the primary.
    pub(super) fn command_line_step(&mut self, place: &Place, cx: &mut Context<Self>) -> StepView {
        self.read_command(cx);
        let settled = place.forward.is_some();
        let state = self.offers.command.as_ref().map(|status| status.state);
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let example = div()
            .px(px(space::LG))
            .py(px(space::SM))
            .rounded(px(radius::MD))
            .bg(theme.muted)
            .text_color(muted)
            .child(
                Text::labeled("onboarding-command-example", format!("$ {EXAMPLE}"))
                    .level(text::MONO_SM),
            );
        let mut body = vec![
            div().text_color(muted).child(LEAD).into_any_element(),
            example.into_any_element(),
        ];
        match line(&self.offers.install, state) {
            Some((text, status)) => {
                let notice = Notice::new("onboarding-command-line", text).size(NoticeSize::Md);
                let notice = match status {
                    Some(status) => notice.status(status),
                    None if self.offers.install == Install::Running => notice.turning("loading"),
                    None => notice,
                };
                body.push(notice.into_any_element());
            }
            None => body.push(
                Text::labeled("onboarding-command-place", installs_to())
                    .level(text::BODY_SM)
                    .tone(TextTone::Muted)
                    .into_any_element(),
            ),
        }
        if state == Some(CommandLineState::NotOnPath) {
            body.push(CodeBlock::new("onboarding-command-export", EXPORT).into_any_element());
        }
        let actions = self.command_actions(place, settled, state, cx);
        StepView {
            title: TITLE,
            wide: false,
            body,
            leading: None,
            actions,
        }
    }

    /// Skip, then Install, or Try again after a failure, beside each other as in the prototype;
    /// installed off the PATH, Continue while the frame holds the step; installed, nothing.
    fn command_actions(
        &self,
        place: &Place,
        settled: bool,
        state: Option<CommandLineState>,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        match state {
            Some(CommandLineState::NotOnPath) if self.held_step.is_some() && !settled => {
                return vec![
                    Button::new("onboarding-command-continue")
                        .label("Continue")
                        .primary()
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.held_step = None;
                            cx.notify();
                        }))
                        .into_any_element(),
                ];
            }
            Some(CommandLineState::Installed | CommandLineState::NotOnPath) => return Vec::new(),
            _ => {}
        }
        let skip = (!settled).then(|| {
            Button::new("onboarding-skip-command")
                .label("Skip")
                .ghost()
                .inactive(self.blocked())
                .on_click(cx.listener(|view, _, _, cx| view.settle_command_step(cx)))
                .into_any_element()
        });
        let label = match self.offers.install {
            Install::Failed(_) => "Try again",
            Install::Idle | Install::Running => "Install",
        };
        let install = Button::new("onboarding-install-command").label(label);
        let install = if settled { install } else { install.primary() };
        let place = place.clone();
        let install = install
            .inactive(self.blocked() || self.offers.command.is_none())
            .on_click(cx.listener(move |view, _, _, cx| {
                view.install_command(place.clone(), cx);
            }))
            .into_any_element();
        skip.into_iter().chain([install]).collect()
    }

    /// Skip: saves that the step was asked, and moves on.
    fn settle_command_step(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        self.run(
            async move { client.settle_command_line().await.map(|_| Effect::None) },
            cx,
        );
    }

    /// Installs the command, holding the frame on the step shown at `place`: once installed the
    /// step no longer applies until it is settled.
    fn install_command(&mut self, place: Place, cx: &mut Context<Self>) {
        if self.blocked() {
            return;
        }
        self.set_busy(true, cx);
        self.offers.install = Install::Running;
        self.held_step = Some(place);
        let client = self.client.clone();
        self.ask(
            async move {
                let status = client.install_command_line().await?;
                if landed(&status) {
                    client.settle_command_line().await?;
                }
                Ok(status)
            },
            |view, installed, cx| view.command_installed(installed, cx),
            cx,
        );
    }

    /// Shows what Install did. Installed, the step reads "skillbinder is ready" for a moment,
    /// then moves on; off the PATH, it waits for Continue. Cancelling the administrator prompt
    /// leaves it as it was.
    fn command_installed(
        &mut self,
        installed: AppResult<CommandLineStatus>,
        cx: &mut Context<Self>,
    ) {
        self.set_busy(false, cx);
        self.offers.install = Install::Idle;
        match installed {
            Ok(status) => {
                let state = status.state;
                self.offers.command = Some(status);
                match state {
                    CommandLineState::Installed => {
                        cx.spawn(async move |this, cx| {
                            cx.background_executor().timer(READY_FOR).await;
                            let _ = this.update(cx, |view, cx| {
                                view.held_step = None;
                                cx.notify();
                            });
                        })
                        .detach();
                    }
                    CommandLineState::NotOnPath => {}
                    _ => self.held_step = None,
                }
            }
            Err(error) => {
                self.offers.install = Install::Failed(error.message);
                self.held_step = None;
                self.reread_command(cx);
            }
        }
        cx.notify();
    }
}
