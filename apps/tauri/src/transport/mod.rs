use serde::{Deserialize, Serialize};
use ts_rs::TS;

macro_rules! contract {
    ($item:item) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
        #[serde(rename_all = "camelCase")]
        $item
    };
}

pub mod bootstrap;
pub mod diagnostics;
pub mod discovery;
pub mod error;
pub mod git_sync;
pub mod imports;
pub mod library;
pub mod onboarding;
pub mod roots;
pub mod validation;

pub use bootstrap::*;
pub use diagnostics::*;
pub use discovery::*;
pub use error::*;
pub use git_sync::*;
pub use imports::*;
pub use library::*;
pub use onboarding::*;
pub use roots::*;
pub use validation::*;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use ts_rs::Config;

    #[test]
    #[ignore = "run through pnpm contracts:generate"]
    fn export_bindings() {
        let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend/src/types");
        let config = Config::new().with_out_dir(output);
        BootstrapResponse::export_all(&config).unwrap();
        CompleteOnboardingResponse::export_all(&config).unwrap();
        UpdateOnboardingProgressRequest::export_all(&config).unwrap();
        CommandResult::<BootstrapResponse>::export_all(&config).unwrap();
        RootsPickResponse::export_all(&config).unwrap();
        RootsRegisterRequest::export_all(&config).unwrap();
        RootsRegisterResponse::export_all(&config).unwrap();
        RootsListResponse::export_all(&config).unwrap();
        RootsUpdateRequest::export_all(&config).unwrap();
        RootsUpdateResponse::export_all(&config).unwrap();
        RootsRemoveRequest::export_all(&config).unwrap();
        RootsRemoveResponse::export_all(&config).unwrap();
        DiscoveryStartResponse::export_all(&config).unwrap();
        DiscoveryResultsRequest::export_all(&config).unwrap();
        DiscoveryResultsResponse::export_all(&config).unwrap();
        DiscoveryCancelRequest::export_all(&config).unwrap();
        DiscoveryCancelResponse::export_all(&config).unwrap();
        DiscoveryCurrentResponse::export_all(&config).unwrap();
        ImportPrepareRequest::export_all(&config).unwrap();
        ImportApplyRequest::export_all(&config).unwrap();
        ImportPlanResponse::export_all(&config).unwrap();
        ImportApplyResponse::export_all(&config).unwrap();
        LibraryListResponse::export_all(&config).unwrap();
        DiagnosticsRevealLogsResponse::export_all(&config).unwrap();
        GitSyncState::export_all(&config).unwrap();
        GitSyncStatus::export_all(&config).unwrap();
        GitSyncConnectRequest::export_all(&config).unwrap();
    }

    #[test]
    fn discovery_candidate_serializes_tagged_variants() {
        let candidate = DiscoveryCandidate {
            candidate_id: "scan-1:0".into(),
            location_id: "scan-1:loc:0".into(),
            display_path: "/home/dev/.claude/skills/code-review".into(),
            slug: "code-review".into(),
            name: Some("code-review".into()),
            description: Some("Reviews code.".into()),
            reader_agent_ids: vec!["claude-code".into()],
            validation: ValidationSummary {
                status: ValidationStatus::Valid,
                messages: vec![ValidationMessage {
                    code: ValidationCode::VcsMetadataExcluded,
                    message: "Excluded .git directory".into(),
                }],
            },
            duplicate: CandidateDuplicate::Identical {
                skill_id: "skill-1".into(),
                slug: "code-review".into(),
            },
            file_count: 3,
            total_bytes: "2048".into(),
            linked: false,
            warnings: Vec::new(),
        };

        let value = serde_json::to_value(&candidate).unwrap();
        assert_eq!(value["duplicate"]["kind"], "identical");
        assert_eq!(value["fileCount"], 3);
        assert_eq!(value["totalBytes"], "2048");
        assert_eq!(value["locationId"], "scan-1:loc:0");
        assert_eq!(value["linked"], false);
        assert_eq!(
            value["validation"]["messages"][0]["code"],
            "vcsMetadataExcluded"
        );

        assert_eq!(
            serde_json::to_value(ImportOutcome::NewSkill).unwrap(),
            serde_json::json!({ "kind": "newSkill" })
        );
    }

    #[test]
    fn import_requests_reject_unknown_input() {
        assert!(
            serde_json::from_value::<ImportPrepareRequest>(serde_json::json!({
                "candidateIds": ["scan-1:0"],
                "path": "/etc"
            }))
            .is_err()
        );
        assert!(
            !serde_json::from_value::<ImportPrepareRequest>(serde_json::json!({
                "candidateIds": ["scan-1:0"]
            }))
            .unwrap()
            .allow_invalid_skills
        );
        assert_eq!(
            serde_json::from_value::<ImportApplyRequest>(serde_json::json!({ "planId": "plan-1" }))
                .unwrap()
                .plan_id,
            "plan-1"
        );
    }

    #[test]
    fn scan_requests_reject_unknown_input() {
        assert!(
            serde_json::from_value::<RootsRegisterRequest>(serde_json::json!({
                "grantId": "grant-1",
                "path": "/etc"
            }))
            .is_err()
        );
        assert_eq!(
            serde_json::from_value::<RootsRegisterRequest>(serde_json::json!({
                "grantId": "grant-1"
            }))
            .unwrap()
            .label,
            None
        );
        assert!(
            serde_json::from_value::<DiscoveryResultsRequest>(serde_json::json!({
                "scanId": "scan-1",
                "offset": 0,
                "limit": 100,
                "unexpected": true
            }))
            .is_err()
        );
        assert_eq!(
            serde_json::from_value::<RootsUpdateRequest>(serde_json::json!({
                "rootId": "root-1",
                "label": "Work",
                "enabled": false
            }))
            .unwrap(),
            RootsUpdateRequest {
                root_id: "root-1".into(),
                label: "Work".into(),
                enabled: false,
            }
        );
        assert_eq!(
            serde_json::from_value::<DiscoveryCancelRequest>(serde_json::json!({
                "scanId": "scan-1"
            }))
            .unwrap()
            .scan_id,
            "scan-1"
        );
    }

    #[test]
    fn scan_phase_and_exclusion_reason_use_camel_case_names() {
        assert_eq!(
            serde_json::to_value(ScanPhase::Cancelled).unwrap(),
            serde_json::json!("cancelled")
        );
        assert_eq!(
            serde_json::to_value(ExclusionReason::MountBoundary).unwrap(),
            serde_json::json!("mountBoundary")
        );
        assert_eq!(
            serde_json::to_value(ExclusionReason::DependencyVendor).unwrap(),
            serde_json::json!("dependencyVendor")
        );
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
