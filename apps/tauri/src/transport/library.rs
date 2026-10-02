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
        pub folder_id: Option<String>,
        pub tag_ids: Vec<String>,
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
        pub folders: Vec<FolderView>,
        pub tags: Vec<TagView>,
        pub organization_revision: String,
    }
}

contract! {
    pub struct FolderView {
        pub id: String,
        pub name: String,
    }
}
contract! {
    pub struct TagView {
        pub id: String,
        pub name: String,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum OrganizationChange {
    CreateFolder {
        name: String,
    },
    UpdateFolder {
        id: String,
        name: String,
    },
    DeleteFolder {
        id: String,
    },
    CreateTag {
        name: String,
    },
    RenameTag {
        id: String,
        name: String,
    },
    DeleteTag {
        id: String,
    },
    Assign {
        skill_ids: Vec<String>,
        folder_id: Option<String>,
        set_folder: bool,
        add_tag_ids: Vec<String>,
        remove_tag_ids: Vec<String>,
    },
}
contract! {
    pub struct OrganizationChangeRequest {
        pub change: OrganizationChange,
        pub expected_revision: Option<String>,
    }
}
contract! {
    pub struct OrganizationChangeResponse {
        pub organization_revision: String,
    }
}
contract! {
    pub struct OrganizationDeletePreviewRequest {
        pub entity: String,
        pub id: String,
    }
}
contract! {
    pub struct OrganizationDeletePreviewResponse {
        pub affected_skills: u32,
        pub organization_revision: String,
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
