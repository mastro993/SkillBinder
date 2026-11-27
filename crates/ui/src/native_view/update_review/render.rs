//! The review dialog's frame: "Update <slug>" with the source and summary under it (R2), the
//! fetching line (R7), the changes under any moved notice (R3, R4, R9), or the error, and the
//! footer's note, Cancel, and Update (R6, R11).

use super::super::NativeView;
use super::super::explore::repository_glyph;
use super::{Phase, ReviewDialog};
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::{Sizable, h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Pixels, Size, Styled, px,
};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{
    Button, DialogFooter, DialogHeader, DialogSize, DiffView, IconSize, Notice, NoticeSize, Status,
    Text, TextTone, icon,
};
use skillbinder_proto::{Moved, SkillId, SkillSource, UpdateReview, normalized_url};

impl NativeView {
    /// The open review dialog in `dialog`, at the Import skills dialog's size (R1).
    pub(in crate::native_view) fn update_review_dialog(
        &self,
        dialog: Dialog,
        review: &ReviewDialog,
        viewport: Size<Pixels>,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let slug = &review.id;
        let header = DialogHeader::new(format!("Update {slug}"));
        let header = match &review.phase {
            Phase::Ready { review, .. } => header.detail(summary(review)),
            Phase::Fetching | Phase::Failed(_) => header,
        };
        let frame = self
            .dialog_frame(DialogSize::Window(viewport), header, cx)
            .footer(self.review_footer(review, cx));
        let centered = |content: Notice| {
            h_flex()
                .id("update-review-message")
                .size_full()
                .justify_center()
                .items_center()
                .child(content.size(NoticeSize::Md))
        };
        match &review.phase {
            Phase::Fetching => frame.apply(dialog, cx).child(centered(
                Notice::new(
                    "update-review-fetching",
                    format!("Fetching {slug} from its source…"),
                )
                .turning("refresh-04"),
            )),
            Phase::Failed(message) => frame.apply(dialog, cx).child(centered(
                Notice::new("update-review-failed", message.clone()).status(Status::Danger),
            )),
            Phase::Ready { diff, moved, .. } => {
                let body = v_flex()
                    .flex_1()
                    .min_h_0()
                    .gap(px(space::LG))
                    .children(moved.map(|moved| moved_notice(slug, moved)))
                    .child(DiffView::new(diff).flex_1());
                frame.body(body).apply(dialog, cx)
            }
        }
    }

    /// "Nothing changes until you update.", then Cancel and Update (R6); Update shows
    /// "Updating…" while it runs, and only changes can be updated.
    fn review_footer(&self, review: &ReviewDialog, cx: &mut Context<Self>) -> DialogFooter {
        let (ready, updating) = match &review.phase {
            Phase::Ready { updating, .. } => (true, *updating),
            Phase::Fetching | Phase::Failed(_) => (false, false),
        };
        let update = Button::new("update-review-update")
            .label(if updating { "Updating…" } else { "Update" })
            .primary()
            .loading(updating)
            .inactive(!ready || self.working())
            .on_click(cx.listener(|view, _, _, cx| view.update_reviewed(cx)));
        DialogFooter::new()
            .leading(
                Text::labeled("update-review-note", "Nothing changes until you update.").muted(),
            )
            .secondary(self.dialog_cancel("update-review-cancel", cx))
            .primary(update)
    }
}

/// The source's repository glyph and its repository and folder in mono, then "· <N> files
/// changed · +<A> −<D>" in the success and danger tones (R2).
fn summary(review: &UpdateReview) -> AnyElement {
    let (added, removed) = review.totals();
    let figure = |text: String, status: Status| {
        Text::new(text)
            .level(text::BODY_SM)
            .tabular()
            .tone(TextTone::Status(status.text()))
    };
    h_flex()
        .id("update-review-summary")
        .min_w_0()
        .gap(px(space::XS))
        .child(icon(repository_glyph(&review.source.url)).with_size(IconSize::Sm))
        .child(
            Text::new(place(&review.source))
                .level(text::MONO_SM)
                .muted()
                .min_w_0()
                .truncate(),
        )
        .child(
            Text::new(files_changed(review.files.len()))
                .level(text::BODY_SM)
                .muted(),
        )
        .child(figure(format!("+{added}"), Status::Success))
        .child(figure(format!("−{removed}"), Status::Danger))
        .into_any_element()
}

/// The source's repository without its scheme, then its folder: `github.com/acme/skills ·
/// skills/pdf`.
fn place(source: &SkillSource) -> String {
    let normalized = normalized_url(&source.url);
    let repository = normalized
        .split_once("://")
        .map_or(normalized.as_str(), |(_, rest)| rest);
    match source.path.as_str() {
        "" => repository.to_owned(),
        path => format!("{repository} · {path}"),
    }
}

/// "· 1 file changed ·" or "· <N> files changed ·".
fn files_changed(count: usize) -> String {
    match count {
        1 => "· 1 file changed ·".to_owned(),
        count => format!("· {count} files changed ·"),
    }
}

/// Why the changes shown replaced the reviewed ones (R9).
fn moved_notice(slug: &SkillId, moved: Moved) -> Notice {
    Notice::new("update-review-moved", moved_text(slug, moved)).status(Status::Info)
}

fn moved_text(slug: &SkillId, moved: Moved) -> String {
    match moved {
        Moved::Source => format!(
            "{slug}'s source changed while you were reviewing. These are its newest changes; review them, then update."
        ),
        Moved::Library => format!(
            "{slug} changed in your library while you were reviewing. These are the changes now; review them, then update."
        ),
    }
}

#[cfg(test)]
mod tests;
