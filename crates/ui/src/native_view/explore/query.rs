//! What Explore shows, derived from the snapshot alone: Trending, Popular creators, the results a
//! search and a picked source narrow it to, their source chips, "In library", and New.

mod repositories;
mod search;

pub(in crate::native_view) use repositories::{
    failures, installable, repository_glyph, summary, toast,
};
pub(in crate::native_view) use search::{Chip, Local, Source};

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

/// The creator who publishes `skill`; none for one of your repositories' skills.
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
