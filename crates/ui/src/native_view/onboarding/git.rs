//! The step shown while no usable Git is found (design T1 to T7, P2): the lead and one status
//! line, the package manager's command with Copy on Linux, then Skip at the left and Download Git
//! and Install Git at the right. Once Git is found it reads "Git <version> is ready" and moves on
//! by itself.

use super::super::git_setup::GitButton;
use super::super::{Effect, NativeView};
use super::StepView;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{Context, IntoElement, ParentElement, Styled, div};
use skillbinder_design::Button;

const LEAD: &str = "Git syncs your library between computers and updates skills from their \
                    sources. Everything else works without it, and you can install it later in \
                    Settings.";

impl NativeView {
    /// The step; once skipped and gone back to, it no longer offers Skip, and Continue is the
    /// primary.
    pub(super) fn git_step(&mut self, skipped: bool, cx: &mut Context<Self>) -> StepView {
        self.watch_git(cx);
        let facts = self.git_facts();
        let actions = facts.actions();
        let found = facts.found.is_some();
        let title = facts.title();
        let skip = (!skipped).then(|| {
            Button::new("onboarding-skip-git")
                .label("Skip")
                .ghost()
                .inactive(found || self.blocked())
                .on_click(cx.listener(|view, _, _, cx| {
                    // Skipped, nothing waits on Git until Settings installs it.
                    view.git_setup.waiting = None;
                    let client = view.client.clone();
                    view.run(
                        async move { client.skip_git().await.map(|_| Effect::None) },
                        cx,
                    );
                }))
                .into_any_element()
        });
        let mut body = vec![
            div()
                .text_color(cx.theme().muted_foreground)
                .child(LEAD)
                .into_any_element(),
        ];
        body.extend(self.git_status_line("onboarding-git-line"));
        body.extend(
            actions
                .command
                .map(|command| self.git_command_box(command, cx)),
        );
        let primary = !skipped;
        let mut buttons = vec![self.git_button(
            GitButton::Download,
            primary && actions.download_primary,
            found,
            cx,
        )];
        if actions.install {
            buttons.push(self.git_button(GitButton::Install, primary, found, cx));
        }
        StepView {
            title,
            wide: false,
            body,
            leading: skip,
            actions: buttons,
        }
    }
}
