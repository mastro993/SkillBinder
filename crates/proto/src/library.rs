use crate::{FolderId, SkillId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Severity in ascending order; blocked payloads can never be imported.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidationStatus {
    /// All checks passed.
    #[default]
    Valid,
    /// Import is allowed with advisory messages.
    Warning,
    /// Import requires explicit acknowledgement.
    Invalid,
    /// The payload cannot safely be copied.
    Blocked,
}
/// One payload validation finding.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValidationMessage {
    /// Severity of this finding.
    pub status: ValidationStatus,
    /// Safe explanation.
    pub message: String,
}
/// A flat portable folder.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    /// Stable folder identity.
    pub id: FolderId,
    /// Unique normalized display name.
    pub name: String,
}
/// A retained portable tag; no tag editor is exposed in the application.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    /// Stable portable identifier.
    pub id: String,
    /// Unique normalized display name.
    pub name: String,
}
/// A managed skill record in portable schema 2.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    /// Portable record identity.
    pub id: SkillId,
    /// Skill directory name independent of conflict suffixes.
    pub slug: String,
    /// Optional user-facing title.
    pub display_name: Option<String>,
    /// The single assigned folder, if any.
    pub folder_id: Option<FolderId>,
    /// Sorted portable tag identities.
    #[serde(default)]
    pub tag_ids: Vec<String>,
    /// Portable upstream annotations retained without a binding UI.
    #[serde(default)]
    pub upstream_bindings: Vec<serde_json::Value>,
    /// Manifest SHA-256.
    pub digest: String,
    /// Number of regular files after link materialization.
    pub file_count: u64,
    /// Total payload bytes.
    pub total_bytes: u64,
}
/// Portable schema 2, committed only by explicit Git operations.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    /// Currently 2.
    pub schema_version: u32,
    /// Stable library identity.
    pub library_id: String,
    /// Creation timestamp in RFC 3339 format.
    pub created_at: String,
    /// Payload policy version, currently 1.
    #[serde(default = "policy_version")]
    pub content_policy_version: u32,
    /// Flat folders.
    #[serde(default, with = "folder_map")]
    pub folders: Vec<Folder>,
    /// Tags preserved in portable state.
    #[serde(default, with = "tag_map")]
    pub tags: Vec<Tag>,
    /// Managed skill records.
    #[serde(default, with = "skill_map")]
    pub skills: Vec<Skill>,
}
fn policy_version() -> u32 {
    1
}
/// Local skill details derived during inspection.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillDetails {
    /// Frontmatter description, used by search.
    pub description: String,
    /// Combined validation severity.
    pub validation: ValidationStatus,
    /// Individual validation findings.
    pub messages: Vec<ValidationMessage>,
    /// Reader labels observed at source locations.
    pub readers: Vec<String>,
    /// Display paths of source observations.
    pub sources: Vec<String>,
}
/// A revision-checked immutable library projection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LibrarySnapshot {
    /// Portable content.
    pub library: Library,
    /// Hash of portable metadata and current payload catalog.
    pub revision: String,
    /// Machine-local details by skill identity.
    pub details: BTreeMap<SkillId, SkillDetails>,
}
/// A portable organization mutation.
#[derive(Clone, Debug)]
pub enum OrganizationChange {
    /// Create a flat folder.
    CreateFolder {
        /// New unique folder name.
        name: String,
    },
    /// Rename an existing folder.
    RenameFolder {
        /// Folder being renamed.
        id: FolderId,
        /// Replacement unique name.
        name: String,
    },
    /// Assign selected skills, or move them to Unfiled.
    Assign {
        /// Selected existing skill identities.
        skills: Vec<SkillId>,
        /// Destination, or Unfiled when absent.
        folder: Option<FolderId>,
    },
    /// Delete a previewed folder and unfile its skills.
    DeleteFolder {
        /// Previewed folder identity.
        id: FolderId,
        /// Revision captured by the deletion preview.
        revision: String,
    },
}
/// Consequences of deleting a flat folder.
#[derive(Clone, Debug)]
pub struct DeletePreview {
    /// Folder being removed.
    pub folder: Folder,
    /// Skills that will become unfiled.
    pub affected_skills: usize,
    /// Required revision for confirmation.
    pub revision: String,
}
/// A bounded payload preview listing.
#[derive(Clone, Debug)]
pub struct SkillPreview {
    /// Skill being inspected.
    pub skill_id: SkillId,
    /// Listed relative paths, including directories.
    pub entries: Vec<PreviewEntry>,
    /// Selected relative text file, if available.
    pub text: Option<String>,
    /// Why selected content cannot be previewed.
    pub unavailable: Option<String>,
}
/// One entry in a skill preview.
#[derive(Clone, Debug)]
pub struct PreviewEntry {
    /// Portable relative path.
    pub path: String,
    /// Whether the entry is a directory.
    pub directory: bool,
    /// File size in bytes.
    pub bytes: u64,
}

macro_rules! record_map {
    ($module:ident, $record:ty) => {
        mod $module {
            use super::*;
            pub fn serialize<S: serde::Serializer>(
                records: &[$record],
                serializer: S,
            ) -> Result<S::Ok, S::Error> {
                records
                    .iter()
                    .map(|record| (record.id.to_string(), record))
                    .collect::<BTreeMap<_, _>>()
                    .serialize(serializer)
            }
            pub fn deserialize<'de, D: serde::Deserializer<'de>>(
                deserializer: D,
            ) -> Result<Vec<$record>, D::Error> {
                let records = BTreeMap::<String, $record>::deserialize(deserializer)?;
                if records
                    .iter()
                    .any(|(key, record)| key != &record.id.to_string())
                {
                    return Err(serde::de::Error::custom(
                        "record identity does not match its map key",
                    ));
                }
                Ok(records.into_values().collect())
            }
        }
    };
}
record_map!(folder_map, Folder);
record_map!(tag_map, Tag);
record_map!(skill_map, Skill);
