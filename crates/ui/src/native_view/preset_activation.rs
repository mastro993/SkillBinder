//! The active preset and the skills it configures: selecting a preset activates it, which writes
//! and removes your agents' copies, and each activation toggle changes the active preset's skills
//! and saves at once.

use super::NativeView;
use super::progress_strip::{Activity, Landing};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div,
};
use skillbinder_client::Client;
use skillbinder_design::Checkbox;
use skillbinder_proto::{
    AppResult, AppSnapshot, LibrarySnapshot, OrganizationChange, Preset, PresetId, SkillId,
};
use std::future::Future;

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
    /// writes your agents' copies to match it; the progress strip blocks the app meanwhile.
    pub(crate) fn set_active_preset(&mut self, id: PresetId, cx: &mut Context<Self>) {
        if self.active_preset_id().as_ref() == Some(&id) {
            return;
        }
        let Some(activity) = switching(&self.snapshot, &id) else {
            return;
        };
        let operation = activate(self.client.clone(), id.clone());
        self.block_change(activity, Some(id), operation, cx);
    }

    /// Runs `operation`, a blocking change, under the strip; `preset`, when it activates one,
    /// shows active at once. Refused while the app is blocked.
    pub(crate) fn block_change<F>(
        &mut self,
        activity: Activity,
        preset: Option<PresetId>,
        operation: F,
        cx: &mut Context<Self>,
    ) where
        F: Future<Output = AppResult<Landing>> + Send + 'static,
    {
        // Checked before the optimistic preset is set, which `block` could not undo.
        if self.blocked() {
            return;
        }
        if preset.is_some() {
            self.active_preset = preset;
        }
        self.block(activity, operation, cx);
    }

    /// Whether the active preset holds `skill`.
    pub(super) fn skill_active(&self, skill: &SkillId) -> bool {
        let Some(active) = self.active_preset_id() else {
            return false;
        };
        self.snapshot
            .library
            .as_ref()
            .and_then(|library| library.library.skills.iter().find(|held| &held.id == skill))
            .is_some_and(|held| held.presets.contains_key(&active))
    }

    /// A one-click switch for a library skill in the active preset. Its changes save at once.
    pub(super) fn activation_toggle(&self, skill: &SkillId, cx: &mut Context<Self>) -> AnyElement {
        let checked = self.skill_active(skill);
        let scope = self.active_preset_name();
        let busy = self.working();
        let weak = cx.entity().downgrade();
        let id = skill.clone();
        let tooltip = format!("Active in {scope}");
        div()
            .id(format!("activation-{skill}"))
            .flex_shrink_0()
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                Checkbox::new(format!("activation-checkbox-{skill}"))
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

    /// Adds `skill` to the active preset, or removes it, saving the change at once.
    pub(super) fn toggle_activation(
        &mut self,
        skill: SkillId,
        active: bool,
        cx: &mut Context<Self>,
    ) {
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

/// Switching to preset `id`, while it exists in `snapshot`.
pub(crate) fn switching(snapshot: &AppSnapshot, id: &PresetId) -> Option<Activity> {
    let library = snapshot.library.as_ref()?;
    let preset = library.library.presets.iter().find(|p| &p.id == id)?;
    Some(Activity::Switching {
        preset: preset.name.clone(),
    })
}

/// The change that activates preset `id`, writing and removing your agents' copies to match it.
pub(crate) async fn activate(client: Client, id: PresetId) -> AppResult<Landing> {
    let change = OrganizationChange::ActivatePreset { id };
    client.change_organization(change).await?;
    Ok(Landing::default())
}

/// The chosen preset while it exists, otherwise the Default preset.
pub(crate) fn resolved_active<'a>(presets: &'a [Preset], chosen: &PresetId) -> Option<&'a Preset> {
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
