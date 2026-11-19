//! Catalog format v1 (`docs/specs/explore-and-install/design.md`, Catalog format v1) and the
//! check the workflow runs on the built site before deploying it.

use super::{
    history::{History, NIGHTS},
    rules,
};
use anyhow::{Context as _, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};

pub(super) const SCHEMA: u32 = 1;
pub(super) const CATALOG: &str = "v1/catalog.json";
pub(super) const HISTORY: &str = "history.json";
/// The app refuses a larger catalog.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Deserialize, Serialize)]
pub(super) struct Catalog {
    pub(super) schema: u32,
    pub(super) generated_at: String,
    pub(super) creators: Vec<Creator>,
    pub(super) skills: Vec<Skill>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Creator {
    pub(super) id: String,
    pub(super) name: String,
    /// Relative to `v1/`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) avatar: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Skill {
    pub(super) slug: String,
    pub(super) name: String,
    pub(super) description: String,
    pub(super) creator: String,
    pub(super) repository: String,
    pub(super) path: String,
    pub(super) installs_week: u64,
}

/// The image formats GitHub serves avatars in.
#[derive(Clone, Copy)]
pub(super) enum Image {
    Png,
    Jpeg,
}

impl Image {
    pub(super) fn of(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(Self::Png)
        } else if bytes.starts_with(b"\xff\xd8\xff") {
            Some(Self::Jpeg)
        } else {
            None
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
        }
    }
}

/// The avatar path of `creator`, relative to `v1/`.
pub(super) fn avatar(creator: &str, image: Image) -> String {
    format!("avatars/{creator}.{}", image.extension())
}

/// Every rule `catalog` breaks, one line each.
pub(super) fn violations(catalog: &Catalog) -> Vec<String> {
    let mut found = Vec::new();
    let mut fail = |text: String| found.push(text);
    if catalog.schema != SCHEMA {
        fail(format!("schema is {}, not {SCHEMA}", catalog.schema));
    }
    let mut creators = BTreeSet::new();
    for creator in &catalog.creators {
        let id = &creator.id;
        if !rules::slug(id) || !creators.insert(id.as_str()) {
            fail(format!("creator {id:?} is invalid or repeated"));
        }
        if creator.name.trim().is_empty() {
            fail(format!("creator {id} has no name"));
        }
        if let Some(path) = &creator.avatar
            && ![Image::Png, Image::Jpeg]
                .iter()
                .any(|&image| *path == avatar(id, image))
        {
            fail(format!(
                "creator {id}'s avatar is not avatars/{id}.png or .jpg"
            ));
        }
    }
    let (mut identities, mut used) = (BTreeSet::new(), BTreeSet::new());
    for skill in &catalog.skills {
        let at = format!("{}#{}", skill.repository, skill.path);
        let checks = [
            (
                rules::slug(&skill.slug),
                "slug breaks the library's name rule",
            ),
            (!skill.name.trim().is_empty(), "name is empty"),
            (!skill.description.trim().is_empty(), "description is empty"),
            (
                skill.description.chars().count() <= rules::DESCRIPTION_LIMIT,
                "description is over 1,024 characters",
            ),
            (
                creators.contains(skill.creator.as_str()),
                "creator is not listed",
            ),
            (
                rules::repository(&skill.repository),
                "repository is not an HTTPS .git URL",
            ),
            (rules::path(&skill.path), "path is not a relative folder"),
            (identities.insert(at.clone()), "repository and path repeat"),
        ];
        for (_, problem) in checks.iter().filter(|(ok, _)| !ok) {
            fail(format!("skill {at}: {problem}"));
        }
        used.insert(skill.creator.as_str());
    }
    for id in creators.difference(&used) {
        fail(format!("creator {id} has no skills"));
    }
    if catalog
        .skills
        .windows(2)
        .any(|pair| pair[0].installs_week < pair[1].installs_week)
    {
        fail("skills are not ordered by installs_week, highest first".to_owned());
    }
    found
}

/// Checks the site built into `dir`: the catalog's rules and size, each listed avatar, and the
/// history's length.
pub(super) fn check_site(dir: &Path) -> anyhow::Result<()> {
    let path = dir.join(CATALOG);
    let size = fs::metadata(&path).with_context(|| format!("Missing {}", path.display()))?;
    ensure!(size.len() <= MAX_BYTES, "{CATALOG} is over 8 MB");
    let catalog: Catalog = serde_json::from_slice(&fs::read(&path)?)
        .with_context(|| format!("{CATALOG} does not parse as catalog format v1"))?;
    let mut problems = violations(&catalog);
    for creator in &catalog.creators {
        if let Some(avatar) = &creator.avatar
            && !dir.join("v1").join(avatar).is_file()
        {
            problems.push(format!(
                "creator {}'s avatar {avatar} is missing",
                creator.id
            ));
        }
    }
    ensure!(
        problems.is_empty(),
        "{CATALOG} breaks catalog format v1:\n{}",
        problems.join("\n")
    );
    let history = History::parse(&fs::read(dir.join(HISTORY))?)?;
    ensure!(
        history.nights() <= NIGHTS as usize,
        "{HISTORY} keeps more than {NIGHTS} nights"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Catalog, Creator, Skill, violations};

    fn skill(slug: &str, installs_week: u64) -> Skill {
        Skill {
            slug: slug.to_owned(),
            name: slug.to_owned(),
            description: "Does a thing.".to_owned(),
            creator: "acme".to_owned(),
            repository: "https://github.com/acme/skills.git".to_owned(),
            path: format!("skills/{slug}"),
            installs_week,
        }
    }

    fn catalog(skills: Vec<Skill>) -> Catalog {
        Catalog {
            schema: 1,
            generated_at: "2026-11-19T03:12:44Z".to_owned(),
            creators: vec![Creator {
                id: "acme".to_owned(),
                name: "Acme".to_owned(),
                avatar: Some("avatars/acme.png".to_owned()),
            }],
            skills,
        }
    }

    #[test]
    fn a_valid_catalog_has_no_violations() {
        assert!(violations(&catalog(vec![skill("a", 2), skill("b", 1)])).is_empty());
    }

    #[test]
    fn each_field_rule_is_checked() {
        let mut long = skill("long", 0);
        long.description = "x".repeat(1_025);
        let mut stray = skill("stray", 0);
        stray.creator = "nobody".to_owned();
        let mut root = skill("root", 0);
        root.path = "../root".to_owned();
        let mut http = skill("http", 0);
        http.repository = "http://github.com/acme/skills.git".to_owned();
        let skills = vec![
            skill("Bad", 0),
            long,
            stray,
            root,
            http,
            skill("a", 0),
            skill("a", 0),
            skill("late", 5),
        ];
        let found = violations(&catalog(skills)).join("\n");
        for problem in [
            "slug breaks",
            "over 1,024",
            "creator is not listed",
            "not a relative folder",
            "not an HTTPS .git URL",
            "repository and path repeat",
            "not ordered",
        ] {
            assert!(found.contains(problem), "missed {problem}:\n{found}");
        }
    }

    #[test]
    fn creators_without_skills_or_with_a_wrong_avatar_are_violations() {
        let mut empty = catalog(Vec::new());
        empty.creators[0].avatar = Some("../acme.png".to_owned());
        let found = violations(&empty).join("\n");
        assert!(found.contains("has no skills"), "{found}");
        assert!(found.contains("avatar is not"), "{found}");
    }
}
