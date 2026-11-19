//! All-time install counts from skills.sh's search endpoint, one query per repository.
//!
//! The endpoint is undocumented: `GET /api/search?q=<text>&owner=<owner>&limit=<n>` answers
//! `{skills: [{id, source: "owner/repo", skillId, name, installs}], …}` and matches `q` against
//! skill and repository names. A response without that shape fails the run, so the change is
//! noticed (ADR 0014).

use super::{
    curation::Repository,
    http::{Fetched, Http},
};
use anyhow::{Context as _, bail};
use serde::Deserialize;
use std::collections::BTreeMap;

pub(super) const BASE: &str = "https://skills.sh/";
/// The most results the endpoint returns.
const LIMIT: u32 = 200;
const MAX_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Deserialize)]
struct Response {
    skills: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    source: String,
    skill_id: String,
    installs: u64,
}

/// The search URL for `repository`'s skills. The endpoint refuses a query under 2 characters.
pub(super) fn url(base: &str, repository: &Repository) -> String {
    let query = if repository.name.len() < 2 {
        repository.source()
    } else {
        repository.name.clone()
    };
    format!(
        "{base}api/search?q={query}&owner={}&limit={LIMIT}",
        repository.owner
    )
}

/// `repository`'s installs by skill slug. A skill with no entry has none listed.
pub(super) fn fetch(
    http: &dyn Http,
    base: &str,
    repository: &Repository,
) -> anyhow::Result<BTreeMap<String, u64>> {
    let url = url(base, repository);
    let Fetched::Found(body) = http.get(&url, MAX_BYTES)? else {
        bail!("GET {url} answered HTTP 404");
    };
    let response: Response = serde_json::from_slice(&body)
        .with_context(|| format!("skills.sh changed its search response ({url})"))?;
    let source = repository.source();
    Ok(response
        .skills
        .into_iter()
        .filter(|entry| entry.source.eq_ignore_ascii_case(&source))
        .map(|entry| (entry.skill_id, entry.installs))
        .collect())
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "Test assertions report fixture failures."
    )]
    use super::{super::curation, super::http, fetch, url};
    use anyhow::bail;

    struct Saved(&'static str);

    impl http::Http for Saved {
        fn get(&self, url: &str, _limit: u64) -> anyhow::Result<http::Fetched> {
            if url != "https://skills.sh/api/search?q=skills&owner=acme&limit=200" {
                bail!("unexpected {url}");
            }
            Ok(http::Fetched::Found(self.0.as_bytes().to_vec()))
        }
    }

    fn repository() -> curation::Repository {
        let text = "[[creator]]\nid = \"acme\"\nname = \"Acme\"\ngithub = \"acme\"\n\
                    repositories = [\"https://github.com/acme/skills.git\"]\n";
        let mut creators = curation::parse(text).expect("valid curation");
        creators.remove(0).repositories.remove(0)
    }

    #[test]
    fn entries_count_for_their_repository_only() {
        let saved = Saved(
            r#"{"query":"skills","skills":[
                {"id":"acme/skills/tdd","source":"acme/skills","skillId":"tdd","name":"tdd","installs":12},
                {"id":"Acme/Skills/plan","source":"Acme/Skills","skillId":"plan","name":"plan","installs":3},
                {"id":"fork/skills/tdd","source":"fork/skills","skillId":"tdd","name":"tdd","installs":99}
            ]}"#,
        );
        let counts = fetch(&saved, "https://skills.sh/", &repository()).expect("counts");
        assert_eq!(counts.get("tdd"), Some(&12));
        assert_eq!(counts.get("plan"), Some(&3));
        assert_eq!(counts.len(), 2);
    }

    #[test]
    fn a_changed_shape_fails() {
        for body in [
            r#"{"results":[]}"#,
            r#"{"skills":[{"source":"acme/skills"}]}"#,
            "<html>",
        ] {
            assert!(fetch(&Saved(body), "https://skills.sh/", &repository()).is_err());
        }
    }

    #[test]
    fn one_letter_repositories_are_searched_by_their_source() {
        let mut repository = repository();
        repository.name = "x".to_owned();
        assert!(url("https://skills.sh/", &repository).contains("q=acme/x&"));
    }
}
