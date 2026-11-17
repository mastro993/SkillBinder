//! Onboarding's frame (design F1 to F4, P1): the steps that apply, from the engine, in a centered
//! column under "SkillBinder · Step N of M" and the step's title, with Back at the left of the
//! button row and the step's actions at its right. Each step's view lives in its own module.

mod data_location;
mod git;
pub(super) mod import;
#[cfg(test)]
mod tests;

use super::controls::{Inactive, app_icon, text_button};
use super::{Effect, NativeView};
use gpui_kit::component::{ActiveTheme, Size, button::ButtonVariants, h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Text, Window, div, px,
};
use skillbinder_proto::{Onboarding, OnboardingStep};

/// The step the frame shows and where it stands among the steps that apply.
#[derive(Debug, PartialEq, Eq)]
struct Place {
    step: OnboardingStep,
    /// Its position among the steps that apply, from 1.
    number: usize,
    /// How many steps apply.
    count: usize,
    /// The step Back returns to.
    back: Option<OnboardingStep>,
    /// The step Continue leads to, while the shown step is done.
    forward: Option<OnboardingStep>,
}

impl Place {
    fn eyebrow(&self) -> String {
        format!("SkillBinder · Step {} of {}", self.number, self.count)
    }
}

/// Where the frame stands: the step gone `back_to`, while it applies before the first step not
/// done, or else that step; `None` once every step is done. Going forward to the first step not
/// done shows it, since `back_to` then no longer comes before it.
fn place(onboarding: &Onboarding, back_to: Option<OnboardingStep>) -> Option<Place> {
    let steps = &onboarding.steps;
    let current = steps.iter().position(|status| !status.done)?;
    let shown = back_to
        .and_then(|step| {
            steps
                .iter()
                .take(current)
                .position(|status| status.step == step)
        })
        .unwrap_or(current);
    Some(Place {
        step: steps.get(shown)?.step,
        number: shown + 1,
        count: steps.len(),
        back: shown
            .checked_sub(1)
            .and_then(|previous| steps.get(previous))
            .map(|status| status.step),
        forward: (shown < current)
            .then(|| steps.get(shown + 1))
            .flatten()
            .map(|status| status.step),
    })
}

/// A step's part of the frame.
pub(super) struct StepView {
    title: &'static str,
    /// What sits under the title: the lead, then the step's content.
    body: Vec<AnyElement>,
    /// The buttons at the right of the button row, the primary last.
    actions: Vec<AnyElement>,
}

impl NativeView {
    pub(super) fn render_onboarding(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(place) = place(&self.snapshot.bootstrap.onboarding, self.onboarding_back_to)
        else {
            return div().into_any_element();
        };
        let view = match place.step {
            OnboardingStep::DataLocation => self.data_location_step(cx),
            OnboardingStep::Git => self.git_step(cx),
            // The engine does not offer it yet.
            OnboardingStep::Agents => StepView {
                title: "Choose your agents",
                body: Vec::new(),
                actions: Vec::new(),
            },
            OnboardingStep::Import => self.import_step(window, cx),
        };
        let back = place.back.map(|step| {
            text_button("onboarding-back", "Back", Size::Medium)
                .outline()
                .icon(app_icon("chevron-left"))
                .inactive(self.blocked())
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.onboarding_back_to = Some(step);
                    cx.notify();
                }))
        });
        // A done step gone back to leads on to the next one.
        let forward = place.forward.map(|step| {
            text_button("onboarding-continue", "Continue", Size::Medium)
                .primary()
                .inactive(self.blocked())
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.onboarding_back_to = Some(step);
                    cx.notify();
                }))
        });
        let muted = cx.theme().muted_foreground;
        let eyebrow = place.eyebrow();
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .id("onboarding-content")
                    .w_96()
                    .max_h_full()
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
                                    .child(Text::new("onboarding-eyebrow".into(), eyebrow.into())),
                            )
                            .child(
                                div()
                                    .id("onboarding-title")
                                    .role(gpui_kit::Role::Heading)
                                    .aria_label(view.title)
                                    .text_size(px(20.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(view.title),
                            ),
                    )
                    .children(view.body)
                    .child(
                        h_flex().gap(px(8.)).children(back).child(
                            h_flex()
                                .flex_1()
                                .justify_end()
                                .gap(px(8.))
                                .children(view.actions)
                                .children(forward),
                        ),
                    ),
            )
            .into_any_element()
    }

    /// The primary Check again, which rechecks Git and storage.
    fn check_again(&self, cx: &mut Context<Self>) -> AnyElement {
        text_button("onboarding-check-again", "Check again", Size::Medium)
            .primary()
            .inactive(self.blocked())
            .on_click(cx.listener(|view, _, _, cx| {
                let client = view.client.clone();
                view.run(
                    async move { client.bootstrap().await.map(|_| Effect::None) },
                    cx,
                );
            }))
            .into_any_element()
    }
}
