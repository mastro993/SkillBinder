use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingTarget {
    pub path: PathBuf,
    pub reader_agent_ids: Vec<String>,
}

pub fn coalesce_targets(
    targets: impl IntoIterator<Item = (PathBuf, String)>,
) -> Vec<BindingTarget> {
    let mut grouped: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for (path, agent) in targets {
        let readers = grouped.entry(path).or_default();
        if !readers.contains(&agent) {
            readers.push(agent);
        }
    }
    grouped
        .into_iter()
        .map(|(path, mut reader_agent_ids)| {
            reader_agent_ids.sort();
            BindingTarget {
                path,
                reader_agent_ids,
            }
        })
        .collect()
}

pub fn has_conflicting_destinations<'a>(
    targets: impl IntoIterator<Item = (&'a str, &'a PathBuf)>,
) -> bool {
    let mut owners = HashMap::new();
    targets.into_iter().any(|(skill_id, path)| {
        owners
            .insert(path, skill_id)
            .is_some_and(|owner| owner != skill_id)
    })
}

pub fn is_safe_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug != "."
        && slug != ".."
        && !slug.bytes().any(|byte| matches!(byte, b'/' | b'\\' | b':'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_path_has_one_physical_target_and_all_readers() {
        let targets = coalesce_targets([
            (
                PathBuf::from("/project/.agents/skills/review"),
                "codex".into(),
            ),
            (
                PathBuf::from("/project/.agents/skills/review"),
                "cline".into(),
            ),
            (
                PathBuf::from("/project/.agents/skills/review"),
                "codex".into(),
            ),
        ]);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].reader_agent_ids, ["cline", "codex"]);
    }

    #[test]
    fn different_skills_cannot_claim_one_destination() {
        let path = PathBuf::from("/project/.agents/skills/review");
        assert!(has_conflicting_destinations([
            ("first", &path),
            ("second", &path)
        ]));
        assert!(!has_conflicting_destinations([
            ("first", &path),
            ("first", &path)
        ]));
    }

    #[test]
    fn skill_slug_cannot_escape_its_agent_folder() {
        assert!(is_safe_slug("review-code"));
        for slug in ["", ".", "..", "../elsewhere", "a/b", "a\\b", "C:drive"] {
            assert!(!is_safe_slug(slug));
        }
    }
}
