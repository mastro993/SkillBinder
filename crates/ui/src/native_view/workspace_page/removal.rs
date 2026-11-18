//! A workspace row's trailing controls, its Delete button, and the confirmation that unticking a
//! skill's last folder asks. Both remove the skill from this workspace only, at once, and the
//! toast then offers Undo, as [`super::super::undo`] describes.

use super::super::NativeView;
use super::super::library_items::SkillItem;
use super::WorkspaceRow;
use gpui_kit::component::dialog::Dialog;
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, prelude::FluentBuilder};
use skillbinder_design::{Button, DialogFooter, DialogHeader, DialogSize, icon, row_end};
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
        row_end(format!("workspace-row-end-{id}"))
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
        let (workspace, id) = (workspace.clone(), skill.clone());
        remove_button(format!("workspace-remove-{skill}"), name)
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
        let header = DialogHeader::new(format!("Remove {skill} from {name}?"));
        let (workspace, skill) = (workspace.clone(), skill.clone());
        let remove = Button::new("remove-skill-confirm")
            .label("Remove")
            .danger()
            .inactive(self.blocked())
            .on_click(cx.listener(move |view, _, window, cx| {
                view.dismiss_dialog(window, cx);
                view.remove_from_workspace(workspace.clone(), skill.clone(), cx);
            }));
        let footer = DialogFooter::new()
            .secondary(self.dialog_cancel("remove-skill-cancel", cx))
            .primary(remove);
        self.dialog_frame(DialogSize::Standard, header, cx)
            .footer(footer)
            .apply(dialog, cx)
    }
}

/// The small ghost Delete button `id`, which removes a skill from the place `name`.
pub(in crate::native_view) fn remove_button(id: String, name: &str) -> Button {
    let label = format!("Remove from {name}");
    Button::new(id)
        .ghost()
        .small()
        .icon(icon("delete"))
        .tooltip(label.clone())
        .accessibility_label(label)
}
