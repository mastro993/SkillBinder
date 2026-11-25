//! The one seam for every toast and notice: nothing else builds or pushes Kit's notifications,
//! so they all stack at the window's bottom-right corner.
//!
//! The toast that confirms a change has the check-circle glyph or, after a removal, its Undo
//! button; an install offers Show in Library beside its glyph, and an app update Release notes.
//! One saying what failed has no glyph, and may have a title over its message. A newer one
//! replaces the shown one, and Kit closes it after its time, which the pointer pauses while it
//! rests on it. A change ends from a timer that has no window at hand, so the toast waits for the
//! next frame, which pushes it. A notice waits for an answer: [`notice`] builds it with its own
//! id and no auto-hide, and [`push_notice`] shows it.

use super::NativeView;
use super::undo::Undo;
use gpui_kit::component::{WindowExt, notification::Notification};
use gpui_kit::{
    Anchor, App, Context, IntoElement, ParentElement, SharedString, Text, Window, div,
    prelude::FluentBuilder,
};
use skillbinder_design::tokens::text;
use skillbinder_design::{Button, icon};
use skillbinder_proto::{Repository, SkillId};

/// Where every toast and notice shows: one stack at the window's bottom-right corner.
const PLACEMENT: Anchor = Anchor::BottomRight;

/// The id every change's toast shares, so a newer one replaces the shown one.
struct ChangeToast;

/// A notice with id `T`, which a newer one with that id replaces: Kit's notification in the
/// toasts' stack, with no auto-hide, for the caller's content and answers.
pub(super) fn notice<T: 'static>() -> Notification {
    Notification::new()
        .id::<T>()
        .autohide(false)
        .placement(PLACEMENT)
}

/// Shows `notice`, which [`notice`] built, in the window's stack.
pub(super) fn push_notice(window: &mut Window, notice: Notification, cx: &mut App) {
    window.push_notification(notice, cx);
}

/// What a toast offers besides closing.
pub(super) enum ToastAction {
    /// Restores what a removal took away.
    Undo(Undo),
    /// Opens Library on the skill.
    ShowInLibrary(SkillId),
    /// Adds a removed repository back.
    RestoreRepository(Repository),
    /// Opens a SkillBinder release's page.
    ReleaseNotes(String),
}

/// Whether a toast confirms a change or says what failed, under an optional title.
enum Tone {
    Done,
    Failed(Option<SharedString>),
}

/// A toast waiting for the next frame, where the window it shows in is at hand, with the action
/// it offers, if any, and its tone.
#[derive(Default)]
pub(super) struct PendingToast(Option<(SharedString, Option<ToastAction>, Tone)>);

impl PendingToast {
    /// Shows `message` from the next frame of the view `cx` draws on, in place of any shown toast.
    pub(super) fn show<V: 'static>(&mut self, message: SharedString, cx: &mut Context<V>) {
        self.offer(message, None, cx);
    }

    /// Shows `message` offering `action`, as [`Self::show`] does.
    pub(super) fn offer<V: 'static>(
        &mut self,
        message: SharedString,
        action: Option<ToastAction>,
        cx: &mut Context<V>,
    ) {
        self.0 = Some((message, action, Tone::Done));
        cx.notify();
    }

    /// Shows `message`, saying what failed, as [`Self::show`] does but with no glyph.
    pub(super) fn fail<V: 'static>(&mut self, message: SharedString, cx: &mut Context<V>) {
        self.0 = Some((message, None, Tone::Failed(None)));
        cx.notify();
    }

    /// Shows `message` under `title`, saying what failed, as [`Self::fail`] does.
    pub(super) fn fail_titled<V: 'static>(
        &mut self,
        title: SharedString,
        message: SharedString,
        cx: &mut Context<V>,
    ) {
        self.0 = Some((message, None, Tone::Failed(Some(title))));
        cx.notify();
    }

    /// The waiting toast as Kit's notification, with its message and action, once. A toast
    /// offering Undo, or reporting a failure, has no glyph.
    pub(super) fn take(&mut self) -> Option<(Notification, SharedString, Option<ToastAction>)> {
        let (message, action, tone) = self.0.take()?;
        let text = message.clone();
        let (glyph, title) = match tone {
            Tone::Done => (
                !matches!(
                    action,
                    Some(ToastAction::Undo(_) | ToastAction::RestoreRepository(_))
                ),
                None,
            ),
            Tone::Failed(title) => (false, title),
        };
        // Kit's message is plain text; content keeps the message a label holding its text.
        let toast = Notification::new()
            .id::<ChangeToast>()
            .when(glyph, |toast| toast.icon(icon("check-circle")))
            .when_some(title, |toast, title| toast.title(title))
            .content(move |_, _, _| {
                text::BODY_MD
                    .apply(div())
                    .child(Text::new("toast-message".into(), text.clone()))
                    .into_any_element()
            })
            .placement(PLACEMENT);
        Some((toast, message, action))
    }
}

impl NativeView {
    /// Shows `message` from the next frame on, in place of any shown toast.
    pub(super) fn show_toast(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast.show(message.into(), cx);
    }

    /// Pushes the toast waiting for a frame, if any, onto the window's notifications.
    pub(super) fn push_toast(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((toast, message, action)) = self.toast.take() else {
            return;
        };
        let toast = match action {
            Some(ToastAction::Undo(undo)) => self.with_undo(toast, undo, cx),
            Some(ToastAction::ShowInLibrary(id)) => self.with_show_in_library(toast, id, cx),
            Some(ToastAction::RestoreRepository(repository)) => {
                self.with_restore(toast, repository)
            }
            Some(ToastAction::ReleaseNotes(url)) => with_release_notes(toast, url),
            None => toast,
        };
        let toast = self.hold_toast(toast, message, cx);
        window.push_notification(toast, cx);
    }
}

impl NativeView {
    /// `toast` with Show in Library, which opens Library on skill `id` and closes the toast.
    fn with_show_in_library(
        &self,
        toast: Notification,
        id: SkillId,
        cx: &mut Context<Self>,
    ) -> Notification {
        let view = cx.entity().downgrade();
        toast
            .action(move |_, _, cx| {
                let (view, id) = (view.clone(), id.clone());
                Button::new("toast-show-in-library")
                    .label("Show in Library")
                    .on_click(cx.listener(move |toast, _, window, cx| {
                        let id = id.clone();
                        let _ = view.update(cx, |view, cx| view.show_in_library(id, window, cx));
                        toast.dismiss(window, cx);
                    }))
                    .into_kit()
            })
            .autohide(true)
    }
}

/// `toast` with Release notes, which opens the release page `url` and closes the toast.
fn with_release_notes(toast: Notification, url: String) -> Notification {
    toast
        .action(move |_, _, cx| {
            let url = url.clone();
            Button::new("toast-release-notes")
                .label("Release notes")
                .on_click(cx.listener(move |toast, _, window, cx| {
                    cx.open_url(&url);
                    toast.dismiss(window, cx);
                }))
                .into_kit()
        })
        .autohide(true)
}

#[cfg(not(feature = "native-test"))]
impl NativeView {
    /// The toast as a native-test hold keeps it.
    fn hold_toast(
        &mut self,
        toast: Notification,
        _: SharedString,
        _: &mut Context<Self>,
    ) -> Notification {
        toast
    }
}

#[cfg(test)]
mod tests;
