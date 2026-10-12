//! The page of Root or a project: its target agents, which can be edited, and its deployments,
//! which can be added and removed.

use std::collections::BTreeSet;

use super::controls::{Inactive, button};
use super::sidebar_projects::ROOT;
use super::{DialogState, Effect, NativeView, Route};
use gpui_kit::component::{ActiveTheme, checkbox::Checkbox};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div};
use skillbinder_proto::*;

impl NativeView {
    pub(super) fn render_target(
        &self,
        target: &DeploymentTarget,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let project = match target {
            DeploymentTarget::Root => None,
            DeploymentTarget::Project(id) => self.snapshot.projects.iter().find(|p| &p.id == id),
        };
        let location = project.map_or_else(
            || "Global agent skill locations under your home".to_owned(),
            |project| project.path.clone(),
        );
        let missing = project.is_some_and(|project| project.missing);
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(self.target_name(target))
            .child(location)
            .children(missing.then_some(
                "The project folder is missing. Its deployments are kept until it returns.",
            ))
            .child(self.render_target_agents(target, cx))
            .child(self.render_deployments(target, missing, cx))
            .into_any_element()
    }

    /// Root, or the project's name.
    pub(super) fn target_name(&self, target: &DeploymentTarget) -> String {
        match target {
            DeploymentTarget::Root => ROOT.to_owned(),
            DeploymentTarget::Project(id) => self
                .snapshot
                .projects
                .iter()
                .find(|project| &project.id == id)
                .map_or_else(|| "Project".to_owned(), |project| project.name.clone()),
        }
    }

    pub(super) fn target_agents(&self, target: &DeploymentTarget) -> &[String] {
        match target {
            DeploymentTarget::Root => &self.snapshot.root_agents,
            DeploymentTarget::Project(id) => self
                .snapshot
                .projects
                .iter()
                .find(|project| &project.id == id)
                .map_or(&[], |project| &project.agents),
        }
    }

    /// Agents the target can write to, each with the skill directory it receives.
    pub(super) fn agent_choices(&self, target: &DeploymentTarget) -> Vec<(String, String)> {
        self.snapshot
            .agents
            .iter()
            .filter_map(|agent| {
                let directory = match target {
                    DeploymentTarget::Root => agent.global_skills_dir.clone()?,
                    DeploymentTarget::Project(_) => agent.project_skills_dir.clone(),
                };
                Some((
                    agent.id.clone(),
                    format!("{} · {directory}", agent.display_name),
                ))
            })
            .collect()
    }

