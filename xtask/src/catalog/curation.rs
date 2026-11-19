//! `catalog/curated.toml`: the creators and the repositories the catalog lists.

use super::rules;
use anyhow::{Context as _, bail, ensure};
use serde::Deserialize;
use std::collections::BTreeSet;

const GITHUB: &str = "https://github.com/";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(rename = "creator")]
    creators: Vec<RawCreator>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCreator {
    id: String,
    name: String,
    github: String,
    repositories: Vec<String>,
    #[serde(default)]
    exclude: Vec<RawExclude>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawExclude {
    repository: String,
    paths: Vec<String>,
}

pub(super) struct Creator {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) github: String,
    pub(super) repositories: Vec<Repository>,
}

pub(super) struct Repository {
    /// `https://github.com/<owner>/<name>.git`, as curated.
    pub(super) url: String,
    pub(super) owner: String,
    pub(super) name: String,
    /// Skill folders left out of the catalog.
    pub(super) excluded: BTreeSet<String>,
}

impl Repository {
    /// skills.sh's `source` for this repository.
    pub(super) fn source(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    fn parse(url: &str) -> anyhow::Result<Self> {
        let invalid = || format!("{url} is not https://github.com/<owner>/<repository>.git");
        let path = url
            .strip_prefix(GITHUB)
            .and_then(|rest| rest.strip_suffix(".git"))
            .with_context(invalid)?;
        let Some((owner, name)) = path.split_once('/') else {
            bail!(invalid());
        };
        ensure!(segment(owner) && segment(name), invalid());
        Ok(Self {
            url: url.to_owned(),
            owner: owner.to_owned(),
            name: name.to_owned(),
            excluded: BTreeSet::new(),
        })
    }
}

/// A GitHub owner or repository name: no separators, nothing Git or a URL would reinterpret.
fn segment(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with(['-', '.'])
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

pub(super) fn parse(text: &str) -> anyhow::Result<Vec<Creator>> {
    let file: File = toml::from_str(text).context("catalog/curated.toml is malformed")?;
    let (mut ids, mut urls) = (BTreeSet::new(), BTreeSet::new());
    let mut creators = Vec::new();
    for raw in file.creators {
        let id = raw.id;
        ensure!(rules::slug(&id), "Creator id {id:?} is not a valid slug");
        ensure!(ids.insert(id.clone()), "Creator {id} is curated twice");
        ensure!(!raw.name.trim().is_empty(), "Creator {id} has no name");
        ensure!(
            segment(&raw.github) && !raw.github.contains(['.', '_']),
            "Creator {id} has an invalid GitHub login"
        );
        ensure!(
            !raw.repositories.is_empty(),
            "Creator {id} has no repositories"
        );
        let mut repositories = Vec::new();
        for url in &raw.repositories {
            let repository = Repository::parse(url)?;
            ensure!(
                urls.insert(url.to_ascii_lowercase()),
                "{url} is curated twice"
            );
            repositories.push(repository);
        }
        for exclude in raw.exclude {
            let Some(repository) = repositories
                .iter_mut()
                .find(|repository| repository.url == exclude.repository)
            else {
                bail!(
                    "Creator {id} excludes paths from {}, which it does not list",
                    exclude.repository
                );
            };
            for path in exclude.paths {
                ensure!(
                    rules::path(&path),
                    "Excluded path {path:?} is not a skill folder"
                );
                repository.excluded.insert(path);
            }
        }
        creators.push(Creator {
            id,
            name: raw.name,
            github: raw.github,
            repositories,
        });
    }
    Ok(creators)
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "Test assertions report fixture failures."
    )]
    use super::parse;

    const VALID: &str = r#"
[[creator]]
id = "acme"
name = "Acme"
github = "acme"
repositories = ["https://github.com/acme/skills.git"]

[[creator.exclude]]
repository = "https://github.com/acme/skills.git"
paths = ["skills/draft"]
"#;

    #[test]
    fn creators_repositories_and_exclusions_are_read() {
        let creators = parse(VALID).expect("valid curation");
        let [creator] = creators.as_slice() else {
            panic!("one creator expected");
        };
        let [repository] = creator.repositories.as_slice() else {
            panic!("one repository expected");
        };
        assert_eq!(repository.source(), "acme/skills");
        assert!(repository.excluded.contains("skills/draft"));
    }

    #[test]
    fn invalid_curation_is_refused() {
        for (from, to) in [
            (
                "https://github.com/acme/skills.git\"]",
                "https://github.com/acme/skills\"]",
            ),
            (
                "https://github.com/acme/skills.git\"]",
                "http://github.com/acme/skills.git\"]",
            ),
            (
                "https://github.com/acme/skills.git\"]",
                "https://u@github.com/acme/skills.git\"]",
            ),
            ("id = \"acme\"", "id = \"Acme\""),
            ("github = \"acme\"", "github = \"-acme\""),
            ("paths = [\"skills/draft\"]", "paths = [\"../draft\"]"),
            (
                "repository = \"https://github.com/acme/skills.git\"",
                "repository = \"https://github.com/acme/other.git\"",
            ),
            ("name = \"Acme\"", "name = \"Acme\"\nwebsite = \"x\""),
        ] {
            let text = VALID.replacen(from, to, 1);
            assert_ne!(text, VALID, "{from} did not apply");
            assert!(parse(&text).is_err(), "accepted {to}");
        }
        assert!(
            parse(&format!("{VALID}{VALID}")).is_err(),
            "accepted a creator twice"
        );
    }
}
