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
        UnsupportedSkill,
        InvalidPath,
        SourceChanged,
        StalePlan,
        LimitExceeded,
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
        RescanDiscovery,
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

contract! {
    pub enum LocationState {
        Scanned,
        Missing,
        Unreadable,
    }
}

contract! {
    pub struct GlobalLocation {
        pub display_path: String,
        pub agent_ids: Vec<String>,
        pub agent_labels: Vec<String>,
        pub state: LocationState,
        pub detail: Option<String>,
    }
}

contract! {
    pub enum ValidationStatus {
        Valid,
        Warning,
        Invalid,
        Blocked,
    }
}

contract! {
    pub enum ValidationCode {
        MissingSkillFile,
        InvalidFrontmatter,
        UnsupportedYaml,
        InvalidUtf8,
        NameMissing,
        NameMismatch,
        NameTooLong,
        NameHyphenRule,
        DescriptionMissing,
        DescriptionTooLong,
        UnsafeEntryPath,
        ReservedEntryName,
        CaseCollision,
        UnsupportedEntryType,
        ExternalSymlink,
        LinkCycle,
        VcsMetadataExcluded,
        PluginManifest,
        PayloadLimitExceeded,
        FileLimitExceeded,
        IndexNotBuilt,
    }
}

contract! {
    pub struct ValidationMessage {
        pub code: ValidationCode,
        pub message: String,
    }
}

contract! {
    pub struct ValidationSummary {
        pub status: ValidationStatus,
        pub messages: Vec<ValidationMessage>,
    }
}

contract! {
    #[serde(tag = "kind", rename_all_fields = "camelCase")]
    pub enum CandidateLink {
        Direct,
        RootLink { resolved_path: String },
        Unresolved { detail: String },
    }
}

contract! {
    #[serde(tag = "kind", rename_all_fields = "camelCase")]
    pub enum CandidateDuplicate {
        Unique,
        Identical { skill_id: String, slug: String },
        SlugInUse { skill_id: String, slug: String },
    }
}

contract! {
    pub struct DiscoveryCandidate {
        pub candidate_id: String,
        pub display_path: String,
        pub slug: String,
        pub name: Option<String>,
        pub description: Option<String>,
        pub reader_agent_ids: Vec<String>,
        pub link: CandidateLink,
        pub validation: ValidationSummary,
        pub duplicate: CandidateDuplicate,
        pub file_count: u32,
        pub total_bytes: String,
        pub warnings: Vec<String>,
    }
}

contract! {
    pub struct DiscoveryScanResponse {
        pub registry_version: u32,
        pub locations: Vec<GlobalLocation>,
        pub candidates: Vec<DiscoveryCandidate>,
    }
}

contract! {
    #[serde(tag = "kind", rename_all_fields = "camelCase")]
    pub enum ImportOutcome {
        NewSkill,
        AttachObservation { skill_id: String },
    }
}

contract! {
    pub struct ImportPlanItem {
        pub candidate_id: String,
        pub display_path: String,
        pub slug: String,
        pub skill_id: String,
        pub outcome: ImportOutcome,
        pub validation: ValidationSummary,
        pub duplicate: CandidateDuplicate,
        pub file_count: u32,
        pub total_bytes: String,
        pub exclusions: Vec<String>,
    }
}

contract! {
    pub struct ImportPlanResponse {
        pub plan_id: String,
        pub expires_at: String,
        pub library_revision: Option<String>,
        pub items: Vec<ImportPlanItem>,
    }
}

contract! {
    pub struct ImportedSkill {
        pub skill_id: String,
        pub slug: String,
        pub display_path: String,
        pub outcome: ImportOutcome,
        pub file_count: u32,
        pub total_bytes: String,
    }
}

contract! {
    pub struct ImportApplyResponse {
        pub plan_id: String,
        pub imported: Vec<ImportedSkill>,
        pub library_revision: Option<String>,
    }
}

contract! {
    pub struct SkillSource {
        pub display_path: String,
        pub reader_agent_ids: Vec<String>,
    }
}

contract! {
    pub struct LibrarySkill {
        pub skill_id: String,
        pub slug: String,
        pub display_name: Option<String>,
        pub description: Option<String>,
        pub validation: ValidationSummary,
        pub file_count: u32,
        pub total_bytes: String,
        pub sources: Vec<SkillSource>,
    }
}

contract! {
    pub struct LibraryListResponse {
        pub library_revision: Option<String>,
        pub has_uncommitted_changes: bool,
        pub skills: Vec<LibrarySkill>,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportPrepareRequest {
    pub candidate_ids: Vec<String>,
    pub allow_invalid_skills: bool,
}

impl<'de> Deserialize<'de> for ImportPrepareRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct WireRequest {
            candidate_ids: Vec<String>,
            #[serde(default)]
            allow_invalid_skills: bool,
        }

        let request = WireRequest::deserialize(deserializer)?;
        Ok(Self {
            candidate_ids: request.candidate_ids,
            allow_invalid_skills: request.allow_invalid_skills,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportApplyRequest {
    pub plan_id: String,
}

impl<'de> Deserialize<'de> for ImportApplyRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct WireRequest {
            plan_id: String,
        }

        let request = WireRequest::deserialize(deserializer)?;
        Ok(Self {
            plan_id: request.plan_id,
        })
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
        DiscoveryScanResponse::export_all(&config).unwrap();
        ImportPrepareRequest::export_all(&config).unwrap();
        ImportApplyRequest::export_all(&config).unwrap();
        ImportPlanResponse::export_all(&config).unwrap();
        ImportApplyResponse::export_all(&config).unwrap();
        LibraryListResponse::export_all(&config).unwrap();
    }

    #[test]
    fn discovery_candidate_serializes_tagged_variants() {
        let candidate = DiscoveryCandidate {
            candidate_id: "scan-1:0".into(),
            display_path: "/home/dev/.claude/skills/code-review".into(),
            slug: "code-review".into(),
            name: Some("code-review".into()),
            description: Some("Reviews code.".into()),
            reader_agent_ids: vec!["claude-code".into()],
            link: CandidateLink::RootLink {
                resolved_path: "/data/skills/code-review".into(),
            },
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
            warnings: Vec::new(),
        };

        let value = serde_json::to_value(&candidate).unwrap();
        assert_eq!(value["link"]["kind"], "rootLink");
        assert_eq!(value["link"]["resolvedPath"], "/data/skills/code-review");
        assert_eq!(value["duplicate"]["kind"], "identical");
        assert_eq!(value["fileCount"], 3);
        assert_eq!(value["totalBytes"], "2048");
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
