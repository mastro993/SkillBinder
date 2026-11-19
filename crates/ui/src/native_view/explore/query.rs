//! What Explore shows, derived from the snapshot alone: Trending, Popular creators, the results a
//! search and a picked creator narrow the catalog to, their creator chips, "In library", and New.

use std::cmp::Reverse;

use skillbinder_proto::{
    AppSnapshot, Catalog, CatalogOrigin, CatalogSkill, Creator, InstallFailure, InstallState,
    Skill, SkillId, install_key, owner_repo, same_source,
};

/// How many cards Trending shows.
pub(super) const TRENDING: usize = 6;
/// How many creator chips show with a picked creator and no search.
const HOME_CHIPS: usize = 4;

/// A creator with their skills' count and installs this week.
#[derive(Debug, PartialEq)]
pub(super) struct Ranked<'a> {
    pub creator: &'a Creator,
    pub skills: usize,
    pub installs: u64,
}

/// The creator who publishes `skill`.
pub(super) fn creator_of<'a>(catalog: &'a Catalog, skill: &CatalogSkill) -> Option<&'a Creator> {
    let CatalogOrigin::Curated { creator } = &skill.origin;
    catalog.creators.iter().find(|known| &known.id == creator)
}

/// Whether `skill` is published by the creator `id`.
fn by(skill: &CatalogSkill, id: &str) -> bool {
    let CatalogOrigin::Curated { creator } = &skill.origin;
    creator == id
}

/// The skills with the most installs this week; the catalog keeps them in that order.
pub(super) fn trending(catalog: &Catalog) -> &[CatalogSkill] {
    &catalog.skills[..catalog.skills.len().min(TRENDING)]
}

/// Every creator with their skills' count and installs this week, most installs first.
pub(super) fn popular_creators(catalog: &Catalog) -> Vec<Ranked<'_>> {
    let mut ranked: Vec<_> = catalog
        .creators
        .iter()
        .map(|creator| {
            let skills = catalog.skills.iter().filter(|skill| by(skill, &creator.id));
            let (count, installs) =
                skills.fold((0, 0), |(n, sum), skill| (n + 1, sum + skill.installs_week));
            Ranked {
                creator,
                skills: count,
                installs,
            }
        })
        .collect();
    ranked.sort_by_key(|ranked| Reverse(ranked.installs));
    ranked
}

/// Whether `skill` matches `search`, already lowercased, by its name, description, creator, or
/// repository's `owner/repo`, so a search for its host or scheme matches nothing.
fn matches(catalog: &Catalog, skill: &CatalogSkill, search: &str) -> bool {
    let creator = creator_of(catalog, skill);
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

/// The skills matching `search`, already lowercased, most installs this week first.
fn found<'a>(catalog: &'a Catalog, search: &str) -> impl Iterator<Item = &'a CatalogSkill> {
    catalog
        .skills
        .iter()
        .filter(move |skill| matches(catalog, skill, search))
}

/// The results for `search`, already lowercased, narrowed to `creator` when one is picked.
pub(super) fn results<'a>(
    catalog: &'a Catalog,
    search: &str,
    creator: Option<&str>,
) -> Vec<&'a CatalogSkill> {
    found(catalog, search)
        .filter(|skill| creator.is_none_or(|id| by(skill, id)))
        .collect()
}

/// The creator chips over the results, each with its count: with a search, every creator with a
/// match, most matches first; without one, the most popular creators. A picked creator always
/// has a chip, first when it would have none.
pub(super) fn chips<'a>(
    catalog: &'a Catalog,
    search: &str,
    picked: Option<&str>,
) -> Vec<(&'a Creator, usize)> {
    let count = |creator: &Creator| {
        found(catalog, search)
            .filter(|skill| by(skill, &creator.id))
            .count()
    };
    let mut chips: Vec<_> = if search.is_empty() {
        popular_creators(catalog)
            .into_iter()
            .take(HOME_CHIPS)
            .map(|ranked| (ranked.creator, ranked.skills))
            .collect()
    } else {
        let mut chips: Vec<_> = catalog
            .creators
            .iter()
            .map(|creator| (creator, count(creator)))
            .filter(|(_, n)| *n > 0)
            .collect();
        chips.sort_by_key(|(_, n)| Reverse(*n));
        chips
    };
    if let Some(id) = picked
        && !chips.iter().any(|(creator, _)| creator.id == id)
        && let Some(creator) = catalog.creators.iter().find(|creator| creator.id == id)
    {
        chips.insert(0, (creator, count(creator)));
    }
    chips
}

