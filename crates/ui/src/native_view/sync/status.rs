//! The status card (Y2): a tinted glyph tile, the state's title and line, its one next action,
//! Check now, and the ⋯ menu of the other remote actions (Y4); and the remote facts under it
//! (Y3).

use super::super::sidebar_presets::menu_action;
use super::super::{DialogState, NativeView};
use super::copy::{self, Tint};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, ClickEvent, ClipboardItem, Context, InteractiveElement, IntoElement,
    MouseButton, MouseDownEvent, ParentElement, Pixels, Point, Styled, Window, div, px,
};
use skillbinder_design::sync::STATUS_TILE;
use skillbinder_design::tokens::{color, radius, space, text};
use skillbinder_design::{
    Button, IconSize, Level, MenuItem, OpenMenu, Surface, Text, icon, icon_color,
};
use skillbinder_proto::{RemoteConfig, SyncAction, SyncState};

const MENU_OWNER: &str = "sync-more";

impl NativeView {
    /// The status card, while the state has one.
    pub(super) fn sync_status(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let status = copy::status(&self.snapshot.sync, chrono::Utc::now())?;
        let running = self.snapshot.syncing;
        let inactive = self.blocked() || running.is_some();
        let main = status.main.map(|action| {
            let label = if running == Some(action) {
                copy::running(action)
            } else {
                copy::main_label(action)
            };
            let button = Button::new("sync-main")
                .label(label)
                .loading(running == Some(action))
                .inactive(inactive)
                .on_click(cx.listener(move |view, _, _, cx| view.synchronize(action, cx)));
            // Try again is the outline button; every other next action is the primary.
            match action {
                SyncAction::Refresh => button,
                _ => button.primary(),
            }
        });
        let checking = running == Some(SyncAction::Refresh);
        let check_now = (status.main != Some(SyncAction::Refresh)).then(|| {
            Button::new("sync-check")
                .label(if checking {
                    copy::running(SyncAction::Refresh)
                } else {
                    "Check now"
                })
                .icon(icon("refresh"))
                .loading(checking)
                .tooltip("Look for changes on the remote")
                .inactive(inactive)
                .on_click(cx.listener(|view, _, _, cx| view.synchronize(SyncAction::Refresh, cx)))
        });
        let more = Button::new(MENU_OWNER)
            .ghost()
            .icon(icon("more"))
            .accessibility_label("More sync actions")
            .tooltip("More")
            .selected(self.menu.as_ref().is_some_and(|m| m.is_for(MENU_OWNER)))
            .inactive(inactive)
            .on_click(cx.listener(|view, event: &ClickEvent, window, cx| {
                if event.is_keyboard() {
                    view.open_sync_menu(event.position(), window, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, event: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    view.open_sync_menu(event.position, window, cx);
                }),
            );
        let card = Surface::new(Level::Flat).radius(radius::LG).apply(
            h_flex()
                .id("sync-status")
                .items_center()
                .gap(px(space::LG))
                .p(px(space::LG))
                .pl(px(space::XL)),
            cx.theme(),
        );
        Some(
            card.child(tile(status.glyph, status.tint, cx))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(space::XXS))
                        .child(Text::labeled("sync-title", status.title).level(text::TITLE_MD))
                        .child(
                            div().truncate().child(
                                Text::labeled("sync-line", status.line)
                                    .level(text::BODY_SM)
                                    .muted(),
                            ),
                        ),
                )
                .child(
                    h_flex()
                        .flex_none()
                        .gap(px(space::SM))
                        .children(main)
                        .children(check_now)
                        .child(more),
                )
                .into_any_element(),
        )
    }

    /// Opens the ⋯ menu at `position`.
    fn open_sync_menu(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let items = self.sync_menu(cx);
        self.menu = Some(OpenMenu::new(
            MENU_OWNER,
            items,
            position,
            self.focus.clone(),
            window,
            cx,
            |view: &mut NativeView| &mut view.menu,
        ));
    }

