use crate::{FolderId, PresetId, SkillId};
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
/// A preset color: a named palette color, resolved against the active theme when rendered, or
/// any custom color, drawn the same in every theme.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PresetColor {
    /// Neutral gray.
    Gray,
    /// Red.
    Red,
    /// Orange.
    Orange,
    /// Yellow.
    Yellow,
    /// Green.
    Green,
    /// Blue.
    Blue,
    /// Purple.
    Purple,
    /// Pink.
    Pink,
    /// A custom color, stored as `"#rrggbb"`.
    #[serde(untagged)]
    Custom(Rgb),
}
impl PresetColor {
    /// Every palette color in menu order.
    pub const ALL: [Self; 8] = [
        Self::Gray,
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::Blue,
        Self::Purple,
        Self::Pink,
    ];
}
/// An opaque sRGB color as `0xRRGGBB`, stored as a `"#rrggbb"` string. A value above
/// `0xffffff` fails to serialize instead of being truncated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgb(pub u32);
impl Serialize for Rgb {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.0 > 0xff_ffff {
            return Err(serde::ser::Error::custom(format!(
                "preset color {:#x} is not 24-bit",
                self.0
            )));
        }
        serializer.serialize_str(&format!("#{:06x}", self.0))
    }
}
impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.strip_prefix('#')
            .filter(|hex| hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .and_then(|hex| u32::from_str_radix(hex, 16).ok())
            .map(Rgb)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown preset color {text:?}")))
    }
}
/// A preset glyph from a curated set, resolved to bundled artwork when rendered.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(missing_docs, reason = "Each variant names its glyph.")]
pub enum PresetIcon {
    /// The folder glyph presets show unless another is chosen.
    #[default]
    Folder,
    Star,
    Heart,
    Bookmark,
    Flag,
    Tag,
    Target,
    Fire,
    Rocket,
    Globe,
    Code,
    Terminal,
    Bug,
    Database,
    Server,
    Cloud,
    Api,
    Chip,
    Package,
    Puzzle,
    Wrench,
    Shield,
    Sparkles,
    Brain,
    Robot,
    MagicWand,
    Idea,
    Book,
    Note,
    Quill,
    Translate,
    Brush,
    Image,
    Video,
    Music,
    Chart,
    Calendar,
    Chat,
    People,
    Briefcase,
}
impl PresetIcon {
    /// Every icon in menu order.
    pub const ALL: [Self; 40] = [
        Self::Folder,
        Self::Star,
        Self::Heart,
        Self::Bookmark,
        Self::Flag,
        Self::Tag,
        Self::Target,
        Self::Fire,
        Self::Rocket,
        Self::Globe,
        Self::Code,
        Self::Terminal,
        Self::Bug,
        Self::Database,
        Self::Server,
        Self::Cloud,
        Self::Api,
        Self::Chip,
        Self::Package,
        Self::Puzzle,
        Self::Wrench,
        Self::Shield,
        Self::Sparkles,
        Self::Brain,
        Self::Robot,
        Self::MagicWand,
        Self::Idea,
        Self::Book,
        Self::Note,
        Self::Quill,
        Self::Translate,
        Self::Brush,
        Self::Image,
        Self::Video,
        Self::Music,
        Self::Chart,
        Self::Calendar,
        Self::Chat,
        Self::People,
        Self::Briefcase,
    ];
}
/// A flat portable preset: a named set of library skills.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    /// Stable preset identity.
    pub id: PresetId,
    /// Unique normalized display name.
    pub name: String,
    /// User-defined order; the engine keeps positions dense from zero.
    #[serde(default)]
    pub position: u32,
    /// Optional quick-scan color.
    #[serde(default)]
    pub color: Option<PresetColor>,
    /// Quick-scan glyph; presets stored before icons existed read as the folder.
    #[serde(default)]
    pub icon: PresetIcon,
}
/// A library folder: a named grouping of library skills for browsing only.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    /// Stable folder identity.
    pub id: FolderId,
    /// Unique normalized display name.
    pub name: String,
    /// Creation order; the engine keeps positions dense from zero.
    #[serde(default)]
    pub position: u32,
}
/// A retained portable tag; no tag editor is exposed in the application.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    /// Stable portable identifier.
    pub id: String,
    /// Unique normalized display name.
    pub name: String,
}
/// A managed skill record in portable schema 3.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    /// Portable record identity.
    pub id: SkillId,
    /// Skill directory name independent of conflict suffixes.
    pub slug: String,
    /// Optional user-facing title.
    pub display_name: Option<String>,
    /// Sorted identities of the presets that hold the skill; schema 2's single `folderId`
    /// reads as one.
    #[serde(default, alias = "folderId", deserialize_with = "schema2::preset_ids")]
    pub preset_ids: Vec<PresetId>,
    /// The library folder holding the skill, if any. Not `folderId`, which schema 2 used for
    /// what are now presets.
    #[serde(
        default,
        rename = "libraryFolderId",
        skip_serializing_if = "Option::is_none"
    )]
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
    /// RFC 3339 time the skill was imported; absent for skills imported before it was recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported_at: Option<String>,
}
/// Portable schema 3, committed only by explicit Git operations.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Library {
    /// Currently 3. Reads upgrade schema 2, which called presets folders and allowed one per
    /// skill, so the next write saves schema 3.
    #[serde(deserialize_with = "schema2::version")]
    pub schema_version: u32,
    /// Stable library identity.
    pub library_id: String,
    /// Creation timestamp in RFC 3339 format.
    pub created_at: String,
    /// Payload policy version, currently 1.
    #[serde(default = "policy_version")]
    pub content_policy_version: u32,
    /// Flat presets, ordered by position after reads.
    #[serde(default, alias = "folders", with = "preset_map")]
    pub presets: Vec<Preset>,
    /// Library folders, ordered by position after reads. Not `folders`, which schema 2 used for
    /// what are now presets.
    #[serde(default, rename = "libraryFolders", with = "folder_map")]
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
    /// When each skill's newest managed file last changed on this device, in Unix seconds;
    /// absent when its payload could not be read.
    pub modified: BTreeMap<SkillId, u64>,
}
/// A portable organization mutation.
#[derive(Clone, Debug)]
pub enum OrganizationChange {
    /// Create a flat preset.
    CreatePreset {
        /// New unique preset name.
        name: String,
    },
    /// Rename an existing preset.
    RenamePreset {
        /// Preset being renamed.
        id: PresetId,
        /// Replacement unique name.
        name: String,
    },
    /// Add selected skills to a preset; skills already in it stay.
    AddToPreset {
        /// Preset receiving the skills.
        id: PresetId,
        /// Selected existing skill identities.
        skills: Vec<SkillId>,
    },
    /// Remove selected skills from a preset; they stay in the library and in other presets.
    RemoveFromPreset {
        /// Preset losing the skills.
        id: PresetId,
        /// Selected existing skill identities.
        skills: Vec<SkillId>,
    },
    /// Replace the preset order with a permutation of every existing preset.
    ReorderPresets {
        /// Every preset identity in its new order.
        order: Vec<PresetId>,
    },
    /// Set or clear a preset color.
    SetPresetColor {
        /// Preset being colored.
        id: PresetId,
        /// Replacement color, or none.
        color: Option<PresetColor>,
    },
    /// Set a preset icon.
    SetPresetIcon {
        /// Preset being changed.
        id: PresetId,
        /// Replacement icon.
        icon: PresetIcon,
    },
    /// Create a library folder after the existing ones.
    CreateFolder {
        /// New unique folder name.
        name: String,
    },
    /// Rename an existing library folder.
    RenameFolder {
        /// Folder being renamed.
        id: FolderId,
        /// Replacement unique name.
        name: String,
    },
    /// Delete a library folder; its skills stay in the library, unfiled.
    DeleteFolder {
        /// Folder being removed.
        id: FolderId,
        /// Revision the confirmation was shown for.
        revision: String,
    },
    /// Move a skill into a library folder, out of any other, or into none.
    MoveToFolder {
        /// Skill being moved.
        skill: SkillId,
        /// Destination folder, or none to leave it unfiled.
        folder: Option<FolderId>,
    },
    /// Delete a previewed preset; its skills stay in the library.
    DeletePreset {
        /// Previewed preset identity.
        id: PresetId,
        /// Revision captured by the deletion preview.
        revision: String,
    },
}
/// Consequences of deleting a preset.
#[derive(Clone, Debug)]
pub struct DeletePreview {
    /// Preset being removed.
    pub preset: Preset,
    /// Skills the preset holds; they stay in the library.
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
record_map!(preset_map, Preset);
record_map!(folder_map, Folder);
record_map!(tag_map, Tag);
record_map!(skill_map, Skill);

/// Reads schema 2 metadata as schema 3.
mod schema2 {
    use super::*;
    use serde::Deserializer;

