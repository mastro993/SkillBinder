//! The library skills one place holds, which its page lists and the Add skill picker shows as
//! already there, and the library skills the picker lists for its folder chip and search, or a
//! page's search keeps.

use super::{matches, name_order};
use skillbinder_proto::{AppSnapshot, FolderId, PresetId, Skill, Workspace};

/// The library skills `workspace` holds, by name.
pub(in crate::native_view) fn workspace_skills<'a>(
    snapshot: &'a AppSnapshot,
    workspace: &Workspace,
) -> Vec<&'a Skill> {
    sorted(snapshot, |skill| workspace.skills.contains_key(&skill.id))
}

/// The library skills `preset` holds for any of `ids`, the detected agents reading one global
/// skill directory, by name.
pub(in crate::native_view) fn agent_skills<'a>(
    snapshot: &'a AppSnapshot,
    preset: &PresetId,
    ids: &[String],
) -> Vec<&'a Skill> {
    sorted(snapshot, |skill| {
        skill
            .presets
            .get(preset)
            .is_some_and(|targets| ids.iter().any(|id| targets.includes(id)))
    })
}

/// The library skills in `folder`, or all of them, that `search`, lowercase, matches, by name.
pub(in crate::native_view) fn searched<'a>(
    snapshot: &'a AppSnapshot,
    folder: Option<&FolderId>,
    search: &str,
) -> Vec<&'a Skill> {
    let Some(library) = &snapshot.library else {
        return Vec::new();
    };
    sorted(snapshot, |skill| {
        folder.is_none_or(|id| skill.folder_id.as_ref() == Some(id))
            && matches(skill, library, search)
    })
}

/// The skills of `skills` that `search`, lowercase, matches by slug or description, in order.
pub(in crate::native_view) fn matching<'a>(
    snapshot: &AppSnapshot,
    skills: Vec<&'a Skill>,
    search: &str,
) -> Vec<&'a Skill> {
    let Some(library) = &snapshot.library else {
        return Vec::new();
    };
    skills
        .into_iter()
        .filter(|skill| matches(skill, library, search))
        .collect()
}

/// The library skills `keep` keeps, by name.
fn sorted(snapshot: &AppSnapshot, keep: impl Fn(&Skill) -> bool) -> Vec<&Skill> {
    let mut skills: Vec<&Skill> = snapshot
        .library
        .iter()
        .flat_map(|library| &library.library.skills)
        .filter(|skill| keep(skill))
        .collect();
    skills.sort_by_cached_key(|skill| name_order(skill));
    skills
}

#[cfg(test)]
mod tests;
