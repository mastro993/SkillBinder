use crate::models::onboarding::OnboardingProgress;
use serde::{Deserialize, Serialize};

contract! {
    pub enum CheckState {
        Ready,
        NeedsAttention,
    }
}

contract! {
    pub struct PrerequisiteStatus {
        pub state: CheckState,
        pub summary: String,
        pub detail: String,
        pub repair_instruction: Option<String>,
    }
}

contract! {
    pub struct GitEnvironmentStatus {
        pub prerequisite: PrerequisiteStatus,
        pub executable: Option<String>,
        pub version: Option<String>,
    }
}

contract! {
    pub enum LibraryState {
        NotCreated,
        Ready,
        RecoveryRequired,
    }
}

contract! {
    pub struct BootstrapResponse {
        pub app_version: String,
        pub protocol_version: String,
        pub capabilities: Vec<String>,
        pub git: GitEnvironmentStatus,
        pub storage: PrerequisiteStatus,
        pub onboarding: OnboardingProgress,
        pub library_state: LibraryState,
        pub current_revision: Option<String>,
        pub recovery_summary: Option<String>,
    }
}