    pub fn version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
        let version = u32::deserialize(deserializer)?;
        Ok(if version == 2 { 3 } else { version })
    }

    /// Schema 3's `presetIds` list, or schema 2's nullable single `folderId`.
    pub fn preset_ids<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<PresetId>, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Ids {
            Folder(PresetId),
            Presets(Vec<PresetId>),
        }
        Ok(match Option::<Ids>::deserialize(deserializer)? {
            None => Vec::new(),
            Some(Ids::Folder(id)) => vec![id],
            Some(Ids::Presets(ids)) => ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Library, PresetId};

    fn skill(id: &str, assignment: &str) -> String {
        format!(
            r#""{id}":{{"id":"{id}","slug":"{id}","displayName":null,{assignment}"tagIds":[],"upstreamBindings":[],"digest":"sha256:0","fileCount":1,"totalBytes":1}}"#
        )
    }

    #[test]
    fn schema_2_folders_read_as_presets() -> serde_json::Result<()> {
        let skills = [
            skill("filed", r#""folderId":"f","#),
            skill("unfiled", r#""folderId":null,"#),
        ];
        let library: Library = serde_json::from_str(&format!(
            r#"{{"schemaVersion":2,"libraryId":"l","createdAt":"now","folders":{{"f":{{"id":"f","name":"Research","position":0,"color":"green"}}}},"tags":{{}},"skills":{{{}}}}}"#,
            skills.join(",")
        ))?;
        assert_eq!(library.schema_version, 3);
        assert_eq!(library.presets[0].name, "Research");
        assert_eq!(library.skills[0].preset_ids, [PresetId("f".into())]);
        assert!(library.skills[1].preset_ids.is_empty());
        let written = serde_json::to_value(&library)?;
        assert_eq!(written["schemaVersion"], 3);
        assert!(written.get("folders").is_none());
        assert_eq!(written["presets"]["f"]["name"], "Research");
        assert!(written["skills"]["filed"].get("folderId").is_none());
        assert_eq!(written["skills"]["filed"]["presetIds"][0], "f");
        Ok(())
    }

    #[test]
    fn preset_colors_store_palette_names_and_custom_hex() -> serde_json::Result<()> {
        use super::{PresetColor, Rgb};
        let colors = [PresetColor::Green, PresetColor::Custom(Rgb(0x3366ff))];
        assert_eq!(serde_json::to_string(&colors)?, r##"["green","#3366ff"]"##);
        let read: Vec<PresetColor> = serde_json::from_str(r##"["green","#3366FF"]"##)?;
        assert_eq!(read, colors);
        assert!(serde_json::to_string(&PresetColor::Custom(Rgb(0x100_0000))).is_err());
        for invalid in [
            r#""teal""#,
            r##""#3366f""##,
            r##""#33g6ff""##,
            r#""3366ff""#,
        ] {
            assert!(
                serde_json::from_str::<PresetColor>(invalid).is_err(),
                "{invalid}"
            );
        }
        Ok(())
    }

    #[test]
    fn folders_use_keys_apart_from_schema_2_folders() -> serde_json::Result<()> {
        let library: Library = serde_json::from_str(&format!(
            r#"{{"schemaVersion":3,"libraryId":"l","createdAt":"now","presets":{{}},"libraryFolders":{{"f":{{"id":"f","name":"Writing","position":0}}}},"tags":{{}},"skills":{{{},{}}}}}"#,
            skill("filed", r#""libraryFolderId":"f","#),
            skill("unfiled", ""),
        ))?;
        assert_eq!(library.folders[0].name, "Writing");
        assert!(library.presets.is_empty());
        assert_eq!(
            library.skills[0].folder_id,
            Some(super::FolderId("f".into()))
        );
        assert!(library.skills[0].preset_ids.is_empty());
        let written = serde_json::to_value(&library)?;
        assert_eq!(written["libraryFolders"]["f"]["name"], "Writing");
        assert_eq!(written["skills"]["filed"]["libraryFolderId"], "f");
        assert!(
            written["skills"]["unfiled"]
                .get("libraryFolderId")
                .is_none()
        );
        Ok(())
    }

    #[test]
    fn schema_3_keeps_every_preset_of_a_skill() -> serde_json::Result<()> {
        let library: Library = serde_json::from_str(&format!(
            r#"{{"schemaVersion":3,"libraryId":"l","createdAt":"now","presets":{{}},"tags":{{}},"skills":{{{},{}}}}}"#,
            skill("both", r#""presetIds":["a","b"],"#),
            skill("none", ""),
        ))?;
        assert_eq!(library.schema_version, 3);
        assert_eq!(library.skills[0].preset_ids.len(), 2);
        assert!(library.skills[1].preset_ids.is_empty());
        Ok(())
    }
}
