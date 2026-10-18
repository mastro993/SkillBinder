//! The `skills` CLI's lock files, which record where each skill it installed was published: the
//! global `.skill-lock.json` beside the universal agents' directory, and a project's
//! `skills-lock.json`. SkillBinder only reads them: for the source of a skill it imports, and for
//! whether a found skill's content is what its source published.

use crate::lifecycle::EngineConfig;
use serde::Deserialize;
use skillbinder_proto::SkillSource;
use std::{collections::BTreeMap, fs, path::Path, path::PathBuf};

/// The lock file of a project at `root`.
pub(crate) fn project_lock(root: &Path) -> PathBuf {
    root.join("skills-lock.json")
}

/// The CLI's global lock file, under `$XDG_STATE_HOME` when that is set.
pub(crate) fn global_lock(config: &EngineConfig) -> PathBuf {
    match config.environment.get("XDG_STATE_HOME") {
        Some(state) if Path::new(state).is_absolute() => {
            Path::new(state).join("skills/.skill-lock.json")
        }
        _ => config.home.join(".agents/.skill-lock.json"),
    }
}

/// A parsed lock file.
#[derive(Deserialize)]
pub(crate) struct Lock {
    skills: BTreeMap<String, Entry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    source: String,
    source_type: String,
    source_url: Option<String>,
    #[serde(rename = "ref")]
    reference: Option<String>,
    skill_path: Option<String>,
    /// The Git tree id of the published folder, for GitHub sources; empty when unknown.
    skill_folder_hash: Option<String>,
}

/// The lock file at `path`; `None` when it is absent or unreadable.
pub(crate) fn read(path: &Path) -> Option<Lock> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

/// The Git source the lock file at `path` records for `slug`; `None` when the lock is absent or
/// unreadable, has no entry for it, or records a source that is not a Git repository.
pub(crate) fn source(path: &Path, slug: &str) -> Option<SkillSource> {
    read(path)?.source(slug)
}

impl Lock {
    /// The Git tree id the lock records as `slug`'s published content, if it records one.
    pub(crate) fn tree(&self, slug: &str) -> Option<&str> {
        self.skills
            .get(slug)?
            .skill_folder_hash
            .as_deref()
            .filter(|hash| !hash.is_empty())
    }

    /// The Git source the lock records for `slug`, if it is a Git repository.
    pub(crate) fn source(&self, slug: &str) -> Option<SkillSource> {
        let entry = self.skills.get(slug)?;
        let url = match (entry.source_type.as_str(), entry.source_url.as_deref()) {
            // The normalized `owner/repo`; the recorded URL may be a browser link into the tree.
            ("github", _) => format!("https://github.com/{}.git", entry.source),
            ("gitlab", Some(url)) if url.contains("/-/") => {
                format!("{}.git", url.split("/-/").next()?.trim_end_matches('/'))
            }
            ("gitlab", None) => format!("https://gitlab.com/{}.git", entry.source),
            ("gitlab" | "git", url) => url.unwrap_or(&entry.source).to_owned(),
            _ => return None,
        };
        let folder = entry.skill_path.as_deref()?.strip_suffix("SKILL.md")?;
        let source = SkillSource {
            url,
            reference: entry
                .reference
                .clone()
                .filter(|reference| !reference.is_empty()),
            path: folder.trim_end_matches('/').to_owned(),
        };
        crate::git::validate_source(&source).ok()?;
        Some(source)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn read(json: &str, slug: &str) -> Option<SkillSource> {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("lock.json");
        fs::write(&path, json).unwrap();
        source(&path, slug)
    }

    #[test]
    fn github_entries_name_their_repository_and_folder() {
        let global = r#"{"version":3,"skills":{"ads":{"source":"o/r","sourceType":"github",
            "sourceUrl":"https://github.com/o/r/tree/main/skills/ads","skillPath":"skills/ads/SKILL.md",
            "skillFolderHash":"abc"}}}"#;
        assert_eq!(
            read(global, "ads"),
            Some(SkillSource {
                url: "https://github.com/o/r.git".into(),
                reference: None,
                path: "skills/ads".into(),
            })
        );
        let root = r#"{"version":1,"skills":{"x":{"source":"o/r","ref":"v2","sourceType":"github",
            "skillPath":"SKILL.md","computedHash":"abc"}}}"#;
        assert_eq!(
            read(root, "x"),
            Some(SkillSource {
                url: "https://github.com/o/r.git".into(),
                reference: Some("v2".into()),
                path: String::new(),
            })
        );
    }

    #[test]
    fn git_entries_keep_their_url() {
        let lock = r#"{"version":1,"skills":{"x":{"source":"git@host:o/r.git","sourceType":"git",
            "sourceUrl":"git@host:o/r.git","skillPath":"a/x/SKILL.md","computedHash":""}}}"#;
        assert_eq!(read(lock, "x").unwrap().url, "git@host:o/r.git");
        let tree = r#"{"version":1,"skills":{"x":{"source":"g/r","sourceType":"gitlab",
            "sourceUrl":"https://gitlab.com/g/r/-/tree/main/x","skillPath":"x/SKILL.md"}}}"#;
        assert_eq!(read(tree, "x").unwrap().url, "https://gitlab.com/g/r.git");
        let bare = r#"{"version":1,"skills":{"x":{"source":"g/r","sourceType":"gitlab",
            "skillPath":"x/SKILL.md"}}}"#;
        assert_eq!(read(bare, "x").unwrap().url, "https://gitlab.com/g/r.git");
    }

    #[test]
    fn other_sources_and_missing_entries_have_none() {
        for source_type in ["local", "well-known", "node_modules"] {
            let lock = format!(
                r#"{{"version":1,"skills":{{"x":{{"source":"./s","sourceType":"{source_type}",
                "skillPath":"x/SKILL.md"}}}}}}"#
            );
            assert_eq!(read(&lock, "x"), None, "{source_type}");
        }
        let lock = r#"{"version":3,"skills":{"x":{"source":"o/r","sourceType":"github"}}}"#;
        assert_eq!(read(lock, "x"), None, "no folder");
        assert_eq!(read(lock, "y"), None, "no entry");
        assert_eq!(read("not json", "x"), None);
        let escaping = r#"{"version":3,"skills":{"x":{"source":"o/r","sourceType":"github",
            "skillPath":"../x/SKILL.md"}}}"#;
        assert_eq!(read(escaping, "x"), None);
    }

    #[test]
    fn the_published_tree_is_the_folder_hash() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("lock.json");
        fs::write(
            &path,
            r#"{"version":3,"skills":{"a":{"source":"o/r","sourceType":"github",
            "skillPath":"a/SKILL.md","skillFolderHash":"abc"},"b":{"source":"o/r",
            "sourceType":"github","skillPath":"b/SKILL.md","skillFolderHash":""}}}"#,
        )
        .unwrap();
        let lock = super::read(&path).unwrap();
        assert_eq!(lock.tree("a"), Some("abc"));
        assert_eq!(lock.tree("b"), None);
        assert_eq!(lock.tree("c"), None);
    }
}
