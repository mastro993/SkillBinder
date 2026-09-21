use crate::transport::validation::ValidationSummary;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
