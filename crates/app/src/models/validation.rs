use serde::{Deserialize, Serialize};

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
    pub enum CandidateDuplicate {
        Unique,
        Identical { skill_id: String, slug: String },
        SlugInUse { skill_id: String, slug: String },
    }
}
