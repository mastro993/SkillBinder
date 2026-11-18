//! Onboarding's last step (design O1, O2, O4, O5, O7): once the library exists, it scans your
//! agents and sums up what it found; Review list opens the Import skills dialog over the step's
//! own list, ending with Done. Import applies the ticked skills into the Default preset under
//! the progress strip and finishes onboarding into Library; Skip finishes it importing nothing.

mod summary;
#[cfg(test)]
mod tests;

use super::super::import_dialog::list::import_label;
use super::super::import_dialog::{End, ImportDialog, Phase, alert_glyph, changed_line};
use super::super::library_updates::spinner;
use super::super::progress_strip::{Activity, Landing};
use super::super::{DialogState, Effect, NativeView, Route};
use super::StepView;
use gpui_kit::component::{ActiveTheme, h_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled, Window, div, px,
};
use skillbinder_design::tokens::color;
use skillbinder_design::{Button, icon};
use skillbinder_proto::FoundImport;

const TITLE: &str = "Import your skills";
const LEAD: &str = "Importing copies these skills from your agents into your library, each staying active for the agents that have it. You can import others later with Scan for skills in Library.";

impl NativeView {
    /// Starts the step's scan the first time the step shows.
    fn start_first_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.first_import.is_some() {
            return;
        }
        self.first_import = Some(ImportDialog::new(End::Done, window, cx));
        self.prepare_found(cx);
    }

    /// The step: Scanning…, the reason a scan failed, nothing found, or the lead with the
    /// summary and Review list, then the buttons (O1 to O5).
    pub(super) fn import_step(&mut self, window: &mut Window, cx: &mut Context<Self>) -> StepView {
        self.start_first_import(window, cx);
        let Some(list) = &self.first_import else {
            return StepView {
                title: TITLE,
                wide: false,
                body: Vec::new(),
                leading: None,
                actions: Vec::new(),
            };
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
        let (body, buttons) = match &list.phase {
            Phase::Scanning => (
                vec![
                    lead(),
                    line(spinner().into_any_element(), "Scanning…".to_owned()),
                ],
                vec![
                    self.finish_button("onboarding-skip", "Skip", false, cx),
                    self.import_button(None, cx),
                ],
            ),
            Phase::Failed(reason) => (
                vec![line(
                    alert_glyph(color::DANGER_TEXT.of(cx.theme())),
                    format!("Couldn't scan: {reason}"),
                )],
                vec![
                    self.finish_button("onboarding-skip", "Skip", false, cx),
                    Button::new("onboarding-retry")
                        .label("Retry")
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
                self.first_import_summary(list, found, cx),
                vec![
                    self.finish_button("onboarding-skip", "Skip", false, cx),
                    self.import_button(Some(found), cx),
                ],
            ),
        };
        StepView {
            title: TITLE,
            wide: false,
            body,
            leading: None,
            actions: buttons,
        }
    }

    /// The lead, the summary card, and Review list, which opens the Import skills dialog over
    /// the step's own list (O7), then the line saying the list changed, if it did.
    fn first_import_summary(
        &self,
        list: &ImportDialog,
        found: &FoundImport,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let muted = cx.theme().muted_foreground;
        let review = Button::new("onboarding-review")
            .label("Review list")
            .small()
            .ghost()
            .icon(icon("chevron-right"))
            .text_color(muted)
            .inactive(self.blocked())
            .on_click(cx.listener(|view, _, _, cx| {
                view.show_dialog(DialogState::FirstImport);
                cx.notify();
            }));
        let mut summary = vec![
            div().text_color(muted).child(LEAD).into_any_element(),
            self.summary_card(&found.scan, cx),
            h_flex().child(review).into_any_element(),
        ];
        if list.changed {
            summary.push(changed_line(cx));
        }
        summary
    }

    /// The primary "Import N skills", inactive with no list or while Import is unavailable.
    fn import_button(&self, found: Option<&FoundImport>, cx: &mut Context<Self>) -> AnyElement {
        let list = self.first_import.as_ref();
        let (label, ready) = match (found, list) {
            (Some(found), Some(list)) => {
                let chosen = list.chosen(found, &self.snapshot.workspaces);
                let ready = chosen.ready(list.acknowledge_invalid) && !list.busy();
                (import_label(chosen.ids.len()), ready)
            }
            _ => (import_label(0), false),
        };
        Button::new("onboarding-import")
            .label(label)
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
        let button = Button::new(id).label(label);
        let button = if primary { button.primary() } else { button };
        button
            .inactive(self.blocked())
            .on_click(cx.listener(|view, _, _, cx| {
                let client = view.client.clone();
                view.route = Route::Library;
                view.run(
                    async move { client.finish_onboarding(None).await.map(|_| Effect::None) },
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
                client.finish_onboarding(Some(import)).await?;
                Ok(Landing { imported: skills })
            },
            cx,
        );
    }
}
