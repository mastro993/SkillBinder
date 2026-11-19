//! What Explore shows, derived from the snapshot alone: Trending, Popular creators, the results a
//! search and a picked creator narrow the catalog to, their creator chips, "In library", and New,
//! and skills.sh's results beside them.

mod skills_sh;

pub(super) use skills_sh::{Found, Remote, remote, result_collides, result_in_library};

use std::cmp::Reverse;

use skillbinder_proto::{
    AppSnapshot, Catalog, CatalogOrigin, CatalogSkill, Creator, InstallFailure, InstallState,
    PreviewState, Skill, SkillId, SkillSource, install_key, normalized_url, owner_repo,
    same_source,
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

/// The creator who publishes `skill`; `None` for a skills.sh result.
pub(super) fn creator_of<'a>(catalog: &'a Catalog, skill: &CatalogSkill) -> Option<&'a Creator> {
    let CatalogOrigin::Curated { creator } = &skill.origin else {
        return None;
    };
    catalog.creators.iter().find(|known| &known.id == creator)
}

/// Whether `skill` is published by the creator `id`.
fn by(skill: &CatalogSkill, id: &str) -> bool {
    matches!(&skill.origin, CatalogOrigin::Curated { creator } if creator == id)
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
    held_from(snapshot, &skill.key, |source| {
        same_source(source, &skill.source)
    })
}

/// The library skill holding the slug `key` names, when `same` accepts its source.
fn held_from<'a>(
    snapshot: &'a AppSnapshot,
    key: &str,
    same: impl Fn(&SkillSource) -> bool,
) -> Option<&'a Skill> {
    let library = snapshot.library.as_ref()?;
    library
        .keys
        .get(key)
        .and_then(|id| library.library.skills.iter().find(|held| &held.id == id))
        .filter(|held| held.source.as_ref().is_some_and(&same))
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

/// Where the preview of `skill`'s open dialog stands; `None` until the projection carries it.
pub(super) fn preview_state<'a>(
    snapshot: &'a AppSnapshot,
    skill: &CatalogSkill,
) -> Option<&'a PreviewState> {
    let preview = snapshot.explore.preview.as_ref()?;
    (install_key(&preview.source) == element_key(skill)).then_some(&preview.state)
}

/// The repository's web page, `https://<host>/<owner>/<repo>`, with SSH and scp-style URLs
/// mapped to HTTPS; on github.com, the folder's page. `None` for a URL with no web page, such as
/// a local repository's.
pub(super) fn browser_url(source: &SkillSource) -> Option<String> {
    let normalized = normalized_url(&source.url);
    let (scheme, rest) = normalized.split_once("://")?;
    let (authority, repository) = rest.split_once('/')?;
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let host = match scheme {
        "https" | "http" => host,
        // An SSH port is not the web server's.
        "ssh" => host.split_once(':').map_or(host, |(host, _)| host),
        _ => return None,
    };
    if host.is_empty() || repository.is_empty() {
        return None;
    }
    let page = format!("https://{host}/{repository}");
    let folder = source.path.trim_matches('/');
    Some(if host == "github.com" && !folder.is_empty() {
        format!("{page}/tree/HEAD/{folder}")
    } else {
        page
    })
}

/// A front matter key as the Details card labels it: `allowed-tools` reads "Allowed tools".
pub(super) fn front_matter_label(key: &str) -> String {
    let words = key.replace(['-', '_'], " ");
    let mut chars = words.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// `skill_md` without its leading YAML front matter, which the dialog shows in its Details card
/// rather than as text.
pub(super) fn without_front_matter(skill_md: &str) -> &str {
    let Some(rest) = skill_md
        .strip_prefix("---\n")
        .or_else(|| skill_md.strip_prefix("---\r\n"))
    else {
        return skill_md;
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        offset += line.len();
        if line.trim_end() == "---" {
            return &rest[offset..];
        }
    }
    skill_md
}

/// The error line of an install of `slug` from the repository `url` that failed with `failure`.
pub(super) fn install_error(slug: &str, url: &str, failure: &InstallFailure) -> String {
    let repository = owner_repo(url);
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

/// `skill`'s installs as a card shows them: this week's gain as `+900 this week`,
/// `+12.4k this week`, or `+1.2M this week`; a skills.sh result's all-time installs as
/// `233.1k installs`.
pub(super) fn installs_label(skill: &CatalogSkill) -> String {
    match skill.origin {
        CatalogOrigin::Curated { .. } => format!("+{} this week", count(skill.installs_week)),
        CatalogOrigin::SkillsSh { installs } => all_time_label(installs),
    }
}

/// skills.sh's all-time installs, as `233.1k installs`.
pub(super) fn all_time_label(installs: u64) -> String {
    format!("{} installs", count(installs))
}

/// `installs` in at most four characters and a unit: `900`, `12.4k`, `1.2M`.
fn count(installs: u64) -> String {
    #[expect(clippy::cast_precision_loss, reason = "One decimal is shown.")]
    let value = installs as f64;
    let (scaled, unit) = match installs {
        0..1_000 => return installs.to_string(),
        1_000..999_950 => (value / 1_000., "k"),
        _ => (value / 1_000_000., "M"),
    };
    let text = format!("{scaled:.1}");
    let text = text.strip_suffix(".0").unwrap_or(&text);
    format!("{text}{unit}")
}

#[cfg(test)]
mod tests;
