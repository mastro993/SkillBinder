//! The Git install flow's pieces, for the Git step and Settings' Git row: the status line, the
//! command box with Copy, and the Install Git and Download Git buttons.

use super::super::NativeView;
use super::super::controls::{Inactive, app_icon, text_button};
use super::super::import_dialog::alert_glyph;
use super::{Action, GitFacts, Status};
use gpui_kit::component::{ActiveTheme, Size, button::ButtonVariants, h_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};
use skillbinder_design::tokens::color;

impl NativeView {
    /// What the snapshot says about Git.
    pub(in crate::native_view) fn git_facts(&self) -> GitFacts<'_> {
        GitFacts::of(&self.snapshot.bootstrap)
    }

    /// The one status line, if any: the loading glyph while waiting, check-circle once Git is
    /// found, and alert-circle when winget failed or Git is too old. `spin` keys the spinner.
    pub(in crate::native_view) fn git_status_line(
        &self,
        id: &'static str,
        spin: &'static str,
        cx: &Context<Self>,
    ) -> Option<AnyElement> {
        let status = self.git_facts().status(self.git_setup.waiting)?;
        let theme = cx.theme();
        let (glyph, color) = match &status {
            Status::Waiting(_) => (
                Self::spinner(spin).into_any_element(),
                theme.muted_foreground,
            ),
            Status::Ready(_) => (
                app_icon("check-circle")
                    .size(px(14.))
                    .text_color(color::SUCCESS_TEXT.of(theme))
                    .into_any_element(),
                color::SUCCESS_TEXT.of(theme),
            ),
            Status::Failed => (
                alert_glyph(color::DANGER_TEXT.of(theme)),
                color::DANGER_TEXT.of(theme),
            ),
            Status::TooOld(_) => (alert_glyph(theme.muted_foreground), theme.muted_foreground),
        };
        Some(
            h_flex()
                .id(id)
                .items_start()
                .gap(px(8.))
                .text_color(color)
                .child(div().flex_none().mt(px(2.)).child(glyph))
                .child(div().min_w_0().flex_1().child(status.text()))
                .into_any_element(),
        )
    }

    /// The package manager's command in monospace on the muted surface, with Copy.
    pub(in crate::native_view) fn git_command_box(
        &self,
        command: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        h_flex()
            .id("git-command")
            .gap(px(8.))
            .pl(px(10.))
            .pr(px(4.))
            .py(px(4.))
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.border)
            .bg(theme.muted)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .font_family(theme.mono_font_family.clone())
                    .text_size(px(12.))
                    .child(command.to_owned()),
            )
            .child(self.git_button(Action::Copy, false, false, cx))
            .into_any_element()
    }

    /// The button for `action`, the primary if `primary`, unavailable once Git is found (T6).
    pub(in crate::native_view) fn git_button(
        &self,
        action: Action,
        primary: bool,
        found: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (id, label, size) = match action {
            Action::Install => ("git-install", "Install Git", Size::Medium),
            Action::Download => ("git-download", "Download Git", Size::Medium),
            Action::Copy => ("git-copy", "Copy", Size::Small),
        };
        let button = text_button(id, label, size);
        let button = if primary {
            button.primary()
        } else {
            button.outline()
        };
        button
            .inactive(found || self.blocked())
            .on_click(cx.listener(move |view, _, _, cx| view.git_action(action, cx)))
            .into_any_element()
    }
}
