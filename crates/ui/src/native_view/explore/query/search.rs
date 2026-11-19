//! What a search finds, derived from the snapshot alone: the catalog's skills, then your
//! repositories' by name, a skill both curated and in one of your repositories shown once, as
//! curated; and the source chips over them, a creator's or a repository's, each with its count.

use super::{HOME_CHIPS, by, creator_of, popular_creators};
use skillbinder_proto::{
    Catalog, CatalogOrigin, CatalogSkill, Creator, Repository, RepositoryId, owner_repo,
    same_source,
};
use std::cmp::Reverse;

/// A source Explore can be narrowed to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_view) enum Source {
    /// A creator, by id, picked on a chip, a creator chip, a result's source tag, or a detail
    /// dialog.
    Creator(String),
    /// One of your repositories, picked on its chip or a result's source tag.
    Repository(RepositoryId),
    /// skills.sh's results, picked on its chip.
    SkillsSh,
}

/// What a source chip stands for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_view) enum Chip<'a> {
    Creator(&'a Creator),
    Repository(&'a Repository),
}

impl Chip<'_> {
    /// The source picking the chip narrows to.
    pub(in crate::native_view) fn source(self) -> Source {
        match self {
            Self::Creator(creator) => Source::Creator(creator.id.clone()),
            Self::Repository(repository) => Source::Repository(repository.id.clone()),
        }
    }
}

/// Where search looks: the catalog and your repositories.
#[derive(Clone, Copy)]
pub(in crate::native_view) struct Local<'a> {
    pub catalog: &'a Catalog,
    pub repositories: &'a [Repository],
}

impl<'a> Local<'a> {
    /// The repository `skill` was listed from, if it is one of your repositories'.
    pub(in crate::native_view) fn repository_of(
        self,
        skill: &CatalogSkill,
    ) -> Option<&'a Repository> {
        let CatalogOrigin::Repository { id } = &skill.origin else {
            return None;
        };
        self.repository(id)
    }

    /// Your repository `id`.
    pub(in crate::native_view) fn repository(self, id: &RepositoryId) -> Option<&'a Repository> {
        self.repositories
            .iter()
            .find(|repository| &repository.id == id)
    }

    /// Whether `skill` matches `search`, already lowercased, by its name, description, creator,
    /// or repository's `owner/repo`, so a search for its host or scheme matches nothing.
    fn matches(self, skill: &CatalogSkill, search: &str) -> bool {
        let creator = creator_of(self.catalog, skill);
        let repository = owner_repo(&skill.source.url);
        [
            skill.name.as_str(),
            skill.description.as_str(),
            repository.as_str(),
        ]
        .into_iter()
        .chain(creator.map(|creator| creator.name.as_str()))
        .chain(creator.map(|creator| creator.id.as_str()))
        .any(|text| text.to_lowercase().contains(search))
    }

    /// The catalog's skills matching `search`, most installs this week first.
    fn curated(self, search: &'a str) -> impl Iterator<Item = &'a CatalogSkill> {
        self.catalog
            .skills
            .iter()
            .filter(move |skill| self.matches(skill, search))
    }

    /// Repository `repository`'s skills matching `search`, by name.
    fn listed(self, repository: &'a Repository, search: &str) -> Vec<&'a CatalogSkill> {
        let mut skills: Vec<_> = repository
            .skills
            .iter()
            .filter(|skill| self.matches(skill, search))
            .collect();
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        skills
    }

    /// The results for `search`, already lowercased, narrowed to `picked` when a source is
    /// picked: the catalog's, then your repositories' that the catalog does not list, by name.
    pub(in crate::native_view) fn results(
        self,
        search: &'a str,
        picked: Option<&Source>,
    ) -> Vec<&'a CatalogSkill> {
        match picked {
            Some(Source::Creator(id)) => self.curated(search).filter(|s| by(s, id)).collect(),
            Some(Source::Repository(id)) => self
                .repository(id)
                .map(|repository| self.listed(repository, search))
                .unwrap_or_default(),
            // skills.sh's results show alone.
            Some(Source::SkillsSh) => Vec::new(),
            None => {
                let mut listed: Vec<_> = self
                    .repositories
                    .iter()
                    .flat_map(|repository| self.listed(repository, search))
                    .filter(|skill| {
                        !self
                            .catalog
                            .skills
                            .iter()
                            .any(|curated| same_source(&curated.source, &skill.source))
                    })
                    .collect();
                listed.sort_by(|a, b| a.name.cmp(&b.name));
                self.curated(search).chain(listed).collect()
            }
        }
    }

    /// The source chips over the results, each with its count: with a search, every creator
    /// with a match, most matches first, then each repository with a match; without one, the
    /// most popular creators. A picked source always has a chip, first when it would have none.
    pub(in crate::native_view) fn chips(
        self,
        search: &'a str,
        picked: Option<&Source>,
    ) -> Vec<(Chip<'a>, usize)> {
        let count = |chip: Chip<'a>| self.results(search, Some(&chip.source())).len();
        let mut chips: Vec<_> = if search.is_empty() {
            popular_creators(self.catalog)
                .into_iter()
                .take(HOME_CHIPS)
                .map(|ranked| (Chip::Creator(ranked.creator), ranked.skills))
                .collect()
        } else {
            let mut creators: Vec<_> = self
                .catalog
                .creators
                .iter()
                .map(|creator| (Chip::Creator(creator), count(Chip::Creator(creator))))
                .filter(|(_, n)| *n > 0)
                .collect();
            creators.sort_by_key(|(_, n)| Reverse(*n));
            creators
                .into_iter()
                .chain(
                    self.repositories
                        .iter()
                        .map(|repository| {
                            let chip = Chip::Repository(repository);
                            (chip, count(chip))
                        })
                        .filter(|(_, n)| *n > 0),
                )
                .collect()
        };
        let picked = picked.and_then(|source| match source {
            Source::Creator(id) => self
                .catalog
                .creators
                .iter()
                .find(|creator| &creator.id == id)
                .map(Chip::Creator),
            Source::Repository(id) => self.repository(id).map(Chip::Repository),
            // skills.sh's chip is its own.
            Source::SkillsSh => None,
        });
        if let Some(chip) = picked
            && !chips.iter().any(|(shown, _)| *shown == chip)
        {
            chips.insert(0, (chip, count(chip)));
        }
        chips
    }
}
