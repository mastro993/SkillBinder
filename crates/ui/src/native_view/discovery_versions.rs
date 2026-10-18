//! Discovery's found skills: the not-in-library candidates grouped into one skill per key, each
//! with its versions and the version an import of it takes.

use skillbinder_proto::{Candidate, CandidateId, ValidationStatus};
use std::collections::BTreeMap;

/// One found skill and its versions, in the order the scan lists them.
pub(super) struct FoundSkill<'a> {
    pub key: &'a str,
    pub versions: Vec<&'a Candidate>,
}

impl<'a> FoundSkill<'a> {
    /// The version an import of this skill takes: the one the user chose, while it is still
    /// found and importable, else the preferred one; none when every version is blocked.
    pub(super) fn chosen(&self, choices: &BTreeMap<String, CandidateId>) -> Option<&'a Candidate> {
        let choice = choices.get(self.key);
        self.versions
            .iter()
            .copied()
            .filter(|version| importable(version))
            .find(|version| Some(&version.id) == choice)
            .or_else(|| {
                self.versions
                    .iter()
                    .copied()
                    .find(|version| version.preferred)
            })
    }

    /// The slug, as the chosen or first version spells it.
    pub(super) fn slug(&self, choices: &BTreeMap<String, CandidateId>) -> &'a str {
        self.chosen(choices)
            .or_else(|| self.versions.first().copied())
            .map_or(self.key, |version| version.slug.as_str())
    }
}

/// Whether a version can be imported: it is not blocked and has files.
pub(super) fn importable(version: &Candidate) -> bool {
    version.validation != ValidationStatus::Blocked && version.file_count > 0
}

/// Groups `listed` by key, keeping the order of each key's first version.
pub(super) fn found_skills<'a>(listed: &[&'a Candidate]) -> Vec<FoundSkill<'a>> {
    let mut skills: Vec<FoundSkill<'a>> = Vec::new();
    let mut index: BTreeMap<&str, usize> = BTreeMap::new();
    for &candidate in listed {
        match index.get(candidate.key.as_str()) {
            Some(&at) => skills[at].versions.push(candidate),
            None => {
                index.insert(&candidate.key, skills.len());
                skills.push(FoundSkill {
                    key: &candidate.key,
                    versions: vec![candidate],
                });
            }
        }
    }
    skills
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(id: &str, key: &str, preferred: bool, validation: ValidationStatus) -> Candidate {
        Candidate {
            id: CandidateId(id.to_owned()),
            key: key.to_owned(),
            preferred,
            matches_source: false,
            modified: None,
            skill_id: None,
            held: None,
            slug: key.to_owned(),
            description: String::new(),
            locations: Vec::new(),
            validation,
            messages: Vec::new(),
            total_bytes: 1,
            file_count: 1,
        }
    }

    #[test]
    fn versions_group_by_key_and_import_the_choice_or_the_preferred_one() {
        let a1 = version("a1", "a", false, ValidationStatus::Valid);
        let b = version("b", "b", true, ValidationStatus::Valid);
        let a2 = version("a2", "a", true, ValidationStatus::Valid);
        let a3 = version("a3", "a", false, ValidationStatus::Blocked);
        let skills = found_skills(&[&a1, &b, &a2, &a3]);
        let keys: Vec<_> = skills.iter().map(|skill| skill.key).collect();
        assert_eq!(keys, ["a", "b"]);
        assert_eq!(skills[0].versions.len(), 3);
        let chosen = |choices: &BTreeMap<String, CandidateId>| {
            skills[0]
                .chosen(choices)
                .map(|version| version.id.0.clone())
        };
        let mut choices = BTreeMap::new();
        assert_eq!(chosen(&choices).as_deref(), Some("a2"));
        choices.insert("a".to_owned(), CandidateId("a1".to_owned()));
        assert_eq!(chosen(&choices).as_deref(), Some("a1"));
        // A blocked or vanished choice falls back to the preferred version.
        choices.insert("a".to_owned(), CandidateId("a3".to_owned()));
        assert_eq!(chosen(&choices).as_deref(), Some("a2"));
    }

    #[test]
    fn a_skill_whose_versions_are_all_blocked_has_no_choice() {
        let blocked = version("x", "x", false, ValidationStatus::Blocked);
        let skills = found_skills(&[&blocked]);
        assert!(skills[0].chosen(&BTreeMap::new()).is_none());
        assert_eq!(skills[0].slug(&BTreeMap::new()), "x");
    }
}
