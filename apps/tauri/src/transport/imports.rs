use crate::transport::validation::{CandidateDuplicate, ValidationSummary};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
