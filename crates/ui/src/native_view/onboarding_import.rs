//! Onboarding's last step: once the library exists, it reviews importing the skills already in
//! your agents, each active for the agents whose global skill directory held it, or skips it.

use super::controls::{Inactive, button};
use super::{Effect, NativeView};
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div, prelude::FluentBuilder, px,
};
use skillbinder_proto::ImportPlan;

/// Onboarding's prepared review of the library's first import.
pub(super) enum FirstImport {
    /// The skills to import.
    Plan(ImportPlan),
    /// Nothing in your agents can be imported.
    Nothing,
}

impl From<Option<ImportPlan>> for FirstImport {
    fn from(plan: Option<ImportPlan>) -> Self {
        plan.map_or(Self::Nothing, Self::Plan)
    }
}

impl NativeView {
    /// The import review step: a button that prepares the review until it is prepared, then the
    /// review with Skip and Import, or Continue when there is nothing to import.
    pub(super) fn render_first_import(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let detail = match &self.first_import {
            None => {
                "SkillBinder can import the skills your agents already have, each staying active for the agents that have it."
            }
            Some(FirstImport::Nothing) => "No skills to import were found in your agents.",
            Some(FirstImport::Plan(_)) => {
                "Importing copies these skills from your agents into your library, each staying active for the agents that have it. Skills you skip stay in Discovery."
            }
        };
        let review = match &self.first_import {
            Some(FirstImport::Plan(plan)) => Some(self.scrolled_review(plan, cx)),
            _ => None,
        };
        let plan = match &self.first_import {
            Some(FirstImport::Plan(plan)) => Some(plan.id.clone()),
            _ => None,
        };
        let primary = match &self.first_import {
            None => "Review skills",
            Some(FirstImport::Nothing) => "Continue",
            Some(FirstImport::Plan(_)) => "Import",
        };
        let prepared = self.first_import.is_some();
        div()
            .id("onboarding-content")
            .w_96()
            .max_h_full()
            .py_6()
            .flex()
            .flex_col()
            .gap_4()
            .child("SkillBinder")
            .child(
                div()
                    .id("onboarding-title")
                    .role(gpui_kit::Role::Heading)
                    .aria_label("Import your skills")
                    .child("Import your skills"),
            )
            .child(detail)
            .children(review)
            .when(
                !matches!(self.first_import, Some(FirstImport::Nothing)),
                |row| {
                    row.child(
                        button("onboarding-skip")
                            .label("Skip")
                            .inactive(self.busy)
                            .on_click(cx.listener(|view, _, _, cx| {
                                let client = view.client.clone();
                                view.run(
                                    async move {
                                        client.complete_onboarding(None).await.map(|_| Effect::None)
                                    },
                                    cx,
                                );
                            })),
                    )
                },
            )
            .child(
                button("onboarding-next")
                    .label(primary)
                    .primary()
                    .inactive(self.busy)
                    .on_click(cx.listener(move |view, _, _, cx| {
                        let client = view.client.clone();
                        let plan = plan.clone();
                        view.run(
                            async move {
                                if !prepared {
                                    let plan = client.prepare_first_import().await?;
                                    return Ok(Effect::FirstImport(plan));
                                }
                                // A plan that expired or no longer matches is reviewed again.
                                Ok(match client.complete_onboarding(plan).await {
                                    Ok(_) => Effect::None,
                                    Err(error) => Effect::FirstImportFailed(error),
                                })
                            },
                            cx,
                        );
                    })),
            )
    }

    /// The review rows in an area that scrolls once they outgrow the window, so the title and
    /// the Skip and Import buttons stay in view. The page's scroll position serves it, since
    /// onboarding shows no page.
    fn scrolled_review(
        &self,
        plan: &ImportPlan,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        div()
            .id("onboarding-review")
            .relative()
            .flex()
            .flex_col()
            .min_h_0()
            .on_hover(cx.listener(|view, hovered: &bool, _, cx| {
                if view.content_scroll.hover(*hovered) {
                    cx.notify();
                }
            }))
            .child(
                div()
                    .id("onboarding-review-rows")
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.content_scroll.handle)
                    .child(self.import_review(plan)),
            )
            .child(
                self.content_scroll
                    .scrollbar("onboarding-review-scrollbar", px(0.), cx),
            )
    }
}
