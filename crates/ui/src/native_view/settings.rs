//! Settings: your agents (design S1, S2), then the application's facts and the Git row (S3).

mod agents;

use super::NativeView;
use super::git_setup::GitButton;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div, px};
use skillbinder_design::tokens::space;
use skillbinder_design::{Button, KeyValueList, SectionHeader, Text};

impl NativeView {
    pub(super) fn render_settings(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        // A wait on Git resumes its check when Settings opens again.
        self.watch_git(cx);
        let bootstrap = &self.snapshot.bootstrap;
        let remote = self
            .snapshot
            .sync
            .remote
            .as_ref()
            .map_or("Local only", |remote| remote.url.as_str());
        let application = KeyValueList::new("settings-application")
            .mono("Managed library", bootstrap.library_path.clone())
            .element("Git", self.git_row(cx))
            .row(
                "Storage writable",
                if bootstrap.storage_writable {
                    "Yes"
                } else {
                    "No"
                },
            )
            .row("Remote", remote.to_owned());
        div()
            .flex()
            .flex_col()
            .gap(px(space::XL))
            .child(SectionHeader::new("settings-agents", "Agents"))
            .child(self.agents_section(cx))
            .child(SectionHeader::new(
                "settings-application-header",
                "Application",
            ))
            .child(application)
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
            return Text::labeled("settings-git", version.to_owned()).into_any_element();
        }
        let actions = facts.actions();
        let primary = if actions.install {
            Some(GitButton::Install)
        } else {
            actions.download_primary.then_some(GitButton::Download)
        };
        let value = facts.outdated.unwrap_or("Not installed");
        v_flex()
            .gap(px(space::SM))
            .child(
                h_flex()
                    .gap(px(space::MD))
                    .child(Text::labeled("settings-git", value.to_owned()))
                    .children(primary.map(|button| self.git_button(button, true, false, cx))),
            )
            .children(self.git_status_line("settings-git-line"))
            .children(
                actions
                    .command
                    .map(|command| self.git_command_box(command, cx)),
            )
            .into_any_element()
    }
}
