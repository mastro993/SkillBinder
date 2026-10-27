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
/// An opaque sRGB color as `0xRRGGBB`, stored as a `"#rrggbb"` string.
/// Values wider than 24 bits cannot be constructed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgb(u32);
impl Rgb {
    /// 24-bit sRGB, or `None` when `value` is wider than `0xffffff`.
    pub const fn from_u24(value: u32) -> Option<Self> {
        if value > 0xff_ffff {
            None
        } else {
            Some(Self(value))
        }
    }

    /// An opaque color from 8-bit channels.
    pub const fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self((red as u32) << 16 | (green as u32) << 8 | blue as u32)
    }

    /// The 24-bit sRGB value.
    pub const fn get(self) -> u32 {
        self.0
    }
}
impl Serialize for Rgb {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("#{:06x}", self.0))
    }
}
impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.strip_prefix('#')
            .filter(|hex| hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .and_then(|hex| u32::from_str_radix(hex, 16).ok())
            .and_then(Rgb::from_u24)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown preset color {text:?}")))
    }
}
/// A preset glyph from a curated set, resolved to bundled artwork when rendered.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(missing_docs, reason = "Each variant names its glyph.")]
pub enum PresetIcon {
    /// The layers glyph presets show unless another is chosen.
    #[default]
    Layers,
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
        Self::Layers,
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
/// A flat machine-local preset: a named configuration of library skills.
///
/// Every library has the default preset, whose [`Preset::DEFAULT_ID`] the app seeds once. The app
/// never deletes it, renames it, changes its icon or color, or moves it; it always comes first.
/// A skill is active in a preset when the preset holds it, and one preset is active at a time.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    /// Stable preset identity.
    pub id: PresetId,
    /// Unique normalized display name.
    pub name: String,
    /// User-defined order; the engine keeps positions dense from zero.
    pub position: u32,
    /// Optional quick-scan color.
    pub color: Option<PresetColor>,
    /// Quick-scan glyph.
    pub icon: PresetIcon,
}
impl Preset {
    /// The stable identity of the preset every library starts with.
    pub const DEFAULT_ID: &'static str = "default";
    /// Whether this is the preset every library starts with.
    pub fn is_default(&self) -> bool {
        self.id.as_str() == Self::DEFAULT_ID
    }
}
/// A machine-local library folder: a named grouping of library skills for browsing only.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    /// Stable folder identity.
    pub id: FolderId,
    /// Unique normalized display name.
    pub name: String,
    /// Creation order; the engine keeps positions dense from zero.
    pub position: u32,
}
/// A library skill: a directory under `skills/` and its machine-local organization.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    /// The skill's slug, its directory name, which identifies it on every device.
    pub id: SkillId,
    /// Sorted identities of the presets that hold the skill.
    pub preset_ids: Vec<PresetId>,
    /// The library folder holding the skill, if any.
    pub folder_id: Option<FolderId>,
    /// Manifest SHA-256 cached when the payload was last inspected.
    pub digest: String,
    /// Number of regular files after link materialization.
    pub file_count: u64,
    /// Total payload bytes.
    pub total_bytes: u64,
    /// RFC 3339 time the skill was imported on this device; absent for a skill that arrived
    /// through Pull.
    pub imported_at: Option<String>,
    /// Where the skill was published, if it was imported from a known Git source.
    pub source: Option<SkillSource>,
    /// The Git tree id of the content the skill's record says it should have.
    pub recorded: String,
    /// Whether the skill's content differs from its record, and whether it can be reinstalled.
    pub drift: Option<Drift>,
}
/// A library skill whose content differs from its record, such as after an edit made outside
/// SkillBinder or a Pull of different content.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Drift {
    /// Reinstall can restore the recorded content, from the library's Git history or its source.
    Restorable,
    /// Neither the library's Git history nor a source holds the recorded content.
    Lost,
}
/// A library skill's source: the Git repository it was published from, an optional pinned ref,
/// and the skill's folder in that repository. It syncs with the library.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SkillSource {
    /// HTTPS, SSH, SCP-style SSH, or file repository URL without credentials.
    pub url: String,
    /// The branch, tag, or commit the source follows; the repository's default branch when
    /// absent.
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// The skill's folder, relative to the repository root with slash separators; empty for the
    /// root itself.
    #[serde(default)]
    pub path: String,
}
impl SkillSource {
    /// Whether the source is pinned to a commit, so it never has an update.
    pub fn pinned(&self) -> bool {
        self.reference.as_deref().is_some_and(|reference| {
            matches!(reference.len(), 40 | 64) && reference.bytes().all(|b| b.is_ascii_hexdigit())
        })
    }
}
/// What the last check of a library skill's source found.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum UpdateCheck {
    /// The source holds the skill's recorded content.
    UpToDate,
    /// The source's content differs from the skill's recorded content.
    Available,
    /// The check failed; a safe sentence explaining why.
    Failed(String),
}
/// A library skill's update state on this device, kept in memory until the next check.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillUpdate {
    /// The last check's result.
    pub check: UpdateCheck,
    /// Why the last Update of the skill failed, until it succeeds or the skill is checked again.
    pub failure: Option<String>,
}
impl SkillUpdate {
    /// Whether the skill has an update.
    pub fn available(&self) -> bool {
        self.check == UpdateCheck::Available
    }
}
/// The result of updating library skills; each skill updates on its own.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdateOutcome {
    /// Skills whose library content now matches their source.
    pub updated: Vec<SkillId>,
    /// Skills that could not be updated; their update state records why.
    pub failed: Vec<SkillId>,
}
/// The library: its skill directories and this device's organization of them.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Library {
    /// Flat presets in user order.
    pub presets: Vec<Preset>,
    /// Library folders in creation order.
    pub folders: Vec<Folder>,
    /// Library skills in slug order.
    pub skills: Vec<Skill>,
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
    /// Skills and organization.
    pub library: Library,
    /// Hash of `library`, checked by previewed confirmations.
    pub revision: String,
    /// Machine-local details by skill identity.
    pub details: BTreeMap<SkillId, SkillDetails>,
}
/// A machine-local organization mutation.
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
    /// Copy a preset, its appearance, and its skills into a new preset named after it.
    ///
    /// The Default preset may be cloned; the copy is an ordinary editable preset.
    ClonePreset {
        /// Preset being copied.
        id: PresetId,
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
    /// Replace the folder order with a permutation of every existing folder.
    ReorderFolders {
        /// Every folder identity in its new order.
        order: Vec<FolderId>,
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

#[cfg(test)]
mod tests {
    #[test]
    fn preset_colors_store_palette_names_and_custom_hex() -> serde_json::Result<()> {
        use super::{PresetColor, Rgb};
        let custom = Rgb::from_rgb(0x33, 0x66, 0xff);
        assert_eq!(custom.get(), 0x3366ff);
        let colors = [PresetColor::Green, PresetColor::Custom(custom)];
        assert_eq!(serde_json::to_string(&colors)?, r##"["green","#3366ff"]"##);
        let read: Vec<PresetColor> = serde_json::from_str(r##"["green","#3366FF"]"##)?;
        assert_eq!(read, colors);
        assert!(Rgb::from_u24(0x100_0000).is_none());
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
}
