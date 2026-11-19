//! The anticorruption layer for the CDN's catalog format v1: its JSON never leaves this module.
//! An entry that breaks a field rule is dropped and the rest are kept.

use crate::{git, library::normalized, payload::portable};
use serde::Deserialize;
use skillbinder_proto::{
    Catalog, CatalogOrigin, CatalogSkill, Creator, SkillSource, normalized_url,
};
use std::collections::BTreeSet;

/// The longest description kept, in characters.
const DESCRIPTION: usize = 1024;

/// Why a file is not a v1 catalog.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum Refusal {
    /// It is JSON with a `schema` other than 1.
    Outdated,
    /// It is not a JSON object.
    Unreadable,
}

/// A v1 catalog, its creators still naming their avatar files on the source.
#[derive(Debug)]
pub(super) struct Translated {
    pub catalog: Catalog,
    /// Each creator's avatar path relative to `v1/`, by creator id.
    pub avatars: Vec<(String, String)>,
}

#[derive(Deserialize)]
struct File {
    schema: serde_json::Value,
    #[serde(default)]
    creators: Vec<serde_json::Value>,
    #[serde(default)]
    skills: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct RawCreator {
    id: String,
    name: String,
    avatar: Option<String>,
}

#[derive(Deserialize)]
struct RawSkill {
    slug: String,
    name: String,
    description: String,
    creator: String,
    repository: String,
    path: String,
    installs_week: u64,
}

/// Translates a v1 catalog file, applying its field rules.
pub(super) fn translate(bytes: &[u8]) -> Result<Translated, Refusal> {
    let file: File = serde_json::from_slice(bytes).map_err(|_| Refusal::Unreadable)?;
    if file.schema != 1 {
        return Err(Refusal::Outdated);
    }
    let mut creators: Vec<RawCreator> = Vec::new();
    for raw in file.creators {
        let Ok(creator) = serde_json::from_value::<RawCreator>(raw) else {
            continue;
        };
        if file_safe(&creator.id)
            && !creator.name.trim().is_empty()
            && !creators.iter().any(|known| known.id == creator.id)
        {
            creators.push(creator);
        }
    }
    let mut seen = BTreeSet::new();
    let mut skills: Vec<CatalogSkill> = file
        .skills
        .into_iter()
        .filter_map(|raw| serde_json::from_value::<RawSkill>(raw).ok())
        .filter(|skill| creators.iter().any(|creator| creator.id == skill.creator))
        .filter_map(skill)
        .filter(|skill| seen.insert((normalized_url(&skill.source.url), skill.source.path.clone())))
        .collect();
    skills.sort_by_key(|skill| std::cmp::Reverse(skill.installs_week));
    let mut avatars = Vec::new();
    let creators = creators
        .into_iter()
        .filter(|creator| {
            skills.iter().any(|skill| {
                matches!(&skill.origin, CatalogOrigin::Curated { creator: id } if *id == creator.id)
            })
        })
        .map(|creator| {
            if let Some(avatar) = creator
                .avatar
                .filter(|path| relative(path) && !path.is_empty())
            {
                avatars.push((creator.id.clone(), avatar));
            }
            Creator {
                id: creator.id,
                name: creator.name,
                avatar: None,
            }
        })
        .collect();
    Ok(Translated {
        catalog: Catalog { creators, skills },
        avatars,
    })
}

/// The skill an entry describes, or `None` when it breaks a field rule.
fn skill(raw: RawSkill) -> Option<CatalogSkill> {
    let https = url::Url::parse(&raw.repository).is_ok_and(|url| url.scheme() == "https");
    if !https || !raw.repository.ends_with(".git") || git::validate_url(&raw.repository).is_err() {
        return None;
    }
    if !relative(&raw.path) || !portable(&raw.slug) || raw.name.trim().is_empty() {
        return None;
    }
    Some(CatalogSkill {
        key: normalized(&raw.slug),
        slug: raw.slug,
        name: raw.name,
        description: raw.description.chars().take(DESCRIPTION).collect(),
        origin: CatalogOrigin::Curated {
            creator: raw.creator,
        },
        source: SkillSource {
            url: raw.repository,
            reference: None,
            path: raw.path,
        },
        installs_week: raw.installs_week,
    })
}

/// A relative path with `/` separators and no empty, `.`, or `..` segment; empty for the root.
fn relative(path: &str) -> bool {
    path.is_empty()
        || path
            .split('/')
            .all(|segment| !matches!(segment, "" | "." | "..") && !segment.contains('\\'))
}

/// A creator id that can name its avatar file: ASCII letters, digits, `-`, `_`, and `.`, not
/// starting with `.`.
fn file_safe(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('.')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use serde_json::json;

    fn entry(slug: &str, path: &str, installs: u64) -> serde_json::Value {
        json!({
            "slug": slug, "name": slug, "description": "Does things.", "creator": "acme",
            "repository": "https://github.com/acme/skills.git", "path": path,
            "installs_week": installs,
        })
    }

    fn translated(skills: Vec<serde_json::Value>) -> Translated {
        let file = json!({
            "schema": 1,
            "creators": [
                {"id": "acme", "name": "Acme", "avatar": "avatars/acme.png"},
                {"id": "idle", "name": "Idle"},
            ],
            "skills": skills,
        });
        translate(file.to_string().as_bytes()).unwrap()
    }

    #[test]
    fn entries_breaking_a_rule_are_dropped_and_the_rest_kept() {
        let mut long = entry("long", "skills/long", 1);
        long["description"] = json!("x".repeat(2000));
        let mut ssh = entry("ssh", "skills/ssh", 1);
        ssh["repository"] = json!("git@github.com:acme/skills.git");
        let mut stranger = entry("stranger", "skills/stranger", 1);
        stranger["creator"] = json!("nobody");
        let mut negative = entry("negative", "skills/negative", 1);
        negative["installs_week"] = json!(-3);
        let translated = translated(vec![
            entry("pdf", "skills/pdf", 5),
            entry("escape", "../skills/escape", 1),
            entry("bad/slug", "skills/bad", 1),
            json!({"slug": "partial"}),
            ssh,
            stranger,
            negative,
            entry("first", "skills/same", 1),
            entry("second", "skills/same", 1),
            entry("root", "", 9),
            long,
        ]);
        let slugs: Vec<_> = translated
            .catalog
            .skills
            .iter()
            .map(|skill| skill.slug.as_str())
            .collect();
        assert_eq!(slugs, ["root", "pdf", "first", "long"]);
        let long = &translated.catalog.skills[3];
        assert_eq!(long.description.chars().count(), DESCRIPTION);
        // A creator left with no valid skills is dropped.
        let creators: Vec<_> = translated.catalog.creators.iter().map(|c| &c.id).collect();
        assert_eq!(creators, ["acme"]);
        assert_eq!(
            translated.avatars,
            [("acme".to_owned(), "avatars/acme.png".to_owned())]
        );
    }

    #[test]
    fn another_schema_is_outdated_and_garbage_unreadable() {
        let outdated = json!({"schema": 2, "creators": [], "skills": []});
        assert_eq!(
            translate(outdated.to_string().as_bytes()).err(),
            Some(Refusal::Outdated)
        );
        assert_eq!(translate(b"<html>").err(), Some(Refusal::Unreadable));
    }
}
