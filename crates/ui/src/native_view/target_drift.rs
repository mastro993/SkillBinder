//! Drift on a target page: what changed since a deployment was written, the remedies the user
//! can pick, and the confirmation before a remedy discards changes made in the target.

use super::controls::{Inactive, button};
use super::{DialogState, Effect, NativeView};
use gpui_kit::component::{ActiveTheme, dialog::Dialog};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div};
use skillbinder_proto::*;

impl NativeView {
    /// The deployment's drift against the library skill in the current snapshot.
    pub(super) fn deployment_drift(&self, deployment: &Deployment) -> Option<Drift> {
        let library = &self.snapshot.library.as_ref()?.library;
        let skill = library
            .skills
            .iter()
            .find(|skill| skill.id == deployment.skill_id)?;
        deployment.drift(&skill.digest)
    }

    /// How many of the target's deployments drifted.
    pub(super) fn drifted(&self, target: &DeploymentTarget) -> usize {
        self.snapshot
            .deployments
            .iter()
            .filter(|deployment| &deployment.target == target)
            .filter(|deployment| self.deployment_drift(deployment).is_some())
            .count()
    }

    /// The drift line and remedy buttons of one deployment, or `None` while it matches.
    pub(super) fn render_drift(
        &self,
        deployment: &Deployment,
        inactive: bool,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let drift = self.deployment_drift(deployment)?;
        let name = self.target_name(&deployment.target);
        let side = match deployment.target {
            DeploymentTarget::Root => "Root",
            DeploymentTarget::Project(_) => "project",
        };
        let (summary, remedies): (String, Vec<(String, Remedy)>) = match drift {
            Drift::ProjectChanged => (
                format!("Changed in {name} since it was added. The library skill is unchanged."),
                vec![
                    ("Restore from library".into(), Remedy::UseLibrary),
                    ("Accept into library".into(), Remedy::UseProject),
                ],
            ),
            Drift::Outdated => (
                "The library skill changed since it was added here.".into(),
                vec![(format!("Update {side}"), Remedy::UseLibrary)],
            ),
            Drift::Conflict => (
                format!("Changed in {name} and in the library since it was added."),
                vec![
                    ("Keep library version".into(), Remedy::UseLibrary),
                    (format!("Keep {side} version"), Remedy::UseProject),
                ],
            ),
            Drift::Missing => (
                format!("A copy is missing from {name}."),
                vec![("Redeploy".into(), Remedy::UseLibrary)],
            ),
        };
        let buttons = remedies.into_iter().map(|(label, remedy)| {
            let shown = deployment.clone();
            button(format!("drift-{remedy:?}-{}", deployment.skill_id))
                .label(label)
                .inactive(inactive)
                .on_click(cx.listener(move |view, _, _, cx| {
                    let shown = shown.clone();
                    // Writing the library over a changed copy discards it, so that asks first.
                    if remedy == Remedy::UseLibrary && drift_discards(drift) {
                        view.dialog = DialogState::DiscardChanges { shown, drift };
                        cx.notify();
                    } else {
                        view.resolve_drift(shown, drift, remedy, cx);
                    }
                }))
        });
        Some(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_color(cx.theme().warning).child(summary))
                .child(div().flex().gap_2().children(buttons))
                .into_any_element(),
        )
    }

    /// Applies `remedy` to the deployment as `shown`; the engine refuses it as stale when the
    /// copies changed since.
    fn resolve_drift(
        &mut self,
        shown: Deployment,
        drift: Drift,
        remedy: Remedy,
        cx: &mut Context<Self>,
    ) {
        let client = self.client.clone();
        self.run(
            async move {
                client
                    .resolve_drift(shown, drift, remedy)
                    .await
                    .map(|()| Effect::CloseDialog)
            },
            cx,
        );
    }

    pub(super) fn discard_changes_dialog(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let DialogState::DiscardChanges { shown, drift } = &self.dialog else {
            return dialog;
        };
        let changed = shown
            .changes
            .iter()
            .filter(|(_, change)| **change == CopyChange::Changed)
            .map(|(path, _)| div().child(path.clone()));
        let (shown, drift) = (shown.clone(), *drift);
        dialog
            .title(self.dialog_title("Discard changes", cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(format!(
                        "{} was changed in {}. Using the library version discards those changes.",
                        self.skill_slug(&shown.skill_id),
                        self.target_name(&shown.target)
                    ))
                    .children(changed),
            )
            .footer(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button("discard-changes-cancel")
                            .label("Cancel")
                            .inactive(self.busy)
                            .on_click(
                                cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)),
                            ),
                    )
                    .child(
                        button("discard-changes-confirm")
                            .label("Discard changes and use library")
                            .inactive(self.busy)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let shown = shown.clone();
                                view.resolve_drift(shown, drift, Remedy::UseLibrary, cx)
                            })),
                    ),
            )
    }

    /// The library slug of `skill`, or its identity once it left the library.
    pub(super) fn skill_slug(&self, skill: &SkillId) -> String {
        self.snapshot
            .library
            .as_ref()
            .and_then(|library| library.library.skills.iter().find(|s| &s.id == skill))
            .map_or_else(|| skill.to_string(), |skill| skill.slug.clone())
    }
}

/// Whether using the library version discards a copy changed in the target.
fn drift_discards(drift: Drift) -> bool {
    matches!(drift, Drift::ProjectChanged | Drift::Conflict)
}
