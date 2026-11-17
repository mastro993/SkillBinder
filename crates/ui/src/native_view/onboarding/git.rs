//! The step shown while no usable Git is found (design T1, T5): Git is optional, so the step
//! offers Skip beside Check again.

use super::super::controls::{Inactive, text_button};
use super::super::{Effect, NativeView};
use super::StepView;
use gpui_kit::component::{ActiveTheme, Size, button::ButtonVariants};
use gpui_kit::{Context, IntoElement, ParentElement, Styled, div};

const LEAD: &str = "Git syncs your library between computers and updates skills from their \
                    sources. Everything else works without it, and you can install it later in \
                    Settings.";

impl NativeView {
    /// The step; once skipped and gone back to, it no longer offers Skip.
    pub(super) fn git_step(&self, skipped: bool, cx: &mut Context<Self>) -> StepView {
        let skip = (!skipped).then(|| {
            text_button("onboarding-skip-git", "Skip", Size::Medium)
                .ghost()
                .inactive(self.blocked())
                .on_click(cx.listener(|view, _, _, cx| {
                    let client = view.client.clone();
                    view.run(
                        async move { client.skip_git().await.map(|_| Effect::None) },
                        cx,
                    );
                }))
                .into_any_element()
        });
        StepView {
            title: "Install Git",
            body: vec![
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(LEAD)
                    .into_any_element(),
            ],
            leading: skip,
            actions: vec![self.check_again(!skipped, cx)],
        }
    }
}
