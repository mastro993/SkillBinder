use super::{Effect, NativeView};
use ely_gpui_component::buttons::Button;
use gpui::{Context, IntoElement, ParentElement, Styled, div, prelude::FluentBuilder};
use skillbinder_proto::OnboardingStep;

impl NativeView {
    pub(super) fn render_onboarding(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let step = self.snapshot.bootstrap.step;
        let ready = self.snapshot.bootstrap.git_version.is_some()
            && self.snapshot.bootstrap.storage_writable;
        let (title, detail) = match step {
            OnboardingStep::Prerequisites => (
                "Check this computer",
                "SkillBinder needs Git and a writable application data location.",
            ),
            OnboardingStep::Boundaries => (
                "Keep sources separate",
                "Discovery reads configured project roots. Imports create managed copies in your private library.",
            ),
            OnboardingStep::SyncChoice => (
                "Sync is optional",
                "You can keep the library only on this computer and connect a Git remote later.",
            ),
            OnboardingStep::Ready => (
                "Create your library",
                "The managed library will be created in application data.",
            ),
            OnboardingStep::Complete => ("SkillBinder", "Your library is ready."),
        };
        let client = self.client.clone();
        let next = match step {
            OnboardingStep::Prerequisites => OnboardingStep::Boundaries,
            OnboardingStep::Boundaries => OnboardingStep::SyncChoice,
            OnboardingStep::SyncChoice => OnboardingStep::Ready,
            OnboardingStep::Ready | OnboardingStep::Complete => OnboardingStep::Complete,
        };
        let back = match step {
            OnboardingStep::Boundaries => Some(OnboardingStep::Prerequisites),
            OnboardingStep::SyncChoice => Some(OnboardingStep::Boundaries),
            OnboardingStep::Ready => Some(OnboardingStep::SyncChoice),
            _ => None,
        };
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w_96()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child("SkillBinder")
                    .child(title)
                    .child(detail)
                    .when(step == OnboardingStep::Prerequisites, |row| {
                        row.child(format!(
                            "Git: {}",
                            self.snapshot
                                .bootstrap
                                .git_version
                                .as_deref()
                                .unwrap_or("Unavailable")
                        ))
                        .child(format!(
                            "Storage: {}",
                            if self.snapshot.bootstrap.storage_writable {
                                "Ready"
                            } else {
                                "Unavailable"
                            }
                        ))
                        .child(
                            Button::new("recheck-prerequisites", "Check again")
                                .disabled(self.busy)
                                .on_click(cx.listener(|view, _, _, cx| {
                                    let client = view.client.clone();
                                    view.run(
                                            async move {
                                                client.bootstrap().await.map(|_| Effect::None)
                                            },
                                            cx,
                                        );
                                })),
                        )
                    })
                    .when_some(back, |row, previous| {
                        row.child(
                            Button::new("onboarding-back", "Back")
                                .disabled(self.busy)
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    let client = view.client.clone();
                                    view.run(
                                        async move {
                                            client
                                                .set_onboarding(previous)
                                                .await
                                                .map(|_| Effect::None)
                                        },
                                        cx,
                                    );
                                })),
                        )
                    })
                    .child(
                        Button::new(
                            "onboarding-next",
                            if step == OnboardingStep::Ready {
                                "Create library"
                            } else {
                                "Continue"
                            },
                        )
                        .primary()
                        .disabled(self.busy || (step == OnboardingStep::Prerequisites && !ready))
                        .on_click(cx.listener(move |view, _, _, cx| {
                            let client = client.clone();
                            view.run(
                                async move {
                                    if next == OnboardingStep::Complete {
                                        client.complete_onboarding().await?;
                                    } else {
                                        client.set_onboarding(next).await?;
                                    }
                                    Ok(Effect::None)
                                },
                                cx,
                            );
                        })),
                    ),
            )
    }
}
