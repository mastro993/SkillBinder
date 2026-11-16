//! Onboarding's last step (design O1 to O6): once the library exists, it scans your agents and
//! sums up what it found, with the Import skills dialog's list behind Review list. Import applies
//! the ticked skills into the Default preset under the progress strip and finishes onboarding
//! into Library; Skip finishes it importing nothing.

mod summary;
#[cfg(test)]
mod tests;

use super::controls::{Inactive, app_icon, text_button};
use super::import_dialog::list::import_label;
use super::import_dialog::{ImportDialog, Phase, alert_glyph, changed_line};
use super::progress_strip::{Activity, Landing};
use super::{Effect, NativeView, Route};
use gpui_kit::component::{ActiveTheme, Size, button::ButtonVariants, h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::FoundImport;

const LEAD: &str = "Importing copies these skills from your agents into your library, each staying active for the agents that have it. You can import others later with Scan for skills in Library.";

/// Onboarding's import step: the list it prepares, and whether Review list shows it.
pub(super) struct FirstImport {
    pub list: ImportDialog,
    /// Review list is expanded; it starts collapsed (O3).
    pub reviewing: bool,
}

impl NativeView {
    /// Starts the step's scan the first time the step shows, after Create library or on a launch
    /// at the step.
    pub(super) fn start_first_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.first_import.is_some() {
            return;
        }
        self.first_import = Some(FirstImport {
            list: ImportDialog::new(false, window, cx),
            reviewing: false,
        });
        self.prepare_found(cx);
    }

    /// The step's column: the heading, then Scanning…, the reason a scan failed, nothing found,
    /// or the lead with the summary and Review list, then the buttons (O1 to O5).
    pub(super) fn render_first_import(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(first) = &self.first_import else {
            return div().into_any_element();
        };
        let muted = cx.theme().muted_foreground;
        let line = |glyph: AnyElement, text: String| {
            h_flex()
                .id("onboarding-import-line")
                .items_start()
                .gap(px(8.))
                .text_color(muted)
                .child(div().flex_none().mt(px(2.)).child(glyph))
                .child(div().min_w_0().flex_1().child(text))
                .into_any_element()
        };
        let lead = || div().text_color(muted).child(LEAD).into_any_element();
        let (body, buttons) = match &first.list.phase {
            Phase::Scanning => (
                vec![
                    lead(),
                    line(
                        Self::spinner("onboarding-scanning").into_any_element(),
                        "Scanning…".to_owned(),
                    ),
                ],
                vec![
                    self.finish_button("onboarding-skip", "Skip", false, cx),
                    self.import_button(None, cx),
                ],
            ),
            Phase::Failed(reason) => (
                vec![line(
                    alert_glyph(cx.theme().danger),
                    format!("Couldn't scan: {reason}"),
                )],
                vec![
                    self.finish_button("onboarding-skip", "Skip", false, cx),
                    text_button("onboarding-retry", "Retry", Size::Medium)
                        .primary()
                        .inactive(self.blocked())
                        .on_click(cx.listener(|view, _, _, cx| view.prepare_found(cx)))
                        .into_any_element(),
                ],
            ),
            Phase::Empty => (
                vec![
                    div()
                        .text_color(muted)
                        .child("No skills to import were found in your agents.")
                        .into_any_element(),
                ],
                vec![self.finish_button("onboarding-next", "Continue", true, cx)],
            ),
            Phase::Ready(found) => (
                self.first_import_review(first, found, cx),
                vec![
                    self.finish_button("onboarding-skip", "Skip", false, cx),
                    self.import_button(Some(found), cx),
                ],
            ),
        };
        // While Review list is expanded the column takes the window's height, so the list fills
        // what is left and scrolls.
        let reviewing = first.reviewing && matches!(first.list.phase, Phase::Ready(_));
        v_flex()
            .id("onboarding-content")
            .w_96()
            .max_h_full()
            .when(reviewing, |column| column.h_full())
            .min_h_0()
            .py_6()
            .gap_4()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(muted)
                            .child("SkillBinder"),
                    )
                    .child(
                        div()
                            .id("onboarding-title")
                            .role(gpui_kit::Role::Heading)
                            .aria_label("Import your skills")
                            .text_size(px(20.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Import your skills"),
                    ),
            )
            .children(body)
            .child(h_flex().justify_end().gap(px(8.)).children(buttons))
            .into_any_element()
    }

    /// The lead, the summary card, and Review list with, while it is expanded, the search, the
    /// list, and the cap note or the acknowledgement.
    fn first_import_review(
        &self,
        first: &FirstImport,
        found: &FoundImport,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let reviewing = first.reviewing;
        let disclosure = text_button("onboarding-review", "Review list", Size::Small)
            .ghost()
            .icon(app_icon(if reviewing {
                "arrow-down"
            } else {
                "chevron-right"
            }))
            .text_color(muted)
            .on_click(cx.listener(|view, _, _, cx| {
                if let Some(first) = view.first_import.as_mut() {
                    first.reviewing = !first.reviewing;
                }
                cx.notify();
            }));
        let mut review = vec![
            div().text_color(muted).child(LEAD).into_any_element(),
            self.summary_card(&found.scan, cx),
            h_flex().child(disclosure).into_any_element(),
        ];
        if first.list.changed {
            review.push(changed_line(cx));
        }
        if reviewing {
            let chosen = first.list.chosen(found, &self.snapshot.workspaces);
            review.push(self.import_search(&first.list, cx));
            review.push(
                v_flex()
                    .flex_1()
                    .min_h(px(160.))
                    .child(self.import_list(&first.list, found, cx))
                    .into_any_element(),
            );
            review.extend(self.import_notice(&first.list, &chosen, cx));
        }
        review
    }

    /// The primary "Import N skills", inactive with no list or while Import is unavailable.
    fn import_button(&self, found: Option<&FoundImport>, cx: &mut Context<Self>) -> AnyElement {
        let list = self.first_import.as_ref().map(|first| &first.list);
        let (label, ready) = match (found, list) {
            (Some(found), Some(list)) => {
                let chosen = list.chosen(found, &self.snapshot.workspaces);
                let ready = chosen.ready(list.acknowledge_invalid) && !list.busy();
                (import_label(chosen.ids.len()), ready)
            }
            _ => (import_label(0), false),
        };
        text_button("onboarding-import", label, Size::Medium)
            .primary()
            .loading(list.is_some_and(ImportDialog::refreshing))
            .inactive(!ready || self.blocked())
            .on_click(cx.listener(|view, _, _, cx| view.import_first(cx)))
            .into_any_element()
    }

    /// Skip or Continue: finishes onboarding importing nothing.
    fn finish_button(
        &self,
        id: &'static str,
        label: &'static str,
        primary: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let button = text_button(id, label, Size::Medium);
        let button = if primary {
            button.primary()
        } else {
            button.outline()
        };
        button
            .inactive(self.blocked())
            .on_click(cx.listener(|view, _, _, cx| {
                let client = view.client.clone();
                view.route = Route::Library;
                view.run(
                    async move { client.complete_onboarding(None).await.map(|_| Effect::None) },
                    cx,
                );
            }))
            .into_any_element()
    }

    /// Imports the ticked skills into Default under the progress strip, finishing onboarding
    /// into Library. An import refused because the list changed prepares it again (D11).
    fn import_first(&mut self, cx: &mut Context<Self>) {
        let Some(import) = self.ticked_import(cx) else {
            return;
        };
        let skills = import.skills.len();
        let client = self.client.clone();
        self.route = Route::Library;
        self.block(
            Activity::Importing { skills },
            async move {
                client.complete_onboarding(Some(import)).await?;
                Ok(Landing { imported: skills })
            },
            cx,
        );
    }
}
