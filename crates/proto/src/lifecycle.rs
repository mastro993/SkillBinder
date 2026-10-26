use crate::{
    ImportOutcome, LibrarySnapshot, PresetId, ScanSnapshot, SkillId, SkillUpdate, SyncSnapshot,
    Workspace,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Ordered, resumable onboarding steps.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum OnboardingStep {
    /// Git and writable storage verification.
    #[default]
    Prerequisites,
    /// Explain source and managed-library boundaries.
    Boundaries,
    /// Offer local-only setup; synchronization remains optional.
    SyncChoice,
    /// Final creation confirmation.
    Ready,
    /// Atomic local library initialization completed.
    Complete,
}
/// Bootstrap result used before enabling product screens.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bootstrap {
    /// Last durable onboarding step.
    pub step: OnboardingStep,
    /// Trusted Git executable version, if supported.
    pub git_version: Option<String>,
    /// Whether application data can be written.
    pub storage_writable: bool,
    /// App-owned managed library location.
    pub library_path: String,
    /// App-owned diagnostics location.
    pub logs_path: String,
}
/// Machine-local preferences.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preferences {
    /// Sidebar width in logical pixels, clamped to 192 through 400.
    pub sidebar_width: f32,
    /// Whether the sidebar is hidden.
    #[serde(default)]
    pub sidebar_collapsed: bool,
    /// Color scheme; preferences stored before it existed follow the system.
    #[serde(default)]
    pub appearance: Appearance,
    /// Sidebar sections whose rows are hidden; preferences stored before it existed fold none.
    #[serde(default)]
    pub folded_sections: BTreeSet<SidebarSection>,
    /// How Library lays out skills; preferences stored before it existed show a list.
    #[serde(default)]
    pub library_view: LibraryView,
    /// The preset whose skills the app activates; absent starts with the Default preset.
    #[serde(default)]
    pub active_preset: Option<PresetId>,
}
/// Library layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryView {
    /// One row per skill.
    #[default]
    List,
    /// Cards in columns that follow the window width.
    Grid,
}
/// A foldable sidebar section.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidebarSection {
    /// User-ordered presets; preferences saved before presets fold them as `folders`.
    #[serde(alias = "folders")]
    Presets,
    /// The added workspaces.
    Workspaces,
    /// Detected agents, alphabetically.
    Agents,
}
/// Color scheme preference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    /// Follows the window appearance.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}
impl Appearance {
    /// Preference after a two-state toggle, given whether the system is currently dark.
    ///
    /// The toggle shows the opposite of what is visible. Landing on the system appearance
    /// clears the override, so the toggle can always return to following the system; any
    /// other result is stored as an explicit override. Only user interaction calls this, so
    /// an override that later matches the system is kept.
    pub fn toggled(self, system_dark: bool) -> Self {
        let dark = match self {
            Self::System => !system_dark,
            Self::Light => true,
            Self::Dark => false,
        };
        if dark == system_dark {
            Self::System
        } else if dark {
            Self::Dark
        } else {
            Self::Light
        }
    }
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            sidebar_width: 232.0,
            sidebar_collapsed: false,
            appearance: Appearance::System,
            folded_sections: BTreeSet::new(),
            library_view: LibraryView::List,
            active_preset: None,
        }
    }
}
/// One immutable projection published by the client.
#[derive(Clone, Debug)]
pub struct AppSnapshot {
    /// Monotonic client request generation.
    pub generation: u64,
    /// Most recent background refresh failure, cleared by a successful refresh.
    pub background_error: Option<crate::AppError>,
    /// Boot and prerequisite state.
    pub bootstrap: Bootstrap,
    /// Library if onboarding has completed.
    pub library: Option<LibrarySnapshot>,
    /// Home folder that holds the agents' global skill directories.
    pub home: String,
    /// Registry agents, by display name.
    pub agents: Vec<crate::AgentOption>,
    /// Added workspaces in user order.
    pub workspaces: Vec<Workspace>,
    /// Skills the last completed scan found.
    pub scan: Option<ScanSnapshot>,
    /// A scan is running; `scan` keeps the previous results until it completes.
    pub scanning: bool,
    /// Update state of the library skills whose source was checked for their current content.
    pub updates: BTreeMap<SkillId, SkillUpdate>,
    /// Git connection and status.
    pub sync: SyncSnapshot,
    /// Last durable import result, until dismissed.
    pub import_outcome: Option<ImportOutcome>,
    /// Local appearance preferences.
    pub preferences: Preferences,
}

#[cfg(test)]
mod tests {
    use super::{Appearance, Preferences};

    #[test]
    fn appearance_toggle_overrides_then_returns_to_system() {
        for system_dark in [false, true] {
            let opposite = if system_dark {
                Appearance::Light
            } else {
                Appearance::Dark
            };
            assert_eq!(Appearance::System.toggled(system_dark), opposite);
            assert_eq!(opposite.toggled(system_dark), Appearance::System);
        }
        // An override that came to match the system still flips what is visible.
        assert_eq!(Appearance::Dark.toggled(true), Appearance::Light);
        assert_eq!(Appearance::Light.toggled(false), Appearance::Dark);
    }

    #[test]
    fn preferences_saved_before_sidebar_collapse_still_load() -> serde_json::Result<()> {
        let preferences: Preferences = serde_json::from_str(r#"{"sidebar_width":300.0}"#)?;
        assert_eq!(preferences.sidebar_width, 300.0);
        assert!(!preferences.sidebar_collapsed);
        assert!(preferences.folded_sections.is_empty());
        assert_eq!(preferences.library_view, super::LibraryView::List);
        assert_eq!(preferences.active_preset, None);
        Ok(())
    }

    #[test]
    fn sections_folded_as_folders_stay_folded_as_presets() -> serde_json::Result<()> {
        let preferences: Preferences =
            serde_json::from_str(r#"{"sidebar_width":300.0,"folded_sections":["folders"]}"#)?;
        assert_eq!(
            preferences.folded_sections,
            [super::SidebarSection::Presets].into()
        );
        Ok(())
    }

    #[test]
    fn the_active_preset_preference_round_trips() -> serde_json::Result<()> {
        let preferences: Preferences =
            serde_json::from_str(r#"{"sidebar_width":300.0,"active_preset":"default"}"#)?;
        assert_eq!(
            preferences.active_preset,
            Some(crate::PresetId("default".to_owned()))
        );
        Ok(())
    }
}
