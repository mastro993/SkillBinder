//! The step shown while application data is not writable (design F3): its path in the danger
//! color and Check again, with no Skip.

use super::super::NativeView;
use super::StepView;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{Context, IntoElement, ParentElement, Styled, div};

impl NativeView {
    pub(super) fn data_location_step(&self, cx: &mut Context<Self>) -> StepView {
        let path = &self.snapshot.bootstrap.data_path;
        StepView {
            title: "SkillBinder can't save its data",
            body: vec![
                div()
                    .text_color(cx.theme().danger)
                    .child(format!("{path} isn't writable."))
                    .into_any_element(),
            ],
            leading: None,
            actions: vec![self.check_again(cx)],
        }
    }
}
