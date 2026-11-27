//! Not connected's setup column (Y10): what syncs, the Remote URL and Branch fields, and
//! Connect; without Git, the same column offers Install Git and Check again.

use super::super::git_setup::GitButton;
use super::super::{Effect, NativeView};
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputState;
use gpui_kit::component::{ActiveTheme, Sizable, v_flex};
use gpui_kit::{
    AnyElement, App, Context, Entity, InteractiveElement, IntoElement, ParentElement, Styled, div,
    px,
};
use skillbinder_design::sync::{SETUP_COLUMN, SETUP_TILE, SETUP_TOP};
use skillbinder_design::tokens::{radius, space, text};
use skillbinder_design::{Button, IconSize, Text, icon, icon_color};
use skillbinder_proto::RemoteConfig;

impl NativeView {
    /// The setup column: the remote to connect, and its branch.
    pub(super) fn sync_setup(&self, cx: &mut Context<Self>) -> AnyElement {
        let working = self.working();
        let connect = Button::new("sync-connect")
            .label("Connect")
            .primary()
            .loading(working)
            .inactive(self.blocked())
            .on_click(cx.listener(|view, _, _, cx| {
                let url = Self::input_text(&view.remote_url, cx);
                let branch = Self::input_text(&view.remote_branch, cx);
                let client = view.client.clone();
                view.run(
                    async move {
                        client
                            .connect_remote(RemoteConfig { url, branch })
                            .await
                            .map(|_| Effect::None)
                    },
                    cx,
                );
            }));
        column(
            "Sync your library between computers",
            "Connect an empty Git repository, or the one your other computer uses. Skills and \
             their records sync; presets, folders, and workspaces stay on each device.",
            cx,
        )
        .child(field("Remote URL", &self.remote_url, working))
        .child(field("Branch", &self.remote_branch, working))
        .child(connect)
        .into_any_element()
    }

    /// The column without Git: Install Git, or Download Git where nothing installs it, and
    /// Check again, which looks for Git now.
    pub(super) fn sync_needs_git(&self, cx: &mut Context<Self>) -> AnyElement {
        let get = if self.git_facts().actions().install {
            GitButton::Install
        } else {
            GitButton::Download
        };
        let check = Button::new("sync-check-git")
            .label("Check again")
            .on_click(cx.listener(|view, _, _, _| view.client.check_git()));
        column(
            "Sync needs Git",
            "Git keeps your library the same on all your computers. Sync and skill updates need \
             Git 2.39 or newer.",
            cx,
        )
        .children(self.git_status_line("sync-git-status"))
        .child(
            div()
                .flex()
                .gap(px(space::MD))
                .child(self.git_button(get, true, false, cx))
                .child(check),
        )
        .into_any_element()
    }
}

/// The setup column's git-branch tile, `title`, and `line`, for the fields and buttons under them.
fn column(title: &'static str, line: &'static str, cx: &App) -> gpui_kit::Stateful<gpui_kit::Div> {
    let tile = div()
        .size(px(SETUP_TILE))
        .rounded(px(radius::LG))
        .bg(cx.theme().muted)
        .flex()
        .items_center()
        .justify_center()
        .child(
            icon("git-branch")
                .with_size(IconSize::Lg)
                .text_color(icon_color(cx.theme())),
        );
    v_flex()
        .id("sync-setup")
        .w_full()
        .max_w(px(SETUP_COLUMN))
        .mx_auto()
        .mt(px(SETUP_TOP))
        .items_start()
        .gap(px(space::LG))
        .child(tile)
        .child(Text::labeled("sync-setup-title", title).level(text::TITLE_MD))
        .child(
            Text::labeled("sync-setup-line", line)
                .level(text::BODY_SM)
                .muted(),
        )
}

/// A field labeled `label` over its input, as wide as the column.
fn field(label: &'static str, input: &Entity<InputState>, disabled: bool) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap(px(space::XS))
        .child(Text::new(label).level(text::LABEL_SM))
        .child(Input::new(input).aria_label(label).disabled(disabled))
}
