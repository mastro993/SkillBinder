//! The first activation in a workspace that uses no agents yet asks which agents it uses: each
//! detected agent's workspace skill directory once, naming the agents that read it. The chosen
//! directories become the workspace's agents, and the skill is active for all of them.

use std::collections::BTreeSet;

use super::controls::{Inactive, button, checkbox};
use super::library_copies::Reader;
use super::workspace_targets::{DirRow, dir_icon, dir_rows};
use super::{DialogState, Effect, NativeView};
use gpui_kit::component::{
    ActiveTheme, Sizable, button::ButtonVariants, dialog::Dialog, h_flex, v_flex,
};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div, px};
use skillbinder_proto::{SkillId, WorkspaceId, WorkspaceTargets};

impl NativeView {
    pub(super) fn workspace_agents_dialog(
        &self,
        dialog: Dialog,
        (id, skill, chosen): (&WorkspaceId, &SkillId, &BTreeSet<String>),
        cx: &mut Context<Self>,
    ) -> Dialog {
        let (name, rows) = self.workspace(id).map_or_else(
            || ("this workspace".to_owned(), Vec::new()),
            |workspace| {
                let rows = dir_rows(&self.snapshot.agents, workspace, &self.snapshot.home);
                (workspace.name.clone(), rows)
            },
        );
        let lead = if rows.is_empty() {
            "No agents are detected on this device.".to_owned()
        } else {
            format!("Which agents does {name} use? {skill} is copied into their skill directories.")
        };
        let choices: Vec<_> = rows
            .iter()
            .enumerate()
            .map(|(index, row)| self.agent_choice(index, row, chosen, cx))
            .collect();
        let (id, skill, picked) = (id.clone(), skill.clone(), chosen.clone());
        dialog
            .title(self.dialog_title("Choose agents", cx))
            .child(
                v_flex()
                    .gap_3()
                    .child(div().text_size(px(13.)).child(lead))
                    .children(choices),
            )
            .footer(
                h_flex()
                    .gap_2()
                    .child(
                        button("workspace-agents-cancel")
                            .label("Cancel")
                            .inactive(self.busy)
                            .on_click(
                                cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)),
                            ),
                    )
                    .child(
                        button("workspace-agents-activate")
                            .label("Activate")
                            .primary()
                            .inactive(self.busy || chosen.is_empty())
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let client = view.client.clone();
                                let (id, skill) = (id.clone(), skill.clone());
                                let targets = WorkspaceTargets::Dirs(picked.clone());
                                view.run(
                                    async move {
                                        client
                                            .set_workspace_skill(id, skill, Some(targets))
                                            .await
                                            .map(|()| Effect::CloseDialog)
                                    },
                                    cx,
                                );
                            })),
                    ),
            )
    }

    /// One directory's glyph and checkbox, naming its agents, and where the directory is.
    fn agent_choice(
        &self,
        index: usize,
        row: &DirRow,
        chosen: &BTreeSet<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let weak = cx.entity().downgrade();
        let dir = row.dir.clone();
        let detail = match &row.row.reader {
            Reader::Universal(names) => {
                format!("{} · Used by {}", row.row.dir, names.join(", "))
            }
            Reader::Agent(_) => row.row.dir.clone(),
        };
        h_flex()
            .gap_2()
            .items_start()
            .child(dir_icon(row).small())
            .child(
                v_flex()
                    .min_w_0()
                    .gap(px(2.))
                    .child(
                        checkbox(format!("workspace-agent-{index}"))
                            .checked(chosen.contains(&row.dir))
                            .label(row.row.name.clone())
                            .small()
                            .inactive(self.busy)
                            .on_change(move |checked, _, cx| {
                                let dir = dir.clone();
                                let _ = weak.update(cx, |view, cx| {
                                    if let DialogState::WorkspaceAgents { chosen, .. } =
                                        &mut view.dialog
                                    {
                                        if *checked {
                                            chosen.insert(dir);
                                        } else {
                                            chosen.remove(&dir);
                                        }
                                        cx.notify();
                                    }
                                });
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(11.5))
                            .text_color(cx.theme().muted_foreground)
                            .child(detail),
                    ),
            )
            .into_any_element()
    }
}
