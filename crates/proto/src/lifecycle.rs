use crate::{ImportOutcome, LibrarySnapshot, ProjectRoot, ScanSnapshot, SyncSnapshot};
use serde::{Deserialize, Serialize};

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
    /// Configured project boundaries.
    pub roots: Vec<ProjectRoot>,
    /// Current retained scan.
    pub scan: Option<ScanSnapshot>,
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
        Ok(())
    }
}
