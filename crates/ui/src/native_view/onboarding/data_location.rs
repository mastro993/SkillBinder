//! The step shown while application data is not writable (design F3): its path in the danger
//! color and Check again, with no Skip.

use super::super::NativeView;
use super::StepView;
use gpui_kit::{Context, IntoElement};
use skillbinder_design::{Notice, NoticeSize, Status};

impl NativeView {
    pub(super) fn data_location_step(&self, cx: &mut Context<Self>) -> StepView {
        let path = &self.snapshot.bootstrap.data_path;
        StepView {
            title: "SkillBinder can't save its data",
            wide: false,
            body: vec![
                Notice::new("data-location-note", format!("{path} isn't writable."))
                    .status(Status::Danger)
                    .size(NoticeSize::Md)
                    .into_any_element(),
            ],
            leading: None,
            actions: vec![self.check_again(cx)],
        }
    }
}
