use serde::{Deserialize, Serialize};
use ts_rs::TS;

macro_rules! contract {
    ($item:item) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
        #[serde(rename_all = "camelCase")]
        $item
    };
}

contract! {
    pub enum ErrorCode {
        ValidationFailed,
        PermissionDenied,
        DatabaseUnavailable,
        GitNotFound,
        GitUnsupported,
        GitProcessFailed,
        GitProcessTimeout,
        LibraryBusy,
        RecoveryRequired,
        InternalError,
    }
}

contract! {
    pub enum RecoveryAction {
        Retry,
        ConfigureGit,
        CheckStoragePermissions,
        OpenRecovery,
    }
}

contract! {
    pub struct AppError {
        pub code: ErrorCode,
        pub message: String,
        pub retryable: bool,
        pub recovery_action: Option<RecoveryAction>,
        pub diagnostic_id: String,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(untagged)]
pub enum CommandResult<T> {
    Success { ok: bool, value: T },
    Failure { ok: bool, error: AppError },
}

impl<T> CommandResult<T> {
    pub fn success(value: T) -> Self {
        Self::Success { ok: true, value }
    }

    pub fn failure(error: AppError) -> Self {
        Self::Failure { ok: false, error }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use ts_rs::Config;

    #[test]
    #[ignore = "run through pnpm contracts:generate"]
    fn export_bindings() {
        let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend/src/generated");
        let config = Config::new().with_out_dir(output);
        BootstrapResponse::export_all(&config).unwrap();
        CompleteOnboardingResponse::export_all(&config).unwrap();
        UpdateOnboardingProgressRequest::export_all(&config).unwrap();
        CommandResult::<BootstrapResponse>::export_all(&config).unwrap();
    }

    #[test]
    fn result_envelope_serializes_success_and_failure() {
        let success = CommandResult::success("ready");
        assert_eq!(
            serde_json::to_value(success).unwrap(),
            serde_json::json!({ "ok": true, "value": "ready" })
        );

        let failure: CommandResult<()> = CommandResult::failure(AppError {
            code: ErrorCode::GitNotFound,
            message: "Git was not found".into(),
            retryable: true,
            recovery_action: Some(RecoveryAction::ConfigureGit),
            diagnostic_id: "bootstrap.git.not-found".into(),
        });
        assert_eq!(serde_json::to_value(failure).unwrap()["ok"], false);
    }

    #[test]
    fn onboarding_request_rejects_unknown_input() {
        assert!(
            serde_json::from_value::<UpdateOnboardingProgressRequest>(serde_json::json!({
                "step": "boundaries",
                "command": "arbitrary"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<UpdateOnboardingProgressRequest>(serde_json::json!({
                "step": "arbitrary"
            }))
            .is_err()
        );
    }
}
