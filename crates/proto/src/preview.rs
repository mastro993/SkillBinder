//! A catalog skill's preview: what its detail dialog shows of the skill before it is installed,
//! read from its source with Git (ADR 0014).

use crate::SkillSource;
use std::sync::Arc;

/// What the source's folder holds, as Install would put it in the library.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogPreview {
    /// Each file's path under the folder, `/`-separated, listed from Git's trees alone.
    pub files: Vec<String>,
    /// The folder's SKILL.md, its first 256 KB when longer.
    pub skill_md: String,
    /// SKILL.md's front matter fields other than its name and description, in their order.
    pub front_matter: Vec<FrontMatterField>,
}

/// One front matter field, such as `license` or `allowed-tools`. A `metadata` mapping gives one
/// field per entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontMatterField {
    /// The key as SKILL.md writes it.
    pub key: String,
    /// Its value.
    pub value: FrontMatterValue,
}

/// A front matter value the detail dialog can show.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrontMatterValue {
    /// A scalar, as text.
    Text(String),
    /// A list of scalars, such as `allowed-tools`, whose string form is split into tools.
    List(Vec<String>),
}

/// Where the open dialog's preview stands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreviewState {
    /// The source is being fetched.
    Loading,
    /// The folder's files and its SKILL.md.
    Loaded(Arc<CatalogPreview>),
    /// The fetched commit has no folder at the skill's path, or the folder has no SKILL.md.
    NotFound,
    /// Git is missing, so nothing was fetched.
    NeedsGit,
    /// The source could not be fetched or read; Retry tries again.
    Failed,
}

/// The preview of the skill whose detail dialog is open.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Preview {
    /// The skill's source, which names the dialog it belongs to.
    pub source: SkillSource,
    /// Where its preview stands.
    pub state: PreviewState,
}
