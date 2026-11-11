//! A workspace row's trailing controls, its Delete button, and the confirmation that unticking a
//! skill's last folder asks. Both remove the skill from this workspace only, at once, and the
//! toast then offers Undo, as [`super::super::undo`] describes.

use super::super::NativeView;
use super::super::controls::{Inactive, app_icon, button};
use super::super::library_items::SkillItem;
use super::WorkspaceRow;
use gpui_kit::component::{Sizable, button::ButtonVariants, dialog::Dialog, h_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    StatefulInteractiveElement, Styled, Text, prelude::FluentBuilder, px,
};
use skillbinder_proto::{SkillId, WorkspaceId};

impl NativeView {
    /// The review glyph while a copy of the skill needs review, the agents button, and Delete.
    /// Clicks and Enter or Space on them never open the skill's page.
    pub(in crate::native_view) fn workspace_row_end(
        &self,
        item: &SkillItem<'_>,
        row: &WorkspaceRow<'_>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = &item.skill.id;
        let unsettled = item
            .placement
            .map_or(0, |placement| placement.unsettled.len());
        h_flex()
            .id(format!("workspace-row-end-{id}"))
            .flex_none()
            .items_center()
            .gap(px(8.))
            .on_click(|_, _, cx| cx.stop_propagation())
            .on_key_down(|event: &KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    cx.stop_propagation();
                }
            })
            .when(unsettled > 0, |end| {
                end.child(self.unsettled_glyph(id, unsettled, cx))
            })
            .child(self.workspace_agents_button(row.workspace, row.folders, id, cx))
            .child(self.workspace_remove_button(
                &row.workspace.id,
                &row.workspace.name,
                id,
                row.workspace.missing,
                cx,
            ))
            .into_any_element()
    }

    /// The small ghost Delete button, which removes the skill from the workspace at once.
    fn workspace_remove_button(
        &self,
        workspace: &WorkspaceId,
        name: &str,
        skill: &SkillId,
        missing: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let label = format!("Remove from {name}");
        let (workspace, id) = (workspace.clone(), skill.clone());
        button(format!("workspace-remove-{skill}"))
            .ghost()
            .small()
            .icon(app_icon("delete"))
            .tooltip(label.clone())
            .accessibility_label(label)
            .inactive(missing || self.blocked())
            .on_click(cx.listener(move |view, _, _, cx| {
                cx.stop_propagation();
                view.remove_from_workspace(workspace.clone(), id.clone(), cx);
            }))
            .into_any_element()
    }

    /// "Remove <skill> from <workspace>?" with Cancel and Remove, which removes it as Delete does.
    pub(in crate::native_view) fn remove_from_workspace_dialog(
        &self,
        dialog: Dialog,
        workspace: &WorkspaceId,
        skill: &SkillId,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let name = self
            .workspace(workspace)
            .map_or_else(|| "this workspace".to_owned(), |w| w.name.clone());
        let title = Text::new(
            "title".into(),
            format!("Remove {skill} from {name}?").into(),
        );
        let (workspace, skill) = (workspace.clone(), skill.clone());
        dialog.title(self.dialog_heading(title, cx)).footer(
            h_flex()
                .gap_2()
                .child(
                    button("remove-skill-cancel")
                        .label("Cancel")
                        .inactive(self.busy)
                        .on_click(
                            cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)),
                        ),
                )
                .child(
                    button("remove-skill-confirm")
                        .label("Remove")
                        .danger()
                        .inactive(self.blocked())
                        .on_click(cx.listener(move |view, _, window, cx| {
                            view.dismiss_dialog(window, cx);
                            view.remove_from_workspace(workspace.clone(), skill.clone(), cx);
                        })),
                ),
        )
    }
}
