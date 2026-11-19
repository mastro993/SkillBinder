//! Onboarding's frame (design F1 to F4, P1): the steps that apply, from the engine, in a centered
//! column under "SkillBinder · Step N of M" and the step's title, with Back at the left of the
//! button row and the step's actions at its right. Each step's view lives in its own module.

mod agents;
mod data_location;
mod git;
pub(super) mod import;
#[cfg(test)]
mod tests;

use super::{Effect, NativeView};
use gpui_kit::component::{ActiveTheme, h_flex, scroll::ScrollableElement, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, linear_color_stop, linear_gradient, px,
};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{Button, SectionHeader, icon};
use skillbinder_proto::{Onboarding, OnboardingStep};

/// The step the frame shows and where it stands among the steps that apply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Place {
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

/// Where the frame shows the Git step, if it does. Once Git is found, the frame stays there while
/// "Git <version> is ready" shows (T6), though the step then no longer applies.
pub(super) fn git_step_shown(
    onboarding: &Onboarding,
    back_to: Option<OnboardingStep>,
) -> Option<Place> {
    place(onboarding, back_to).filter(|place| place.step == OnboardingStep::Git)
}

/// The column's width: the steps' 384 points, or 576 for Choose your agents' three tile columns
/// (design P6): layout measured from the approved onboarding prototype.
const NARROW: f32 = 384.;
const WIDE: f32 = 576.;
/// The space between the step's content and its button row.
const BUTTONS_GAP: f32 = space::XXXL;
/// How far the content fades out under a pinned button row.
const FADE: f32 = space::XXL;

/// A step's part of the frame.
pub(super) struct StepView {
    title: &'static str,
    /// Its column is [`WIDE`] rather than [`NARROW`].
    wide: bool,
    /// What sits under the title: the lead, then the step's content.
    body: Vec<AnyElement>,
    /// The button at the left of the button row, after Back, such as Skip.
    leading: Option<AnyElement>,
    /// The buttons at the right of the button row, the primary last.
    actions: Vec<AnyElement>,
}

impl NativeView {
    pub(super) fn render_onboarding(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let shown = self
            .git_ready
            .clone()
            .or_else(|| place(&self.snapshot.bootstrap.onboarding, self.onboarding_back_to));
        let Some(place) = shown else {
            return div().into_any_element();
        };
        let view = match place.step {
            OnboardingStep::DataLocation => self.data_location_step(cx),
            OnboardingStep::Git => self.git_step(place.forward.is_some(), cx),
            OnboardingStep::Agents => self.agents_step(place.forward.is_some(), cx),
            OnboardingStep::Import => self.import_step(window, cx),
        };
        let back = place.back.map(|step| {
            Button::new("onboarding-back")
                .label("Back")
                .icon(icon("chevron-left"))
                .inactive(self.blocked())
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.onboarding_back_to = Some(step);
                    cx.notify();
                }))
        });
        // A done step gone back to leads on to the next one, except Choose your agents, whose
        // Continue saves the choice again.
        let forward = place
            .forward
            .filter(|_| place.step != OnboardingStep::Agents);
        let forward = forward.map(|step| {
            Button::new("onboarding-continue")
                .label("Continue")
                .primary()
                .inactive(self.blocked())
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.onboarding_back_to = Some(step);
                    cx.notify();
                }))
        });
        let theme = cx.theme();
        let background = theme.background;
        let width = px(if view.wide { WIDE } else { NARROW });
        let head = v_flex()
            .gap(px(space::XS))
            .child(SectionHeader::new("onboarding-eyebrow", place.eyebrow()))
            .child(
                text::TITLE_LG
                    .apply(div().id("onboarding-title"))
                    .role(gpui_kit::Role::Heading)
                    .aria_label(view.title)
                    .child(view.title),
            );
        let buttons = h_flex()
            .w(width)
            .flex_none()
            .pt(px(BUTTONS_GAP - FADE))
            .pb(px(space::XXL))
            .gap(px(space::MD))
            .children(back)
            .children(view.leading)
            .child(
                h_flex()
                    .flex_1()
                    .justify_end()
                    .gap(px(space::MD))
                    .children(view.actions)
                    .children(forward),
            );
        // The content and buttons sit centered together. Content taller than the window scrolls
        // at the window's edge, never inside the column, and fades out under the buttons, which
        // stay in view (design A4).
        let content = div()
            .id("onboarding-scroll")
            .w_full()
            .min_h_0()
            .child(
                h_flex().w_full().justify_center().child(
                    v_flex()
                        .id("onboarding-content")
                        .w(width)
                        .pt(px(space::XXL))
                        .pb(px(FADE))
                        .gap(px(space::XL))
                        .child(head)
                        .children(view.body),
                ),
            )
            .overflow_y_scrollbar();
        let fade = div()
            .absolute()
            .bottom_0()
            .left_0()
            .right_0()
            .h(px(FADE))
            .bg(linear_gradient(
                180.,
                linear_color_stop(background.opacity(0.), 0.),
                linear_color_stop(background, 1.),
            ));
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .relative()
                    .w_full()
                    .min_h_0()
                    .child(content)
                    .child(fade),
            )
            .child(buttons)
            .into_any_element()
    }

    /// Check again, which rechecks Git and storage.
    fn check_again(&self, cx: &mut Context<Self>) -> AnyElement {
        Button::new("onboarding-check-again")
            .label("Check again")
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
