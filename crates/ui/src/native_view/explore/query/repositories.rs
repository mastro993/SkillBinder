//! What Your repositories shows, derived from the snapshot alone: each repository's glyph, the
//! skills Install all installs, and how an Install all ended (R3, R5).

use super::install_error;
use skillbinder_proto::{AppSnapshot, InstallAllOutcome, Repository, normalized_url};

/// A repository's glyph: `lock` for an SSH or SCP-style URL, `github` otherwise. The app cannot
/// tell which repositories are private.
pub(in crate::native_view) fn repository_glyph(url: &str) -> &'static str {
    if normalized_url(url).starts_with("ssh://") {
        "lock"
    } else {
        "github"
    }
}

/// How many of `repository`'s skills Install all installs: those whose slug the library does
/// not hold.
pub(in crate::native_view) fn installable(
    snapshot: &AppSnapshot,
    repository: &Repository,
) -> usize {
    let held = |key: &str| {
        snapshot
            .library
            .as_ref()
            .is_some_and(|library| library.keys.contains_key(key))
    };
    repository
        .skills
        .iter()
        .filter(|skill| !held(&skill.key))
        .count()
}

/// The row's summary of an ended Install all: "Installed 14", or "Installed 13 · 1 failed".
pub(in crate::native_view) fn summary(outcome: &InstallAllOutcome) -> String {
    match outcome.failed.len() {
        0 => format!("Installed {}", outcome.installed.len()),
        failed => format!("Installed {} · {failed} failed", outcome.installed.len()),
    }
}

/// The toast of an ended Install all: "Installed 14 skills", or "Installed 13 · 1 failed".
pub(in crate::native_view) fn toast(outcome: &InstallAllOutcome) -> String {
    match (outcome.installed.len(), outcome.failed.len()) {
        (1, 0) => "Installed 1 skill".to_owned(),
        (installed, 0) => format!("Installed {installed} skills"),
        _ => summary(outcome),
    }
}

/// The summary's tooltip: each failed skill's error line, one per line.
pub(in crate::native_view) fn failures(
    repository: &Repository,
    outcome: &InstallAllOutcome,
) -> String {
    outcome
        .failed
        .iter()
        .filter_map(|(slug, failure)| {
            let skill = repository.skills.iter().find(|skill| &skill.slug == slug)?;
            Some(install_error(skill, failure))
        })
        .collect::<Vec<_>>()
        .join("\n")
}
