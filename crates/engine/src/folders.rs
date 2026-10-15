//! Library folders: browse-only groupings that hold each skill at most once.

use crate::library::{normalized, valid_id, validate_name};
use skillbinder_proto::{AppError, AppResult, Folder, FolderId, Library, SkillId};
use std::collections::BTreeSet;

/// The folder identities, once every folder has a valid identity and a unique name.
pub(crate) fn validate_folders(library: &Library) -> AppResult<BTreeSet<&FolderId>> {
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for folder in &library.folders {
        if !valid_id(folder.id.as_str())
            || !ids.insert(&folder.id)
            || !names.insert(normalized(&validate_name(&folder.name)?))
        {
            return Err(AppError::validation(
                "Folder identities and names must be unique.",
            ));
        }
    }
    Ok(ids)
}

pub(crate) fn create_folder(library: &mut Library, name: &str) -> AppResult<()> {
    library.folders.push(Folder {
        id: FolderId::new(),
        name: validate_name(name)?,
        position: u32::try_from(library.folders.len()).unwrap_or(u32::MAX),
    });
    Ok(())
}

pub(crate) fn rename_folder(library: &mut Library, id: &FolderId, name: &str) -> AppResult<()> {
    folder_mut(library, id)?.name = validate_name(name)?;
    Ok(())
}

/// Deletes the folder if the library is still at the `seen` revision; its skills become unfiled.
pub(crate) fn delete_folder(
    library: &mut Library,
    id: &FolderId,
    seen: &str,
    revision: &str,
) -> AppResult<()> {
    if seen != revision {
        return Err(AppError::stale(
            "The library changed before the folder was deleted.",
        ));
    }
    folder_mut(library, id)?;
    library.folders.retain(|folder| &folder.id != id);
    for skill in &mut library.skills {
        if skill.folder_id.as_ref() == Some(id) {
            skill.folder_id = None;
        }
    }
    Ok(())
}

/// Files `skill` in `folder`, out of any other, or leaves it unfiled.
pub(crate) fn move_to_folder(
    library: &mut Library,
    skill: &SkillId,
    folder: Option<FolderId>,
) -> AppResult<()> {
    if let Some(id) = &folder {
        folder_mut(library, id)?;
    }
    library
        .skills
        .iter_mut()
        .find(|record| &record.id == skill)
        .ok_or_else(|| AppError::not_found("Skill no longer exists."))?
        .folder_id = folder;
    Ok(())
}

/// Orders folders by position, falling back to name for duplicates from separate edits, and
/// renumbers them densely.
pub(crate) fn order_folders(folders: &mut [Folder]) {
    folders.sort_by_cached_key(|folder| {
        (folder.position, normalized(&folder.name), folder.id.clone())
    });
    for (position, folder) in folders.iter_mut().enumerate() {
        folder.position = u32::try_from(position).unwrap_or(u32::MAX);
    }
}

fn folder_mut<'a>(library: &'a mut Library, id: &FolderId) -> AppResult<&'a mut Folder> {
    library
        .folders
        .iter_mut()
        .find(|folder| &folder.id == id)
        .ok_or_else(|| AppError::not_found("Folder no longer exists."))
}
