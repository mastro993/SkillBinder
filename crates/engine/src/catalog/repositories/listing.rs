//! Listing a repository's skills on the Git pool, never the worker (design T6): a bare, shallow,
//! blobless fetch into its catalog cache, its file names from Git's trees alone, the `skills`
//! CLI's discovery rules, and each SKILL.md's front matter, which fetches only those blobs.

use super::super::folders::skill_folders;
use crate::{
    catalog::Previews,
    git::Git,
    library::normalized,
    logging::{Area, Level, Logger},
    payload::{self, portable},
};
use serde::{Deserialize, Serialize};
use skillbinder_proto::{
    AppResult, CatalogOrigin, CatalogSkill, ErrorCategory, RepositoryId, RepositoryProblem,
    SkillSource,
};
use std::{collections::BTreeSet, path::PathBuf};

/// The longest description kept, in characters, as the catalog keeps.
const DESCRIPTION: usize = 1024;

/// One skill a listing found, as a repository keeps it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct Listed {
    /// Its SKILL.md name, or its folder's name without a usable one.
    pub slug: String,
    pub name: String,
    pub description: String,
    /// Its folder in the repository, empty for a root SKILL.md.
    pub path: String,
}

impl Listed {
    /// The skill as Explore shows it, from repository `id` at `url`.
    pub(crate) fn skill(&self, id: &RepositoryId, url: &str) -> CatalogSkill {
        CatalogSkill {
            slug: self.slug.clone(),
            key: normalized(&self.slug),
            name: self.name.clone(),
            description: self.description.clone(),
            origin: CatalogOrigin::Repository { id: id.clone() },
            source: SkillSource {
                url: url.to_owned(),
                reference: None,
                path: self.path.clone(),
            },
            installs_week: 0,
        }
    }

    /// The skill a repository's listing holds.
    pub(crate) fn of(skill: &CatalogSkill) -> Self {
        Self {
            slug: skill.slug.clone(),
            name: skill.name.clone(),
            description: skill.description.clone(),
            path: skill.source.path.clone(),
        }
    }
}

/// A listing's work on the Git pool.
pub(crate) struct Listing {
    pub git: Git,
    /// The URL as the user entered it.
    pub url: String,
    pub cache: PathBuf,
    /// Holds the cache's lock, shared with previews and pruning.
    pub previews: Previews,
    /// The registry's workspace skill directories, which discovery searches.
    pub agent_dirs: Vec<String>,
    pub log: Logger,
}

impl Listing {
    /// The repository's skills, possibly none; unreachable when Git could not fetch it.
    pub(crate) fn run(self) -> Result<Vec<Listed>, RepositoryProblem> {
        let lock = self.previews.cache_lock(&self.cache);
        let _reading = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.list().map_err(|error| {
            self.log
                .failure(
                    Level::Warn,
                    Area::Explore,
                    (
                        "repository listing failed",
                        "repository listing stopped at quit",
                    ),
                    &error,
                )
                .field("url", &self.url);
            match error.category {
                ErrorCategory::Offline | ErrorCategory::Git => RepositoryProblem::Unreachable,
                _ => RepositoryProblem::Failed(error),
            }
        })
    }

    fn list(&self) -> AppResult<Vec<Listed>> {
        let source = SkillSource {
            url: self.url.clone(),
            reference: None,
            path: String::new(),
        };
        self.git.fetch_source(&self.cache, &source)?;
        let Some(root) = self.git.source_tree(&self.cache, "")? else {
            return Ok(Vec::new());
        };
        // Symbolic links and submodules are never skills.
        let files: Vec<_> = self
            .git
            .tree_entries(&self.cache, &root)?
            .into_iter()
            .filter(|entry| {
                entry.kind == "blob" && matches!(entry.mode.as_str(), "100644" | "100755")
            })
            .collect();
        let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
        let folders = skill_folders(&paths, &self.agent_dirs);
        let skills: Vec<_> = folders
            .into_iter()
            .filter_map(|path| {
                let skill_md = if path.is_empty() {
                    "SKILL.md".to_owned()
                } else {
                    format!("{path}/SKILL.md")
                };
                let file = files.iter().find(|file| file.path == skill_md)?;
                Some((path, file.object.as_str()))
            })
            .collect();
        let objects: Vec<&str> = skills.iter().map(|(_, object)| *object).collect();
        // One request instead of one per SKILL.md; reading each still fetches any it missed.
        if let Err(error) = self.git.fetch_blobs(&self.cache, &objects)
            && matches!(
                error.category,
                ErrorCategory::Offline | ErrorCategory::Cancelled
            )
        {
            return Err(error);
        }
        let mut keys = BTreeSet::new();
        let mut listed = Vec::new();
        for (path, object) in skills {
            // Any SKILL.md Git cannot read fails the listing, so a partial one never replaces the
            // last; one that reads but is not text has no name and keeps its folder's.
            let text = self.git.read_blob_bytes(&self.cache, object)?;
            if let Some(skill) = listed_skill(path, &text)
                && keys.insert(normalized(&skill.slug))
            {
                listed.push(skill);
            }
        }
        Ok(listed)
    }
}

/// The skill in folder `path` whose SKILL.md is `skill_md`: its slug is its front matter name,
/// or its folder's name, whichever the library could hold first; `None` when neither can.
fn listed_skill(path: &str, skill_md: &[u8]) -> Option<Listed> {
    let (name, description) = payload::name_and_description(skill_md).unwrap_or_default();
    let folder = path.rsplit('/').next().unwrap_or_default();
    let slug = name
        .clone()
        .filter(|name| portable(name))
        .or_else(|| portable(folder).then(|| folder.to_owned()))?;
    Some(Listed {
        name: name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| slug.clone()),
        slug,
        description: description
            .unwrap_or_default()
            .chars()
            .take(DESCRIPTION)
            .collect(),
        path: path.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::listed_skill;

    #[test]
    fn a_slug_is_the_front_matter_name_else_the_folder_name() {
        let named = listed_skill(
            "skills/pdf",
            b"---\nname: pdf-tools\ndescription: PDFs.\n---\n",
        );
        let named = named.unwrap();
        assert_eq!(
            (named.slug.as_str(), named.path.as_str()),
            ("pdf-tools", "skills/pdf")
        );
        assert_eq!(named.description, "PDFs.");
        let unnamed = listed_skill("skills/docx", b"# Word documents\n").unwrap();
        assert_eq!(
            (unnamed.slug.as_str(), unnamed.name.as_str()),
            ("docx", "docx")
        );
        let unusable = listed_skill("skills/a:b", b"---\nname: ../up\n---\n");
        assert!(unusable.is_none());
    }
}
