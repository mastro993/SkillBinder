use crate::models::validation::ValidationSummary;
use serde::{Deserialize, Serialize};

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
        pub digest: String,
        pub payload_directory: String,
    }
}

contract! {
    pub struct LibraryListResponse {
        pub library_revision: Option<String>,
        pub has_uncommitted_changes: bool,
        pub pending_resolution: bool,
        pub skills: Vec<LibrarySkill>,
    }
}

contract! {
    pub struct LibraryResolveConflictRequest {
        pub slug: String,
        pub keep_skill_id: String,
        pub expected_skill_ids: Vec<String>,
    }
}

contract! {
    pub struct LibraryResolveConflictResponse {
        pub slug: String,
        pub kept_skill_id: String,
        pub removed_skill_ids: Vec<String>,
        pub payload_directory: String,
        pub library_revision: Option<String>,
        pub has_uncommitted_changes: bool,
    }
}

contract! {
    pub struct LibrarySkillPreviewRequest {
        pub skill_id: String,
        pub path: Option<String>,
    }
}

contract! {
    pub struct LibrarySkillPreviewResponse {
        pub skill_id: String,
        pub last_edited_at: Option<u32>,
        pub files: Vec<String>,
        pub path: String,
        pub content: Option<String>,
        pub unavailable_reason: Option<String>,
    }
}