/// What names `skill`'s elements and its install: its repository and folder, since a slug can
/// repeat across repositories.
pub(super) fn element_key(skill: &CatalogSkill) -> String {
    install_key(&skill.source)
}

/// Whether the library holds `skill`'s slug from the same source.
pub(super) fn in_library(snapshot: &AppSnapshot, skill: &CatalogSkill) -> bool {
    held(snapshot, skill).is_some()
}

/// The library skill holding `skill` from the same source, when it is New there.
pub(super) fn new_in_library<'a>(
    snapshot: &'a AppSnapshot,
    skill: &CatalogSkill,
) -> Option<&'a SkillId> {
    held(snapshot, skill)
        .filter(|held| held.new)
        .map(|held| &held.id)
}

/// The library skill holding `skill`'s slug from the same source.
fn held<'a>(snapshot: &'a AppSnapshot, skill: &CatalogSkill) -> Option<&'a Skill> {
    let library = snapshot.library.as_ref()?;
    library
        .keys
        .get(&skill.key)
        .and_then(|id| library.library.skills.iter().find(|held| &held.id == id))
        .filter(|held| {
            held.source
                .as_ref()
                .is_some_and(|source| same_source(source, &skill.source))
        })
}

/// Whether the library holds `skill`'s slug from another source or from none, so installing it
/// is refused.
pub(super) fn collides(snapshot: &AppSnapshot, skill: &CatalogSkill) -> bool {
    snapshot
        .library
        .as_ref()
        .is_some_and(|library| library.keys.contains_key(&skill.key))
        && !in_library(snapshot, skill)
}

/// Where `skill`'s install stands, if it is not idle.
pub(super) fn install_state<'a>(
    snapshot: &'a AppSnapshot,
    skill: &CatalogSkill,
) -> Option<&'a InstallState> {
    snapshot.explore.installs.get(&element_key(skill))
}

/// The error line of an install that failed with `failure`.
pub(super) fn install_error(skill: &CatalogSkill, failure: &InstallFailure) -> String {
    let (slug, repository) = (&skill.slug, owner_repo(&skill.source.url));
    match failure {
        InstallFailure::Access => format!("Couldn't install. Check your access to {repository}."),
        InstallFailure::Offline => "Couldn't install: no connection.".to_owned(),
        InstallFailure::NotFound => format!("Couldn't find this skill in {repository}"),
        InstallFailure::Blocked => format!("Couldn't install {slug}: it is blocked."),
        InstallFailure::Invalid => {
            format!("Couldn't install {slug}: its SKILL.md metadata is invalid.")
        }
        InstallFailure::Empty => format!("Couldn't install {slug}: its folder has no files."),
        InstallFailure::Other(message) => format!("Couldn't install {slug}. {message}"),
    }
}

/// Installs this week as a card shows them: `+900`, `+12.4k`, `+1.2M`.
pub(super) fn installs_label(installs: u64) -> String {
    #[expect(clippy::cast_precision_loss, reason = "One decimal is shown.")]
    let value = installs as f64;
    let (scaled, unit) = match installs {
        0..1_000 => return format!("+{installs} this week"),
        1_000..999_950 => (value / 1_000., "k"),
        _ => (value / 1_000_000., "M"),
    };
    let text = format!("{scaled:.1}");
    let text = text.strip_suffix(".0").unwrap_or(&text);
    format!("+{text}{unit} this week")
}

#[cfg(test)]
mod tests;
