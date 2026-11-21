//! The app update notice (spec rules 9–11, 13, 14): after onboarding, a newer SkillBinder gets one
//! Kit notification at the window's bottom-right, with its own id and no auto-hide, that stacks
//! with the new agent notice. A ready update reads "SkillBinder <v> is ready" with Dismiss and the
//! primary Restart, which waits while long work runs; a download-only one reads "SkillBinder <v>
//! is available" with the reason and the primary Download, which opens the release page. Dismiss,
//! Download, or closing it hides it until the next launch. It waits while the progress strip is
//! up and comes back once it ends. The first launch of a newer version shows the "Updated to"
//! toast once.

use super::NativeView;
use super::toasts::ToastAction;
use gpui_kit::component::{ActiveTheme, WindowExt, h_flex, notification::Notification, v_flex};
use gpui_kit::{
    Anchor, AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, WeakEntity, Window, div, px,
};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{Button, Text, tooltip as tip};
use skillbinder_proto::{AppUpdateState, AppVersion, CannotInstall, InstallState, SyncAction};

/// The notice's id, so a newer version replaces the shown one.
struct AppUpdateNotice;

/// The line under a ready update's title.
pub(super) const READY_LINE: &str = "Restart now, or it installs when you quit.";

/// Why a download-only update is not installed in place (rule 13).
pub(super) fn reason_line(reason: CannotInstall) -> &'static str {
    match reason {
        CannotInstall::OutsideApplications => {
            "Move SkillBinder to Applications to update automatically."
        }
        CannotInstall::NotWritable => "SkillBinder can't write to its folder.",
        CannotInstall::NotInstalled => "Download it from the release page.",
    }
}

/// What the notice offers.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Offer {
    Ready(AppVersion),
    DownloadOnly {
        version: AppVersion,
        notes_url: String,
        reason: CannotInstall,
    },
}

/// Which notice shows.
#[derive(Default)]
pub(super) struct Notice {
    /// The offer of the notice shown now.
    shown: Option<Offer>,
    /// Hidden until the next launch.
    dismissed: bool,
    /// The "Updated to" toast showed this launch.
    announced: bool,
}

/// What keeps Restart unavailable until it finishes (rule 10).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RestartBlocker {
    Sync,
    Push,
    UpdateAll,
    Install,
    Change,
}

impl RestartBlocker {
    pub(super) fn tooltip(self) -> &'static str {
        match self {
            Self::Sync => "Available when Sync finishes",
            Self::Push => "Available when Push finishes",
            Self::UpdateAll => "Available when Update all finishes",
            Self::Install => "Available when installing finishes",
            Self::Change => "Available when the change finishes",
        }
    }
}

impl NativeView {
    /// Shows the notice for a newer version, replaces it when the offer changes, and takes it
    /// down while onboarding or the progress strip runs, once dismissed, or once none is offered.
    pub(super) fn sync_app_update_notice(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let finished = self.snapshot.bootstrap.onboarding.finished;
        self.announce_update(finished, cx);
        let shows = finished && self.strip.is_none() && !self.app_update_notice.dismissed;
        let offer = match self.snapshot.app_update.state.clone() {
            AppUpdateState::Ready { version, .. } if shows => Some(Offer::Ready(version)),
            AppUpdateState::DownloadOnly {
                version,
                notes_url,
                reason,
            } if shows => Some(Offer::DownloadOnly {
                version,
                notes_url,
                reason,
            }),
            _ => None,
        };
        if self.app_update_notice.shown == offer {
            return;
        }
        self.app_update_notice.shown = offer.clone();
        match offer {
            Some(offer) => {
                let notice = self.app_update_notification(offer, cx);
                window.push_notification(notice, cx);
            }
            None => window.remove_notification::<AppUpdateNotice>(cx),
        }
    }

    /// The first launch of a newer version says so once, after onboarding (rule 14).
    fn announce_update(&mut self, finished: bool, cx: &mut Context<Self>) {
        let notes = self.snapshot.app_update.updated.clone();
        if let Some(notes) = notes.filter(|_| finished && !self.app_update_notice.announced) {
            self.app_update_notice.announced = true;
            let message = format!(
                "Updated to SkillBinder {}",
                self.snapshot.app_update.current
            );
            self.toast
                .offer(message.into(), Some(ToastAction::ReleaseNotes(notes)), cx);
        }
    }

