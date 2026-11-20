//! The app update notice (spec rules 9, 13): after onboarding, a newer SkillBinder gets one Kit
//! notification at the window's bottom-right, with its own id and no auto-hide, that stacks with
//! the new agent notice. In this slice it is download-only: "SkillBinder <v> is available" with
//! Dismiss and the primary Download, which opens the release page. Dismiss, Download, or closing
//! it hides it until the next launch. It waits while the progress strip is up and comes back once
//! it ends.

use super::NativeView;
use super::settings::DOWNLOAD_LINE;
use gpui_kit::component::{ActiveTheme, WindowExt, h_flex, notification::Notification, v_flex};
use gpui_kit::{Anchor, Context, IntoElement, ParentElement, Styled, Window, div, px};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{Button, Text};
use skillbinder_proto::{AppUpdateState, AppVersion};

/// The notice's id, so a newer version replaces the shown one.
struct AppUpdateNotice;

/// Which version the notice offers.
#[derive(Default)]
pub(super) struct Notice {
    /// The version of the notice shown now.
    shown: Option<AppVersion>,
    /// Hidden until the next launch.
    dismissed: bool,
}

impl NativeView {
    /// Shows the notice for a newer version, replaces it when the version changes, and takes it
    /// down while onboarding or the progress strip runs, once dismissed, or once none is offered.
    pub(super) fn sync_app_update_notice(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ready = self.snapshot.bootstrap.onboarding.finished
            && self.strip.is_none()
            && !self.app_update_notice.dismissed;
        let offer = match &self.snapshot.app_update.state {
            AppUpdateState::DownloadOnly { version, notes_url } if ready => {
                Some((*version, notes_url.clone()))
            }
            _ => None,
        };
        let version = offer.as_ref().map(|(version, _)| *version);
        if self.app_update_notice.shown == version {
            return;
        }
        self.app_update_notice.shown = version;
        match offer {
            Some((version, notes_url)) => {
                let notice = self.app_update_notification(version, notes_url, cx);
                window.push_notification(notice, cx);
            }
            None => window.remove_notification::<AppUpdateNotice>(cx),
        }
    }

    fn app_update_notification(
        &self,
        version: AppVersion,
        notes_url: String,
        cx: &mut Context<Self>,
    ) -> Notification {
        let view = cx.entity().downgrade();
        let closed = view.clone();
        Notification::new()
            .id::<AppUpdateNotice>()
            .autohide(false)
            .placement(Anchor::BottomRight)
            .content(move |_, _, cx| {
                let dismiss = view.clone();
                let download = (view.clone(), notes_url.clone());
                v_flex()
                    .gap(px(space::MD))
                    .child(
                        v_flex()
                            .child(
                                Text::labeled(
                                    "app-update-notice-title",
                                    format!("SkillBinder {version} is available"),
                                )
                                .level(text::TITLE_SM),
                            )
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(DOWNLOAD_LINE),
                            ),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap(px(space::MD))
                            .child(
                                Button::new("app-update-notice-dismiss")
                                    .label("Dismiss")
                                    .small()
                                    .on_click(cx.listener(
                                        move |note: &mut Notification, _, window, cx| {
                                            let _ = dismiss.update(cx, |view, _| {
                                                view.app_update_notice.dismissed = true;
                                            });
                                            note.dismiss(window, cx);
                                        },
                                    )),
                            )
                            .child(
                                Button::new("app-update-notice-download")
                                    .label("Download")
                                    .small()
                                    .primary()
                                    .on_click(cx.listener(
                                        move |note: &mut Notification, _, window, cx| {
                                            let (view, url) = &download;
                                            cx.open_url(url);
                                            let _ = view.update(cx, |view, _| {
                                                view.app_update_notice.dismissed = true;
                                            });
                                            note.dismiss(window, cx);
                                        },
                                    )),
                            ),
                    )
                    .into_any_element()
            })
            .on_close(move |_, cx| {
                let _ = closed.update(cx, |view, _| view.close_app_update_notice(version));
            })
    }

    /// The notice for `version` closed: unless it was replaced, it stays hidden until relaunch.
    fn close_app_update_notice(&mut self, version: AppVersion) {
        if self.app_update_notice.shown == Some(version) {
            self.app_update_notice.dismissed = true;
        }
    }
}
