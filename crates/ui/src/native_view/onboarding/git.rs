//! The step shown while no usable Git is found. Git is still required, so the step offers only
//! Check again.

use super::super::NativeView;
use super::StepView;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{Context, IntoElement, ParentElement, Styled, div};

impl NativeView {
    pub(super) fn git_step(&self, cx: &mut Context<Self>) -> StepView {
        StepView {
            title: "Install Git",
            body: vec![
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child("SkillBinder needs Git 2.39 or newer. Install it, then check again.")
                    .into_any_element(),
            ],
            actions: vec![self.check_again(cx)],
            fill: false,
        }
    }
}
