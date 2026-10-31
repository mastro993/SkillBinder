//! Deleting a preset: its skills stay in the library, and deleting the active preset activates
//! the Default preset, so the confirmation states the skills that switch adds and removes.

use super::controls::{Inactive, button};
use super::{Effect, NativeView};
use gpui_kit::component::dialog::Dialog;
use gpui_kit::{Context, ParentElement, Styled, div};
use skillbinder_proto::{DeletePreview, OrganizationChange, PresetSwitch, SkillId};

impl NativeView {
    pub(super) fn delete_preset_dialog(
        &self,
        dialog: Dialog,
        preview: &DeletePreview,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let busy = self.busy;
        let id = preview.preset.id.clone();
        let revision = preview.revision.clone();
        dialog
            .title(self.dialog_title("Delete preset", cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(format!(
                        "Delete {}? Its {} skills stay in the library.",
                        preview.preset.name, preview.affected_skills
                    ))
                    .children(preview.switch.as_ref().map(switch_note)),
            )
            .footer(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button("delete-cancel")
                            .label("Cancel")
                            .inactive(busy)
                            .on_click(
                                cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)),
                            ),
                    )
                    .child(
                        button("delete-confirm")
                            .label("Delete preset")
                            .inactive(busy)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let client = view.client.clone();
                                let change = OrganizationChange::DeletePreset {
                                    id: id.clone(),
                                    revision: revision.clone(),
                                };
                                view.run(
                                    async move {
                                        client
                                            .change_organization(change)
                                            .await
                                            .map(|_| Effect::CloseDialog)
                                    },
                                    cx,
                                );
                            })),
                    ),
            )
    }
}

/// What activating the Default preset in the deleted one's place does for your agents.
fn switch_note(switch: &PresetSwitch) -> String {
    let list = |skills: &[SkillId]| {
        skills
            .iter()
            .map(SkillId::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let changes = match (switch.adds.as_slice(), switch.removes.as_slice()) {
        ([], []) => return "Default becomes the active preset, with the same skills.".to_owned(),
        (adds, []) => format!("adds {}", list(adds)),
        ([], removes) => format!("removes {}", list(removes)),
        (adds, removes) => format!("adds {} and removes {}", list(adds), list(removes)),
    };
    format!("Default becomes the active preset, which {changes} for your agents.")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skills(slugs: &[&str]) -> Vec<SkillId> {
        slugs
            .iter()
            .map(|slug| SkillId((*slug).to_owned()))
            .collect()
    }

    #[test]
    fn the_switch_note_names_what_default_adds_and_removes() {
        let switch = |adds: &[&str], removes: &[&str]| PresetSwitch {
            adds: skills(adds),
            removes: skills(removes),
        };
        assert_eq!(
            switch_note(&switch(&["pdf"], &["docx", "xlsx"])),
            "Default becomes the active preset, which adds pdf and removes docx, xlsx for your \
             agents."
        );
        assert_eq!(
            switch_note(&switch(&[], &["docx"])),
            "Default becomes the active preset, which removes docx for your agents."
        );
        assert_eq!(
            switch_note(&switch(&[], &[])),
            "Default becomes the active preset, with the same skills."
        );
    }
}
