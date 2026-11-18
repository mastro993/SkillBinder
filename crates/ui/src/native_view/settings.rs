//! Settings: your agents (design S1, S2), then the application's facts and the Git row (S3).

mod agents;

use super::NativeView;
use super::git_setup::Action;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::{AnyElement, Context, FontWeight, IntoElement, ParentElement, Styled, div, px};
use skillbinder_design::Button;

/// A section's heading.
fn heading(text: &'static str) -> impl IntoElement {
    div().font_weight(FontWeight::SEMIBOLD).child(text)
}

impl NativeView {
    pub(super) fn render_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        // A wait on Git resumes its check when Settings opens again.
        self.watch_git(cx);
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(heading("Agents"))
            .child(self.agents_section(cx))
            .child(heading("Application"))
            .child(format!(
                "Managed library: {}",
                self.snapshot.bootstrap.library_path
            ))
            .child(self.git_row(cx))
            .child(format!(
                "Storage writable: {}",
                if self.snapshot.bootstrap.storage_writable {
                    "Yes"
                } else {
                    "No"
                }
            ))
            .child(format!(
                "Remote: {}",
                self.snapshot
                    .sync
                    .remote
                    .as_ref()
                    .map_or("Local only", |remote| remote.url.as_str())
            ))
            .child(
                Button::new("reveal-logs")
                    .label("Reveal logs")
                    .on_click(cx.listener(|view, _, _, cx| view.reveal_logs(cx))),
            )
    }

    /// Git's version, or "Not installed" with the step's primary action and its status line
    /// inline, and the package manager's command on Linux (S3).
    fn git_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let facts = self.git_facts();
        if let Some(version) = facts.found {
            return div().child(format!("Git: {version}")).into_any_element();
        }
        let actions = facts.actions();
        let primary = if actions.install {
            Some(Action::Install)
        } else {
            actions.download_primary.then_some(Action::Download)
        };
        let value = facts.outdated.unwrap_or("Not installed");
        v_flex()
            .gap(px(6.))
            .child(
                h_flex()
                    .gap(px(8.))
                    .child(format!("Git: {value}"))
                    .children(primary.map(|action| self.git_button(action, true, false, cx))),
            )
            .children(self.git_status_line("settings-git-line", cx))
            .children(
                actions
                    .command
                    .map(|command| self.git_command_box(command, cx)),
            )
            .into_any_element()
    }
}
