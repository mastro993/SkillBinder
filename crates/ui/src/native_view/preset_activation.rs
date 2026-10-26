//! The active preset and the skills it configures: selecting a preset loads it, and each
//! activation toggle changes the active preset's skills and saves at once.

use super::NativeView;
use super::controls::{Inactive, checkbox};
use gpui_kit::component::Sizable;
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div,
};
use skillbinder_proto::{LibrarySnapshot, OrganizationChange, Preset, PresetId, SkillId};

impl NativeView {
    /// The preset whose configuration the app edits: the chosen one while it still exists,
    /// otherwise the Default preset every library starts with.
    pub(super) fn active_preset<'a>(&self, library: &'a LibrarySnapshot) -> Option<&'a Preset> {
        resolved_active(&library.library.presets, self.active_preset.as_ref())
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

    /// Changes the active preset and persists the choice with the other local preferences.
    pub(super) fn set_active_preset(&mut self, id: Option<PresetId>, cx: &mut Context<Self>) {
        if self.active_preset != id {
            self.active_preset = id;
            self.persist_preferences(cx);
            cx.notify();
        }
    }

    /// Whether `skill` is active in the active preset.
    pub(super) fn skill_active(&self, skill: &SkillId) -> bool {
        let Some(active) = self.active_preset_id() else {
            return false;
        };
        self.snapshot
            .library
            .as_ref()
            .and_then(|library| library.library.skills.iter().find(|held| &held.id == skill))
            .is_some_and(|held| held.preset_ids.contains(&active))
    }

    /// A one-click switch for a library skill in the active preset. Its changes save at once.
    pub(super) fn activation_toggle(&self, skill: &SkillId, cx: &mut Context<Self>) -> AnyElement {
        let checked = self.skill_active(skill);
        let preset = self.active_preset_name();
        let busy = self.busy;
        let weak = cx.entity().downgrade();
        let id = skill.clone();
        let tooltip = format!("Active in {preset}");
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

    fn toggle_activation(&mut self, skill: SkillId, active: bool, cx: &mut Context<Self>) {
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
fn resolved_active<'a>(presets: &'a [Preset], chosen: Option<&PresetId>) -> Option<&'a Preset> {
    chosen
        .and_then(|id| presets.iter().find(|preset| &preset.id == id))
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
        let chosen = |id: &str| Some(PresetId(id.to_owned()));
        assert_eq!(
            resolved_active(&presets, chosen("review").as_ref()).map(|p| p.name.as_str()),
            Some("Review")
        );
        assert_eq!(
            resolved_active(&presets, chosen("gone").as_ref()).map(|p| p.name.as_str()),
            Some("Default")
        );
        assert_eq!(
            resolved_active(&presets, None).map(|p| p.name.as_str()),
            Some("Default")
        );
    }
}
