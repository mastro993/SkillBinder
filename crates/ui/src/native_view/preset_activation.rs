//! The active preset and the skills it configures: selecting a preset activates it, which writes
//! and removes your agents' copies, and each activation toggle changes the active preset's skills,
//! or on a workspace's page the workspace's, and saves at once.

use super::NativeView;
use super::controls::{Inactive, checkbox};
use gpui_kit::component::Sizable;
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div,
};
use skillbinder_proto::{LibrarySnapshot, OrganizationChange, Preset, PresetId, SkillId};

impl NativeView {
    /// The preset whose configuration the app edits: one just clicked while its activation is
    /// saved, otherwise the library's active preset.
    pub(super) fn active_preset<'a>(&self, library: &'a LibrarySnapshot) -> Option<&'a Preset> {
        resolved_active(
            &library.library.presets,
            self.active_preset
                .as_ref()
                .unwrap_or(&library.library.active_preset),
        )
    }

    /// The identity of the active preset, resolved against the current library.
    pub(super) fn active_preset_id(&self) -> Option<PresetId> {
        self.snapshot
            .library
            .as_ref()
            .and_then(|library| self.active_preset(library))
            .map(|preset| preset.id.clone())
    }

    /// The active preset's display name, as the sidebar shows it.
    pub(super) fn active_preset_name(&self) -> String {
        self.snapshot
            .library
            .as_ref()
            .and_then(|library| self.active_preset(library))
            .map_or_else(|| "Default".to_owned(), |preset| preset.name.clone())
    }

    /// Activates preset `id`, showing it active at once while the engine saves the choice and
    /// writes your agents' copies to match it.
    pub(super) fn set_active_preset(&mut self, id: PresetId, cx: &mut Context<Self>) {
        if self.busy || self.active_preset_id().as_ref() == Some(&id) {
            return;
        }
        self.active_preset = Some(id.clone());
        self.change_organization(OrganizationChange::ActivatePreset { id }, cx);
    }

    /// Where activation toggles act: the open workspace, or else the active preset.
    pub(super) fn scope_name(&self) -> String {
        self.scoped_workspace().map_or_else(
            || self.active_preset_name(),
            |workspace| workspace.name.clone(),
        )
    }

    /// Whether `skill` is active in the open workspace, or else in the active preset.
    pub(super) fn skill_active(&self, skill: &SkillId) -> bool {
        if let Some(workspace) = self.scoped_workspace() {
            return workspace.skills.contains_key(skill);
        }
        let Some(active) = self.active_preset_id() else {
            return false;
        };
        self.snapshot
            .library
            .as_ref()
            .and_then(|library| library.library.skills.iter().find(|held| &held.id == skill))
            .is_some_and(|held| held.presets.contains_key(&active))
    }

    /// A one-click switch for a library skill in the open workspace or the active preset. Its
    /// changes save at once.
    pub(super) fn activation_toggle(&self, skill: &SkillId, cx: &mut Context<Self>) -> AnyElement {
        let checked = self.skill_active(skill);
        let scope = self.scope_name();
        let busy = self.busy;
        let weak = cx.entity().downgrade();
        let id = skill.clone();
        let tooltip = format!("Active in {scope}");
        div()
            .id(format!("activation-{skill}"))
            .flex_shrink_0()
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                checkbox(format!("activation-checkbox-{skill}"))
                    .checked(checked)
                    .label("Active")
                    .small()
                    .inactive(busy)
                    .tooltip(tooltip)
                    .on_change(move |checked, _, cx| {
                        let id = id.clone();
                        let _ = weak.update(cx, |view, cx| {
                            view.toggle_activation(id.clone(), *checked, cx);
                        });
                    }),
            )
            .into_any_element()
    }

    /// Adds `skill` to the open workspace or the active preset, or removes it, saving the change
    /// at once.
    pub(super) fn toggle_activation(
        &mut self,
        skill: SkillId,
        active: bool,
        cx: &mut Context<Self>,
    ) {
        if let Some(workspace) = self.scoped_workspace().cloned() {
            return self.toggle_workspace_skill(&workspace, skill, active, cx);
        }
        let Some(id) = self.active_preset_id() else {
            return;
        };
        let change = if active {
            OrganizationChange::AddToPreset {
                id,
                skills: vec![skill],
            }
        } else {
            OrganizationChange::RemoveFromPreset {
                id,
                skills: vec![skill],
            }
        };
        self.change_organization(change, cx);
    }
}

/// The chosen preset while it exists, otherwise the Default preset.
fn resolved_active<'a>(presets: &'a [Preset], chosen: &PresetId) -> Option<&'a Preset> {
    presets
        .iter()
        .find(|preset| &preset.id == chosen)
        .or_else(|| presets.iter().find(|preset| preset.is_default()))
        .or_else(|| presets.first())
}

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_proto::PresetIcon;

    fn preset(id: &str, name: &str, position: u32) -> Preset {
        Preset {
            id: PresetId(id.to_owned()),
            name: name.to_owned(),
            position,
            color: None,
            icon: PresetIcon::Layers,
        }
    }

    #[test]
    fn the_chosen_preset_wins_while_it_exists_and_default_returns_otherwise() {
        let presets = [
            preset("default", "Default", 0),
            preset("review", "Review", 1),
        ];
        let chosen = |id: &str| PresetId(id.to_owned());
        assert_eq!(
            resolved_active(&presets, &chosen("review")).map(|p| p.name.as_str()),
            Some("Review")
        );
        assert_eq!(
            resolved_active(&presets, &chosen("gone")).map(|p| p.name.as_str()),
            Some("Default")
        );
    }
}