    /// Pull only and Push only; the remote's URL, web page, and branch, and this device's name;
    /// and Disconnect… (Y4).
    fn sync_menu(&self, cx: &mut Context<Self>) -> Vec<MenuItem> {
        let view = cx.entity().downgrade();
        let sync = &self.snapshot.sync;
        let Some(remote) = sync.remote.clone() else {
            return Vec::new();
        };
        let changed_here = matches!(sync.state, SyncState::NeedsPush | SyncState::NeedsSync);
        let changed_there = matches!(sync.state, SyncState::NeedsPull | SyncState::NeedsSync);
        let run = |action| {
            menu_action(view.clone(), action, |view, action, _, cx| {
                view.synchronize(*action, cx);
            })
        };
        let mut pull = MenuItem::new("sync-pull-only", "Pull only")
            .icon("download")
            .disabled(sync.state != SyncState::NeedsPull)
            .on_select(run(SyncAction::Pull));
        if changed_here {
            pull = pull.tooltip("Push or Sync first: this device has changes");
        }
        let mut push = MenuItem::new("sync-push-only", "Push only")
            .icon("upload")
            .disabled(sync.state != SyncState::NeedsPush)
            .on_select(run(SyncAction::Push));
        if changed_there {
            push = push.tooltip("Pull or Sync first: the remote has changes");
        }
        let url = remote.url.clone();
        let copy = MenuItem::new("sync-copy-url", "Copy remote URL")
            .icon("copy")
            .on_select(menu_action(view.clone(), url, |view, url, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(url.clone()));
                view.show_toast("Copied the remote URL", cx);
            }));
        let browse = remote.web_url().map(|web| {
            MenuItem::new("sync-open-web", "Open in browser")
                .icon("globe")
                .on_select(move |_: &mut Window, cx: &mut App| cx.open_url(&web))
        });
        let branch = MenuItem::new("sync-change-branch", "Change branch…")
            .icon("git-branch")
            .on_select(menu_action(
                view.clone(),
                remote,
                |view, remote, window, cx| {
                    view.remote_branch.update(cx, |input, cx| {
                        input.set_value(remote.branch.clone(), window, cx);
                    });
                    view.show_dialog(DialogState::ChangeBranch);
                    cx.notify();
                },
            ));
        let device = self.snapshot.sync.device.clone();
        let rename = MenuItem::new("sync-rename-device", "Rename this device…")
            .icon("laptop")
            .on_select(menu_action(
                view.clone(),
                device,
                |view, device, window, cx| {
                    view.device_name.update(cx, |input, cx| {
                        input.set_value(device.clone(), window, cx);
                    });
                    view.show_dialog(DialogState::RenameDevice);
                    cx.notify();
                },
            ));
        let disconnect = MenuItem::new("sync-disconnect", "Disconnect…")
            .icon("unplug")
            .danger()
            .on_select(menu_action(view, (), |view, (), _, cx| {
                view.show_dialog(DialogState::Disconnect);
                cx.notify();
            }));
        [pull, push, MenuItem::separator(), copy]
            .into_iter()
            .chain(browse)
            .chain([branch, rename, MenuItem::separator(), disconnect])
            .collect()
    }
}

/// The glyph tile, tinted by the state.
fn tile(glyph: &'static str, tint: Tint, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let (fill, ink) = match tint {
        Tint::Accent => (
            color::ACCENT_SOFT.of(theme),
            color::ACCENT_SOFT_FOREGROUND.of(theme),
        ),
        Tint::Success => (theme.muted, color::SUCCESS_TEXT.of(theme)),
        Tint::Danger => (theme.muted, color::DANGER_TEXT.of(theme)),
    };
    div()
        .flex_none()
        .size(px(STATUS_TILE))
        .rounded(px(radius::MD))
        .bg(fill)
        .flex()
        .items_center()
        .justify_center()
        .child(icon(glyph).with_size(IconSize::Lg).text_color(ink))
}

/// The remote's facts under the status: where it is, which branch syncs, and this device's name
/// (Y3).
pub(super) fn facts(remote: &RemoteConfig, device: &str, cx: &App) -> impl IntoElement {
    let address = remote.address();
    let host = if address.starts_with("github.com/") {
        "github"
    } else {
        "globe"
    };
    let fact = |id: &'static str, glyph: &'static str, value: String| {
        h_flex()
            .gap(px(space::XS))
            .child(
                icon(glyph)
                    .with_size(IconSize::Xs)
                    .text_color(icon_color(cx.theme())),
            )
            .child(Text::labeled(id, value).level(text::CAPTION).muted())
    };
    h_flex()
        .id("sync-facts")
        .flex_wrap()
        .gap(px(space::LG))
        .px(px(space::XXS))
        .child(fact("sync-remote", host, address))
        .child(fact("sync-branch", "git-branch", remote.branch.clone()))
        .child(fact("sync-device", "laptop", device.to_owned()))
}
