//! Settings › Updates (spec rules 18, 19, 24): the running version in tabular figures, the
//! channel as Kit's segmented control, the Check automatically switch, and one status line with
//! its action at the row's end. Every newer version is download-only in this slice. Copy always
//! names SkillBinder: "update" alone means a skill update.

#[cfg(test)]
mod tests;

use super::super::NativeView;
use super::super::library_dates::{from_unix, relative};
use chrono::Utc;
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{h_flex, switch::Switch, v_flex};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, px};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{Button, IconSize, KeyValueList, SectionHeader, Spinner, Text};
use skillbinder_proto::{AppUpdateSnapshot, AppUpdateState, Channel, CheckFailure};

const CHANNELS: [(Channel, &str); 2] = [(Channel::Stable, "Stable"), (Channel::Beta, "Beta")];
/// Why the notice and Settings offer Download in this slice, before the app can update itself.
pub(in crate::native_view) const DOWNLOAD_LINE: &str = "Download it from the release page.";

/// What the Status row's action does.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    None,
    /// Check now, disabled while a check runs.
    CheckNow {
        enabled: bool,
    },
    /// Download and What's new, both opening the release page.
    Download {
        notes_url: String,
    },
}

/// The Status row: its line, a muted line under it, whether a spinner leads it, and its action.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Status {
    line: String,
    detail: Option<&'static str>,
    spinner: bool,
    action: Action,
}

/// Rule 18's row for `update` at Unix time `now`.
fn status(update: &AppUpdateSnapshot, now: u64) -> Status {
    let line = |line: String, action| Status {
        line,
        detail: None,
        spinner: false,
        action,
    };
    let check_now = Action::CheckNow { enabled: true };
    match &update.state {
        AppUpdateState::Off => line("Updates are off in this build.".to_owned(), Action::None),
        AppUpdateState::Idle { last_failure: None } => {
            let checked = update.last_checked.and_then(from_unix);
            let text = match (checked, from_unix(now)) {
                (Some(checked), Some(now)) => {
                    format!("Up to date · checked {}", relative(checked, now))
                }
                _ => "Not checked yet".to_owned(),
            };
            line(text, check_now)
        }
        AppUpdateState::Idle {
            last_failure: Some(CheckFailure::Offline),
        } => line(
            "Couldn't check for updates. Check your connection.".to_owned(),
            check_now,
        ),
        AppUpdateState::Idle {
            last_failure: Some(CheckFailure::Failed),
        } => line("Couldn't check for updates.".to_owned(), check_now),
        AppUpdateState::Checking => Status {
            spinner: true,
            ..line("Checking…".to_owned(), Action::CheckNow { enabled: false })
        },
        AppUpdateState::DownloadOnly { version, notes_url } => Status {
            detail: Some(DOWNLOAD_LINE),
            ..line(
                format!("SkillBinder {version} is available."),
                Action::Download {
                    notes_url: notes_url.clone(),
                },
            )
        },
    }
}

/// The line under the channel while a beta build follows Stable (rule 19).
fn channel_line(update: &AppUpdateSnapshot) -> Option<String> {
    (update.channel == Channel::Stable && update.current.beta.is_some()).then(|| {
        format!(
            "You'll move to Stable at its first release after {}.",
            update.current
        )
    })
}

impl NativeView {
    /// The Updates header and its list.
    pub(super) fn updates_section(&self, cx: &mut Context<Self>) -> [AnyElement; 2] {
        let update = &self.snapshot.app_update;
        let now = u64::try_from(Utc::now().timestamp()).unwrap_or_default();
        let mut list = KeyValueList::new("settings-updates")
            .element(
                "Version",
                Text::labeled("settings-version", update.current.to_string()).tabular(),
            )
            .element("Channel", self.channel_control(update, cx));
        if update.state != AppUpdateState::Off {
            let auto_check = Switch::new("settings-auto-check")
                .checked(update.auto_check)
                .accessibility_label("Check automatically")
                .on_click(cx.listener(|view, on: &bool, _, cx| view.set_auto_check(*on, cx)));
            list = list.element("Check automatically", auto_check);
        }
        let list = list.element("Status", self.status_row(&status(update, now), cx));
        [
            SectionHeader::new("settings-updates-header", "Updates").into_any_element(),
            list.into_any_element(),
        ]
    }

