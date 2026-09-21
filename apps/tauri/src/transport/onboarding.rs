use crate::transport::bootstrap::LibraryState;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

contract! {
    pub enum OnboardingStep {
        Prerequisites,
        Boundaries,
        SyncChoice,
        Ready,
    }
}

contract! {
    pub struct OnboardingProgress {
        pub step: OnboardingStep,
        pub completed: bool,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOnboardingProgressRequest {
    pub step: OnboardingStep,
}

impl<'de> Deserialize<'de> for UpdateOnboardingProgressRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct WireRequest {
            step: OnboardingStep,
        }

        let request = WireRequest::deserialize(deserializer)?;
        Ok(Self { step: request.step })
    }
}

contract! {
    pub struct CompleteOnboardingResponse {
        pub library_state: LibraryState,
        pub library_id: String,
        pub current_revision: String,
    }
}
