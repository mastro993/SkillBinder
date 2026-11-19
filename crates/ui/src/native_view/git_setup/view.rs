//! The Git install flow's pieces, for the Git step and Settings' Git row: the status line, the
//! command box with Copy, and the Install Git and Download Git buttons.

use super::super::NativeView;
use super::{Action, GitButton, GitFacts, Status};
use gpui_kit::{AnyElement, Context, IntoElement};
use skillbinder_design::{Button, CodeBlock, Notice, NoticeSize};

impl NativeView {
    /// What the snapshot says about Git.
    pub(in crate::native_view) fn git_facts(&self) -> GitFacts<'_> {
        GitFacts::of(&self.snapshot.bootstrap)
    }

    /// The one status line, if any: the turning glyph while waiting, check-circle once Git is
    /// found, and alert-circle when winget failed or Git is too old.
    pub(in crate::native_view) fn git_status_line(&self, id: &'static str) -> Option<AnyElement> {
        let status = self.git_facts().status(self.git_setup.waiting)?;
        let line = Notice::new(id, status.text()).size(NoticeSize::Md);
        let line = match &status {
            Status::Waiting(_) => line.turning("refresh-04"),
            Status::Ready(_) => line.status(skillbinder_design::Status::Success),
            Status::Failed => line.status(skillbinder_design::Status::Danger),
            Status::TooOld(_) => line.glyph("alert-circle"),
        };
        Some(line.into_any_element())
    }

    /// The package manager's command on the muted surface, with Copy, which starts the wait.
    pub(in crate::native_view) fn git_command_box(
        &self,
        command: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let copied = cx.entity().downgrade();
        CodeBlock::new("git-command", command.to_owned())
            .on_copy(move |_, cx| {
                let _ = copied.update(cx, |view, cx| view.git_action(Action::Copy, cx));
            })
            .into_any_element()
    }

    /// The `button`, the primary if `primary`, unavailable once Git is found (T6).
    pub(in crate::native_view) fn git_button(
        &self,
        button: GitButton,
        primary: bool,
        found: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (id, label) = match button {
            GitButton::Install => ("git-install", "Install Git"),
            GitButton::Download => ("git-download", "Download Git"),
        };
        let action = button.action();
        let button = Button::new(id).label(label);
        let button = if primary { button.primary() } else { button };
        button
            .inactive(found || self.blocked())
            .on_click(cx.listener(move |view, _, _, cx| view.git_action(action, cx)))
            .into_any_element()
    }

    /// The small outline button a page's quiet "needs Git" line offers: Install Git, reading
    /// "Installing Git" while the installer runs, or Download Git where nothing installs it.
    pub(in crate::native_view) fn git_line_button(
        &self,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let installing = self.git_setup.waiting == Some(super::Waiting::Installer);
        let (action, label) = match (self.git_facts().actions().install, installing) {
            (true, true) => (Action::Install, "Installing Git"),
            (true, false) => (Action::Install, "Install Git"),
            (false, _) => (Action::Download, "Download Git"),
        };
        Button::new(id)
            .label(label)
            .small()
            .loading(installing)
            .inactive(installing || self.blocked())
            .on_click(cx.listener(move |view, _, _, cx| view.git_action(action, cx)))
            .into_any_element()
    }
}
