//! How the review list draws: the app's dialog, 420 wide, with the copies in a card and Done.

use super::super::controls::app_icon;
use super::super::controls::dialog_surface::{
    body, close_button, fading_in, footer, header, scrim, surface,
};
use super::super::controls::exact_text::exact_text;
use super::super::controls::text_button::{Height, Look, text_button};
use super::super::copy_dialogs::CopiesListing;
use super::super::import_review::place_name;
use super::super::{DialogState, Effect, NativeView};
use super::{Answer, Choice, Item, Row};
use gpui_kit::base::FocusTrapElement;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, Context, Div, FontWeight, InteractiveElement, IntoElement, KeyDownEvent,
    MouseButton, ParentElement, StatefulInteractiveElement, Styled, div, prelude::FluentBuilder,
    px,
};
use skillbinder_proto::{CopyChoice, ManagedCopy};
use skillbinder_theme::library::surface as card_surface;

/// Width of the review dialog.
const WIDTH: f32 = 420.;
/// Line box of a row's slug, 14 px at line height 1.4; GPUI rounds a text line to whole pixels.
const TITLE_LINE: f32 = 19.6;
/// Space between a row's slug line and its reason line.
const LINE_GAP: f32 = 2.;
/// Least height of a row's reason line.
const REASON_LINE: f32 = 18.;
/// Height of a row's answer buttons.
const ANSWER_HEIGHT: f32 = 24.;
/// Where the answers sit below a row's padding: centered on the row's two lines as the design
/// lays them out. Centered in GPUI's row instead, whose slug line rounds to 19.5, they land exactly
/// on a half device pixel, which GPUI rounds toward the top, a device pixel above the design.
const ANSWERS_TOP: f32 = (TITLE_LINE + LINE_GAP + REASON_LINE - ANSWER_HEIGHT) / 2.;

impl NativeView {
    /// The review list over the window, while it is open.
    pub(in crate::native_view) fn render_review(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let DialogState::Copies(CopiesListing::Attention(review)) = &self.dialog else {
            return None;
        };
        let copies = self
            .snapshot
            .library
            .as_ref()
            .map(|library| library.copies.as_slice())
            .unwrap_or_default();
        let count = review.items.len();
        let title = match count {
            1 => "1 needs attention".to_owned(),
            count => format!("{count} need attention"),
        };
        let rows: Vec<_> = review
            .items
            .iter()
            .enumerate()
            .map(|(index, (item, reason))| {
                let row = review.row(item, copies);
                self.review_row(index, count, (item, reason), row, cx)
            })
            .collect();
        let theme = cx.theme();
        let card = div()
            .flex()
            .flex_col()
            .border_1()
            .border_color(theme.border)
            .rounded(px(10.))
            .bg(card_surface(theme))
            .children(rows);
        let close = close_button("copies-review-close", cx)
            .on_click(cx.listener(|view, _, _, cx| view.close_review(cx)));
        let dialog = surface(WIDTH, cx)
            .occlude()
            .child(header(title, close))
            .child(body().child(card))
            .child(
                footer().child(
                    text_button(
                        "copies-review-done",
                        "Done",
                        Look::Outline,
                        Height::Regular,
                        cx,
                    )
                    .on_click(cx.listener(|view, _, _, cx| view.close_review(cx))),
                ),
            );
        // The focus trap also tracks the overlay's focus, so keys reach the scrim.
        let scrim = scrim("copies-review", dialog, cx)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|view, _, _, cx| view.close_review(cx)),
            )
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    view.close_review(cx);
                }
            }))
            .focus_trap("copies-review-trap", &self.overlay_focus);
        Some(fading_in("copies-review-fade", scrim))
    }

    /// One copy: slug and place, why it needs the user, and its answers or how it settled.
    fn review_row(
        &self,
        index: usize,
        count: usize,
        (item, listed): (&Item, &'static str),
        row: Row,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let (muted, hover, line) = (theme.muted_foreground, theme.muted, theme.border);
        let warning = theme.warning;
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
        let main = div()
            .flex()
            .flex_col()
            .gap(px(LINE_GAP))
            .flex_1()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .h(px(TITLE_LINE))
                    .min_w_0()
                    .child(
                        div()
                            .flex_none()
                            .text_size(px(14.))
                            .line_height(px(TITLE_LINE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(
                                exact_text(item.1.to_string()).named(("copies-review-slug", index)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .min_w_0()
                            .text_color(muted)
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(
                                exact_text(format!("→ {place}"))
                                    .named(("copies-review-place", index)),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .min_h(px(REASON_LINE))
                    .text_size(px(11.5))
                    .line_height(px(16.1))
                    .text_color(muted)
                    .child(app_icon("alert-circle").size(px(13.)).text_color(warning))
                    .child(exact_text(reason).named(("copies-review-reason", index))),
            );
        let end = match row {
            Row::Ask(choice, copy) => self
                .review_answers(index, choice, copy, cx)
                .self_start()
                .mt(px(ANSWERS_TOP)),
            Row::Settled(word) => div()
                .flex()
                .items_center()
                .gap(px(4.))
                .text_size(px(12.))
                .text_color(muted)
                .child(app_icon("check").size(px(14.)))
                .child(word),
        };
        div()
            .id(("copies-review-row", index))
            .relative()
            .flex()
            .items_center()
            .gap(px(12.))
            .px(px(14.))
            .py(px(10.))
            .when(index == 0, |row| row.rounded_t(px(9.)))
            .when(index + 1 == count, |row| row.rounded_b(px(9.)))
            .hover(move |style| style.bg(hover))
            .when(index > 0, |row| {
                row.child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(14.))
                        .right(px(14.))
                        .h(px(1.))
                        .bg(line),
                )
            })
            .child(main)
            .child(end.flex_none())
            .into_any_element()
    }

    /// The row's two answers.
    fn review_answers(
        &self,
        index: usize,
        choice: Choice,
        copy: ManagedCopy,
        cx: &mut Context<Self>,
    ) -> Div {
        let button = |id, answer: Answer, look, cx: &mut Context<Self>| {
            let copy = copy.clone();
            text_button((id, index), answer.label(), look, Height::Small, cx)
                .on_click(cx.listener(move |view, _, _, cx| view.answer_review(answer, &copy, cx)))
        };
        div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(button("copies-review-yes", choice.yes, Look::Primary, cx))
            .child(button("copies-review-no", choice.no, Look::Outline, cx))
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

    fn close_review(&mut self, cx: &mut Context<Self>) {
        if matches!(
            self.dialog,
            DialogState::Copies(CopiesListing::Attention(_))
        ) {
            self.dialog = DialogState::None;
            cx.notify();
        }
    }
}