    fn channel_control(&self, update: &AppUpdateSnapshot, cx: &mut Context<Self>) -> AnyElement {
        let chosen = CHANNELS
            .iter()
            .position(|(channel, _)| *channel == update.channel)
            .unwrap_or_default();
        let segments = CHANNELS.map(|(_, label)| {
            Tab::new()
                .aria_label(label)
                .cursor_pointer()
                .child(Text::new(label).level(text::LABEL_SM))
        });
        let segmented = TabBar::new("settings-channel")
            .segmented()
            .flex_none()
            .selected_index(chosen)
            .children(segments)
            .on_click(cx.listener(|view, index: &usize, _, cx| {
                if let Some(&(channel, _)) = CHANNELS.get(*index) {
                    view.set_update_channel(channel, cx);
                }
            }));
        v_flex()
            .items_start()
            .gap(px(space::SM))
            .child(segmented)
            .children(
                channel_line(update)
                    .map(|line| Text::labeled("settings-channel-line", line).muted()),
            )
            .into_any_element()
    }

    /// The status line, with a muted line under it, and its action at the row's end.
    fn status_row(&self, status: &Status, cx: &mut Context<Self>) -> AnyElement {
        let line = h_flex()
            .gap(px(space::SM))
            .children(status.spinner.then(|| Spinner::new(IconSize::Sm)))
            .child(Text::labeled("settings-update-status", status.line.clone()));
        let text = v_flex().gap(px(space::XS)).child(line).children(
            status
                .detail
                .map(|detail| Text::labeled("settings-update-detail", detail).muted()),
        );
        let actions = h_flex().gap(px(space::MD)).flex_none();
        let actions = match &status.action {
            Action::None => actions,
            Action::CheckNow { enabled } => actions.child(
                Button::new("settings-check-now")
                    .label("Check now")
                    .small()
                    .inactive(!enabled)
                    .on_click(cx.listener(|view, _, _, _| view.client.check_app_update())),
            ),
            Action::Download { notes_url } => {
                let (download, notes) = (notes_url.clone(), notes_url.clone());
                actions
                    .child(
                        Button::new("settings-download")
                            .label("Download")
                            .small()
                            .on_click(move |_, _, cx| cx.open_url(&download)),
                    )
                    .child(
                        Button::new("settings-whats-new")
                            .label("What's new")
                            .small()
                            .ghost()
                            .on_click(move |_, _, cx| cx.open_url(&notes)),
                    )
            }
        };
        h_flex()
            .justify_between()
            .items_center()
            .gap(px(space::LG))
            .child(text)
            .child(actions)
            .into_any_element()
    }

    fn set_update_channel(&mut self, channel: Channel, cx: &mut Context<Self>) {
        let client = self.client.clone();
        self.save_in_background(async move { client.set_update_channel(channel).await }, cx);
    }

    fn set_auto_check(&mut self, on: bool, cx: &mut Context<Self>) {
        let client = self.client.clone();
        self.save_in_background(async move { client.set_update_auto_check(on).await }, cx);
    }

    /// Runs `save` on the client's runtime; a failure shows in the error box.
    fn save_in_background(
        &mut self,
        save: impl Future<Output = skillbinder_proto::AppResult<()>> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        let saved = self.client.runtime().spawn(save);
        cx.spawn(async move |this, cx| {
            if let Ok(Err(error)) = saved.await {
                let _ = this.update(cx, |view, cx| {
                    view.error = Some(error);
                    cx.notify();
                });
            }
        })
        .detach();
    }
}
