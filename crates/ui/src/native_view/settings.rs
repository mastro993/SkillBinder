use super::NativeView;
use super::controls::button;
use super::git_setup::Action;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div, px};

impl NativeView {
    pub(super) fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child("Application")
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
                button("reveal-logs")
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
            .children(self.git_status_line("settings-git-line", "settings-git-spin", cx))
            .children(
                actions
                    .command
                    .map(|command| self.git_command_box(command, cx)),
            )
            .into_any_element()
    }
}
