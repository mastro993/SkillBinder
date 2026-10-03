use crate::{components::*, state::*, workspace::Workspace};
use gpui::{prelude::*, *};
use gpui_component::{Disableable, button::ButtonVariants};
use skillbinder_app::{actions, models::*};

impl Workspace {
    pub(crate) fn onboarding(&mut self, _: &mut Window, cx: &mut Context<Self>) -> Div {
        let mut content = column().w_full().max_w(px(880.)).mx_auto();
        let Some(data) = self.state.bootstrap.as_ref() else {
            return content
                .child(heading(
                    "Check your setup",
                    "Preparing SkillBinder on this device.",
                    cx,
                ))
                .child(loading(cx))
                .child(
                    button("retry-bootstrap", "Retry")
                        .disabled(self.state.pending.contains(&Operation::Bootstrap))
                        .on_click(cx.listener(|s, _, w, cx| s.load_bootstrap(w, cx))),
                );
        };
        if data.library_state == LibraryState::RecoveryRequired {
            return content.child(heading("Library needs recovery", data.recovery_summary.clone().unwrap_or_default(), cx))
                .child(notice("Repair the local library before making changes. Your existing files are preserved.", cx))
                .child(button("recheck-recovery", "Recheck").on_click(cx.listener(|s, _, w, cx| s.load_bootstrap(w, cx))))
                .child(button("recovery-logs", "Open log folder").on_click(cx.listener(|s, _, w, cx| s.reveal_logs(w, cx))));
        }
        let step = onboarding_step(data);
        let (index, title, detail): (usize, _, _) = match step {
            OnboardingStep::Prerequisites => (
                0,
                "Check your setup",
                "Required local tools must be ready before any library is created.",
            ),
            OnboardingStep::Boundaries => (
                1,
                "Know what SkillBinder owns",
                "Clear lines keep source skills, managed content, and device state safe.",
            ),
            OnboardingStep::SyncChoice => (
                2,
                "Choose how to begin",
                "Remote sync is optional. Start offline and connect one later.",
            ),
            OnboardingStep::Ready => (
                3,
                "Create your library",
                "One last review before SkillBinder writes durable state.",
            ),
        };
        content = content
            .child(muted(format!("LOCAL SETUP · {} of 4", index + 1), cx))
            .child(heading(title, detail, cx));
        content = match step {
            OnboardingStep::Prerequisites => content
                .children([&data.git.prerequisite, &data.storage].into_iter().map(|status| {
                    panel(cx).child(status.summary.clone()).child(muted(status.detail.clone(), cx))
                        .when_some(status.repair_instruction.clone(), |el, repair| el.child(notice(repair, cx)))
                }))
                .child(button("recheck", "Recheck").disabled(self.state.pending.contains(&Operation::Bootstrap))
                    .on_click(cx.listener(|s, _, w, cx| s.load_bootstrap(w, cx)))),
            OnboardingStep::Boundaries => content.children([
                ("Managed copies", "Imports create canonical copies. Original skill folders stay unchanged."),
                ("Portable library", "Canonical skills, portable IDs, organization, and history move together in the Git library."),
                ("Machine-local state", "Device paths, settings, credentials, and deployment state stay on this machine and outside Git."),
                ("Bounded discovery", "Known agent locations and known skill folders in your project roots."),
                ("Local Git history", "Every library has version history through your supported system Git."),
                ("No account or token", "SkillBinder has no hosted account and stores no Git credentials."),
                ("Explicit sync", "Remote sync stays optional and never runs in the background."),
            ].into_iter().map(|(title, detail)| panel(cx).child(title).child(muted(detail, cx)))),
            OnboardingStep::SyncChoice => content.child(panel(cx).child("Start local-only · Recommended")
                .child(muted("Create a private local library now. Add an existing Git remote later from Git sync.", cx))),
            OnboardingStep::Ready => content.child(panel(cx).child("Ready for a local library")
                .child(muted("SkillBinder will initialize a Git-backed library. No network request, account, runtime, or remote is required.", cx))
                .child("Canonical skill content stays under skills/.")
                .child("Portable metadata stays in .skillbinder.json.")
                .child("Machine paths and settings stay outside Git.")),
        };
        let ready = prerequisites_ready(data);
        content.child(
            row()
                .justify_between()
                .child(
                    button("onboarding-back", "Back")
                        .disabled(index == 0 || self.busy())
                        .on_click(cx.listener(move |s, _, w, cx| {
                            let step = [
                                OnboardingStep::Prerequisites,
                                OnboardingStep::Boundaries,
                                OnboardingStep::SyncChoice,
                            ][index.saturating_sub(1)]
                            .clone();
                            s.save_step(step, w, cx);
                        })),
                )
                .child(
                    button(
                        "onboarding-next",
                        if index == 3 {
                            "Create local library"
                        } else {
                            "Continue"
                        },
                    )
                    .primary()
                    .disabled(
                        !ready || self.busy() || self.state.pending.contains(&Operation::Bootstrap),
                    )
                    .on_click(cx.listener(move |s, _, w, cx| {
                        if index == 3 {
                            s.call(
                                Operation::CompleteSetup,
                                |s| {
                                    actions::onboarding::onboarding_complete_local(&s).into_result()
                                },
                                |s, _, w, cx| s.load_bootstrap(w, cx),
                                w,
                                cx,
                            );
                        } else {
                            let step = [
                                OnboardingStep::Boundaries,
                                OnboardingStep::SyncChoice,
                                OnboardingStep::Ready,
                            ][index]
                                .clone();
                            s.save_step(step, w, cx);
                        }
                    })),
                ),
        )
    }

    fn save_step(&mut self, step: OnboardingStep, window: &mut Window, cx: &mut Context<Self>) {
        self.call(
            Operation::SetupStep,
            move |s| {
                actions::onboarding::onboarding_progress_update(
                    &s,
                    UpdateOnboardingProgressRequest { step },
                )
                .into_result()
            },
            |s, _, w, cx| s.load_bootstrap(w, cx),
            window,
            cx,
        );
    }
}
