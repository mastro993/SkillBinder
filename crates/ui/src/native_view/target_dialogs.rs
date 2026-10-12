//! Choosing library skills to add to a target, with its agents when it has none yet, and
//! confirming the removal of changed copies.

use std::collections::BTreeSet;

use super::controls::{Inactive, button};
use super::target_page::agent_checklist;
use super::{DialogState, Effect, NativeView};
use gpui_kit::component::{button::ButtonVariants, checkbox::Checkbox, dialog::Dialog};
use gpui_kit::{
    Context, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled, div, px,
};

/// Lists taller than this scroll inside the dialog.
const LIST_HEIGHT: f32 = 240.0;

impl NativeView {
    pub(super) fn add_skills_dialog(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let DialogState::AddSkills {
            target,
            skills,
            agents,
        } = &self.dialog
        else {
            return dialog;
        };
        let deployed: BTreeSet<_> = self
            .snapshot
            .deployments
            .iter()
            .filter(|deployment| &deployment.target == target)
            .map(|deployment| &deployment.skill_id)
            .collect();
        let library = self
            .snapshot
            .library
            .as_ref()
            .map_or(&[][..], |library| &library.library.skills[..]);
        let weak = cx.entity().downgrade();
        let choices = library
            .iter()
            .filter(|skill| !deployed.contains(&skill.id))
            .map(|skill| {
                let (weak, id) = (weak.clone(), skill.id.clone());
                Checkbox::new(format!("add-skill-{id}"))
                    .checked(skills.contains(&skill.id))
                    .label(skill.slug.clone())
                    .on_change(move |checked, _, cx| {
                        let _ = weak.update(cx, |view, cx| {
                            if let DialogState::AddSkills { skills, .. } = &mut view.dialog {
                                if *checked {
                                    skills.insert(id.clone());
                                } else {
                                    skills.remove(&id);
                                }
                            }
                            cx.notify();
                        });
                    })
            });
        let mut body = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(format!(
                "Choose library skills to copy into {}.",
                self.target_name(target)
            ))
            .child(scrolling("add-skills-list").children(choices));
        if let Some(agents) = agents {
            body = body
                .child(
                    "This target has no agent skill directory yet. Choose the agents whose skill directories receive the copies.",
                )
                .child(scrolling("add-skills-agents").child(agent_checklist(
                    "add-skills-agent",
                    self.agent_choices(target),
                    agents,
                    cx,
                    |view| match &mut view.dialog {
                        DialogState::AddSkills { agents, .. } => agents.as_mut(),
                        _ => None,
                    },
                )));
        }
        let ready = !skills.is_empty() && agents.as_ref().is_none_or(|agents| !agents.is_empty());
        let (target, chosen, agents) = (
            target.clone(),
            skills.iter().cloned().collect::<Vec<_>>(),
            agents.clone(),
        );
        dialog
            .title(self.dialog_title("Add skills", cx))
            .child(body)
            .footer(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button("add-skills-cancel")
                            .label("Cancel")
                            .inactive(self.busy)
                            .on_click(
                                cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)),
                            ),
                    )
                    .child(
                        button("add-skills-confirm")
                            .label(format!("Add {} skills", chosen.len()))
                            .primary()
                            .inactive(self.busy || !ready)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let client = view.client.clone();
                                let (target, skills) = (target.clone(), chosen.clone());
                                let agents = agents.clone();
                                view.run(
                                    async move {
                                        if let Some(agents) = agents {
                                            let agents = agents.into_iter().collect();
                                            client
                                                .set_target_agents(target.clone(), agents)
                                                .await?;
                                        }
                                        client
                                            .add_deployments(target, skills)
                                            .await
                                            .map(|()| Effect::CloseDialog)
                                    },
                                    cx,
                                );
                            })),
                    ),
            )
    }

    pub(super) fn remove_changed_dialog(&self, dialog: Dialog, cx: &mut Context<Self>) -> Dialog {
        let DialogState::RemoveChanged {
            target,
            skill,
            paths,
        } = &self.dialog
        else {
            return dialog;
        };
        let slug = self
            .snapshot
            .library
            .as_ref()
            .and_then(|library| library.library.skills.iter().find(|s| &s.id == skill))
            .map_or_else(|| skill.to_string(), |skill| skill.slug.clone());
        let (target, skill, confirmed) = (target.clone(), skill.clone(), paths.clone());
        dialog
            .title(self.dialog_title("Remove changed copy", cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(format!(
                        "{slug} was changed in {} since it was added. Removing it discards those changes. The library skill stays.",
                        self.target_name(&target)
                    ))
                    .children(paths.iter().map(|path| div().child(path.clone()))),
            )
            .footer(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button("remove-changed-cancel")
                            .label("Cancel")
                            .inactive(self.busy)
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.dismiss_dialog(window, cx)
                            })),
                    )
                    .child(
                        button("remove-changed-confirm")
                            .label("Discard changes and remove")
                            .inactive(self.busy)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.remove_deployment(target.clone(), skill.clone(), confirmed.clone(), cx)
                            })),
                    ),
            )
    }
}

fn scrolling(id: &'static str) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(id)
        .max_h(px(LIST_HEIGHT))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .gap_1()
}