    /// What keeps Restart unavailable: Sync, Push, Update all, an install, or a blocking change.
    pub(super) fn restart_blocker(&self) -> Option<RestartBlocker> {
        let explore = &self.snapshot.explore;
        let installing = !explore.install_all.is_empty()
            || explore
                .installs
                .values()
                .any(|install| *install == InstallState::Installing);
        match self.syncing {
            Some(SyncAction::Push) => Some(RestartBlocker::Push),
            Some(_) => Some(RestartBlocker::Sync),
            None if self.updating.is_some() => Some(RestartBlocker::UpdateAll),
            None if installing => Some(RestartBlocker::Install),
            None if self.blocked() || self.snapshot.activation.is_some() => {
                Some(RestartBlocker::Change)
            }
            None => None,
        }
    }

    /// Restart, `id`, primary and small, for `view`, this view: it quits, the update installs,
    /// and the app opens again. While long work runs it is unavailable, and its tooltip says until
    /// when.
    pub(super) fn restart_button(&self, id: &'static str, view: WeakEntity<Self>) -> AnyElement {
        let button = Button::new(id).label("Restart").small().primary();
        if let Some(blocker) = self.restart_blocker() {
            return div()
                .id(format!("{id}-blocked"))
                .tooltip(tip(blocker.tooltip()))
                .child(button.inactive(true))
                .into_any_element();
        }
        button
            .on_click(move |_, _, cx| {
                let _ = view.update(cx, |view, cx| view.restart(cx));
            })
            .into_any_element()
    }

    fn restart(&mut self, cx: &mut Context<Self>) {
        if self.restart_blocker().is_some() {
            return;
        }
        match self.client.request_restart() {
            Ok(()) => cx.quit(),
            Err(error) => {
                self.error = Some(error);
                cx.notify();
            }
        }
    }

    fn app_update_notification(&self, offer: Offer, cx: &mut Context<Self>) -> Notification {
        let view = cx.entity().downgrade();
        let closed = view.clone();
        let shown = offer.clone();
        Notification::new()
            .id::<AppUpdateNotice>()
            .autohide(false)
            .placement(Anchor::BottomRight)
            .content(move |_, _, cx| {
                let (title, line) = match &offer {
                    Offer::Ready(version) => {
                        (format!("SkillBinder {version} is ready"), READY_LINE)
                    }
                    Offer::DownloadOnly {
                        version, reason, ..
                    } => (
                        format!("SkillBinder {version} is available"),
                        reason_line(*reason),
                    ),
                };
                let dismiss = view.clone();
                let primary = match &offer {
                    Offer::Ready(_) => view.upgrade().map(|entity| {
                        entity
                            .read(cx)
                            .restart_button("app-update-notice-restart", view.clone())
                    }),
                    Offer::DownloadOnly { notes_url, .. } => {
                        let (view, url) = (view.clone(), notes_url.clone());
                        Some(
                            Button::new("app-update-notice-download")
                                .label("Download")
                                .small()
                                .primary()
                                .on_click(cx.listener(
                                    move |note: &mut Notification, _, window, cx| {
                                        cx.open_url(&url);
                                        let _ = view.update(cx, |view, _| {
                                            view.app_update_notice.dismissed = true;
                                        });
                                        note.dismiss(window, cx);
                                    },
                                ))
                                .into_any_element(),
                        )
                    }
                };
                v_flex()
                    .gap(px(space::MD))
                    .child(
                        v_flex()
                            .child(
                                Text::labeled("app-update-notice-title", title)
                                    .level(text::TITLE_SM),
                            )
                            .child(div().text_color(cx.theme().muted_foreground).child(line)),
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
                            .children(primary),
                    )
                    .into_any_element()
            })
            .on_close(move |_, cx| {
                let _ = closed.update(cx, |view, _| view.close_app_update_notice(&shown));
            })
    }

    /// The notice for `offer` closed: unless it was replaced, it stays hidden until relaunch.
    fn close_app_update_notice(&mut self, offer: &Offer) {
        if self.app_update_notice.shown.as_ref() == Some(offer) {
            self.app_update_notice.dismissed = true;
        }
    }
}