    fn render_target_agents(
        &self,
        target: &DeploymentTarget,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let current = self.target_agents(target);
        let section = div().flex().flex_col().gap_2().child("Target agents");
        let Some(selected) = &self.agent_edit else {
            let choices = self.agent_choices(target);
            let listed = choices
                .into_iter()
                .filter(|(id, _)| current.contains(id))
                .map(|(_, label)| div().child(label));
            return section
                .children(current.is_empty().then_some(
                    "No target agents yet. Adding skills asks which agents receive them.",
                ))
                .children(listed)
                .child(
                    button("target-agents-edit")
                        .label("Edit agents")
                        .inactive(self.busy)
                        .on_click(cx.listener(|view, _, _, cx| {
                            if let Route::Target(target) = &view.route {
                                let agents = view.target_agents(&target.clone()).iter().cloned();
                                view.agent_edit = Some(agents.collect());
                                cx.notify();
                            }
                        })),
                )
                .into_any_element();
        };
        let target = target.clone();
        section
            .child(agent_checklist(
                "target-agent",
                self.agent_choices(&target),
                selected,
                cx,
                |view| view.agent_edit.as_mut(),
            ))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button("target-agents-cancel")
                            .label("Cancel")
                            .inactive(self.busy)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.agent_edit = None;
                                cx.notify();
                            })),
                    )
                    .child(
                        button("target-agents-save")
                            .label("Save agents")
                            .inactive(self.busy)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let Some(agents) = view.agent_edit.take() else {
                                    return;
                                };
                                let client = view.client.clone();
                                let target = target.clone();
                                let agents = agents.into_iter().collect();
                                view.run(
                                    async move {
                                        client
                                            .set_target_agents(target, agents)
                                            .await
                                            .map(|()| Effect::None)
                                    },
                                    cx,
                                );
                            })),
                    ),
            )
            .into_any_element()
    }

    fn render_deployments(
        &self,
        target: &DeploymentTarget,
        missing: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skills = self
            .snapshot
            .library
            .as_ref()
            .map_or(&[][..], |library| &library.library.skills[..]);
        let deployments: Vec<_> = self
            .snapshot
            .deployments
            .iter()
            .filter(|deployment| &deployment.target == target)
            .collect();
        let addable = skills.len() > deployments.len();
        let add_target = target.clone();
        let mut section = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(format!("{} deployments", deployments.len()))
            .child(
                button("target-add-skills")
                    .label("Add skills")
                    .inactive(self.busy || missing || !addable)
                    .on_click(cx.listener(move |view, _, _, cx| {
                        let agents = view
                            .target_agents(&add_target)
                            .is_empty()
                            .then(BTreeSet::new);
                        view.dialog = DialogState::AddSkills {
                            target: add_target.clone(),
                            skills: BTreeSet::new(),
                            agents,
                        };
                        cx.notify();
                    })),
            );
        for deployment in deployments {
            let slug = skills
                .iter()
                .find(|skill| skill.id == deployment.skill_id)
                .map_or_else(
                    || deployment.skill_id.to_string(),
                    |skill| skill.slug.clone(),
                );
            let (target, skill) = (target.clone(), deployment.skill_id.clone());
            section = section.child(
                div()
                    .p_3()
                    .border_1()
                    .border_color(cx.theme().colors.border)
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(slug)
                    .children(
                        deployment
                            .paths
                            .iter()
                            .map(|path| div().child(path.clone())),
                    )
                    .child(
                        button(format!("deployment-remove-{}", deployment.skill_id))
                            .label("Remove")
                            .inactive(self.busy || missing)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.remove_deployment(
                                    target.clone(),
                                    skill.clone(),
                                    Vec::new(),
                                    cx,
                                )
                            })),
                    ),
            );
        }
        section.into_any_element()
    }

    /// Removes a deployment. A clean copy is deleted at once; changed copies ask first.
    pub(super) fn remove_deployment(
        &mut self,
        target: DeploymentTarget,
        skill: SkillId,
        discard: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        let client = self.client.clone();
        self.run(
            async move {
                let removal = client
                    .remove_deployment(target.clone(), skill.clone(), discard)
                    .await?;
                Ok(match removal {
                    Removal::Removed => Effect::CloseDialog,
                    Removal::Drifted { paths } => Effect::Dialog(DialogState::RemoveChanged {
                        target,
                        skill,
                        paths,
                    }),
                })
            },
            cx,
        );
    }
}

/// One checkbox per agent; `selection` finds the set the checkboxes edit.
pub(super) fn agent_checklist(
    id: &'static str,
    choices: Vec<(String, String)>,
    selected: &BTreeSet<String>,
    cx: &mut Context<NativeView>,
    selection: fn(&mut NativeView) -> Option<&mut BTreeSet<String>>,
) -> AnyElement {
    let weak = cx.entity().downgrade();
    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(choices.into_iter().map(|(agent, label)| {
            let weak = weak.clone();
            Checkbox::new(format!("{id}-{agent}"))
                .checked(selected.contains(&agent))
                .label(label)
                .on_change(move |checked, _, cx| {
                    let _ = weak.update(cx, |view, cx| {
                        if let Some(set) = selection(view) {
                            if *checked {
                                set.insert(agent.clone());
                            } else {
                                set.remove(&agent);
                            }
                        }
                        cx.notify();
                    });
                })
        }))
        .into_any_element()
}
