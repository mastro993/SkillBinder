//! How the review list draws: a standard dialog 56 px from the window's top edge, with the
//! copies in a card and Done.

use super::super::copy_dialogs::CopiesListing;
use super::super::import_review::place_name;
use super::super::{DialogState, Effect, NativeView};
use super::{Answer, Choice, Item, Review, Row, attention_label};
use gpui_kit::component::{ActiveTheme, Sizable, dialog::Dialog};
use gpui_kit::{Context, Div, ParentElement, Styled, div, px};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{
    Button, ButtonVariant, CardList, DialogFooter, DialogHeader, DialogSize, IconSize, ListRow,
    Notice, Status, Text, icon,
};
use skillbinder_proto::{CopyChoice, ManagedCopy};

/// Distance from the window's top edge to the review dialog.
const TOP: f32 = 56.;

impl NativeView {
    /// The review list in Kit's `dialog`: its title, the copies in a card, and Done.
    pub(in crate::native_view) fn review_dialog(
        &self,
        dialog: Dialog,
        review: &Review,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let copies = self
            .snapshot
            .library
            .as_ref()
            .map(|library| library.copies.as_slice())
            .unwrap_or_default();
        let count = review.len();
        let rows: Vec<_> = review
            .items
            .iter()
            .enumerate()
            .map(|(index, (item, reason))| {
                let row = review.row(item, copies);
                self.review_row(index, (item, reason), row, cx)
            })
            .collect();
        let done = Button::new("copies-review-done")
            .label("Done")
            .on_click(cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)));
        let header = DialogHeader::new(attention_label(count));
        self.dialog_frame(DialogSize::Standard, header, cx)
            .footer(DialogFooter::new().secondary(done))
            .apply(dialog, cx)
            .margin_top(px(TOP))
            .child(CardList::new("copies-review").rows(rows))
    }

    /// One copy: slug and place, why it needs the user, and its answers or how it settled.
    fn review_row(
        &self,
        index: usize,
        (item, listed): (&Item, &'static str),
        row: Row,
        cx: &mut Context<Self>,
    ) -> ListRow {
        let snapshot = &self.snapshot;
        let place = place_name(
            &item.0,
            &snapshot.agents,
            &snapshot.workspaces,
            &snapshot.home,
        );
        // An answered row keeps the reason it was listed for.
        let reason = match &row {
            Row::Ask(choice, _) => choice.reason,
            Row::Settled(_) => listed,
        };
        let title = div()
            .flex()
            .items_center()
            .gap(px(space::SM))
            .min_w_0()
            .child(Text::labeled(("copies-review-slug", index), item.1.to_string()).flex_none())
            .child(
                Text::labeled(("copies-review-place", index), format!("→ {place}"))
                    .level(text::BODY_SM)
                    .muted()
                    .min_w_0()
                    .truncate(),
            );
        let end = match row {
            Row::Ask(choice, copy) => self.review_answers(index, choice, copy, cx),
            Row::Settled(word) => div()
                .flex()
                .items_center()
                .gap(px(space::XS))
                .text_color(cx.theme().muted_foreground)
                .child(icon("check").with_size(IconSize::Sm))
                .child(Text::labeled(("copies-review-settled", index), word).level(text::CAPTION)),
        };
        ListRow::new(("copies-review-row", index))
            .title(title)
            .subtitle(Notice::new(("copies-review-reason", index), reason).status(Status::Warning))
            .trailing(end)
            .hover(true)
    }

    /// The row's two answers.
    fn review_answers(
        &self,
        index: usize,
        choice: Choice,
        copy: ManagedCopy,
        cx: &mut Context<Self>,
    ) -> Div {
        let button = |id, answer: Answer, look: ButtonVariant, cx: &mut Context<Self>| {
            let copy = copy.clone();
            Button::new((id, index))
                .variant(look)
                .label(answer.label())
                .small()
                .on_click(cx.listener(move |view, _, _, cx| view.answer_review(answer, &copy, cx)))
        };
        div()
            .flex()
            .items_center()
            .gap(px(space::MD))
            .child(button(
                "copies-review-yes",
                choice.yes,
                ButtonVariant::Primary,
                cx,
            ))
            .child(button(
                "copies-review-no",
                choice.no,
                ButtonVariant::Secondary,
                cx,
            ))
    }

    /// Records `answer` for `copy` and carries it out; the row settles once the copy no longer
    /// needs the user. Skip only settles the row.
    fn answer_review(&mut self, answer: Answer, copy: &ManagedCopy, cx: &mut Context<Self>) {
        if self.blocked() {
            return;
        }
        let DialogState::Copies(CopiesListing::Attention(review)) = &mut self.dialog else {
            return;
        };
        review
            .answers
            .insert((copy.directory.clone(), copy.skill.clone()), answer);
        let client = self.client.clone();
        let copy = copy.clone();
        let choice = match answer {
            Answer::Skip => return cx.notify(),
            Answer::Retry => None,
            Answer::Replace | Answer::Remove => Some(CopyChoice::Replace),
            Answer::Keep => Some(CopyChoice::Keep),
        };
        self.run(
            async move {
                match choice {
                    Some(choice) => client.resolve_copy(&copy, choice).await,
                    None => client.retry_copies().await,
                }
                .map(|()| Effect::None)
            },
            cx,
        );
    }
}
