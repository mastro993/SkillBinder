use crate::{
    database,
    folders::{
        create_folder, delete_folder, move_to_folder, order_folders, rename_folder,
        validate_folders,
    },
    library_store,
    worker::State,
};
use sha2::{Digest, Sha256};
use skillbinder_proto::*;
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;

pub(crate) fn normalized(value: &str) -> String {
    value.nfkc().case_fold().collect()
}
pub(crate) fn validate_name(value: &str) -> AppResult<String> {
    let name = value.trim();
    if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
        return Err(AppError::validation(
            "Use a name with 1–100 characters and no control characters.",
        ));
    }
    Ok(name.to_owned())
}
/// A hash of the library, which previewed confirmations check.
fn revision(library: &Library) -> AppResult<String> {
    let bytes = serde_json::to_vec(library).map_err(|_| AppError::storage())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
/// Refuses duplicate preset or folder names after normalization.
fn validate_organization(library: &Library) -> AppResult<()> {
    let mut names = BTreeSet::new();
    for preset in &library.presets {
        if !names.insert(normalized(&validate_name(&preset.name)?)) {
            return Err(AppError::validation("Preset names must be unique."));
        }
    }
    validate_folders(library)
}
/// The newest modification time, in Unix seconds, of the regular files under `dir`; links are
/// skipped because managed payloads materialize them.
fn newest_modification(dir: &Path) -> Option<u64> {
    let mut newest = None;
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let Ok(metadata) = entry.path().symlink_metadata() else {
            continue;
        };
        let time = if metadata.is_dir() {
            newest_modification(&entry.path())
        } else if metadata.is_file() {
            metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|age| age.as_secs())
        } else {
            None
        };
        newest = newest.max(time);
    }
    newest
}
/// Orders presets by user position and renumbers them densely.
fn sort_presets(presets: &mut [Preset]) {
    presets.sort_by_key(|preset| preset.position);
    for (position, preset) in presets.iter_mut().enumerate() {
        preset.position = u32::try_from(position).unwrap_or(u32::MAX);
    }
}
fn apply_organization(
    library: &mut Library,
    change: OrganizationChange,
    revision: &str,
) -> AppResult<()> {
    match change {
        OrganizationChange::CreatePreset { name } => {
            let position = library
                .presets
                .iter()
                .map(|preset| preset.position.saturating_add(1))
                .max()
                .unwrap_or(0);
            library.presets.push(Preset {
                id: PresetId::new(),
                name: validate_name(&name)?,
                position,
                color: None,
                icon: PresetIcon::default(),
            });
        }
        OrganizationChange::RenamePreset { id, name } => {
            preset_mut(library, &id)?.name = validate_name(&name)?;
        }
        OrganizationChange::AddToPreset { id, skills } => {
            for presets in memberships(library, &id, &skills)? {
                if !presets.contains(&id) {
                    presets.push(id.clone());
                    presets.sort();
                }
            }
        }
        OrganizationChange::RemoveFromPreset { id, skills } => {
            for presets in memberships(library, &id, &skills)? {
                presets.retain(|preset| preset != &id);
            }
        }
        OrganizationChange::ReorderPresets { order } => reorder(
            &mut library.presets,
            &order,
            |preset| (&preset.id, &mut preset.position),
            "The presets changed. Reorder them again.",
        )?,
        OrganizationChange::SetPresetColor { id, color } => {
            preset_mut(library, &id)?.color = color;
        }
        OrganizationChange::SetPresetIcon { id, icon } => {
            preset_mut(library, &id)?.icon = icon;
        }
        OrganizationChange::CreateFolder { name } => create_folder(library, &name)?,
        OrganizationChange::RenameFolder { id, name } => rename_folder(library, &id, &name)?,
        OrganizationChange::DeleteFolder { id, revision: seen } => {
            delete_folder(library, &id, &seen, revision)?;
        }
        OrganizationChange::ReorderFolders { order } => reorder(
            &mut library.folders,
            &order,
            |folder| (&folder.id, &mut folder.position),
            "The folders changed. Reorder them again.",
        )?,
        OrganizationChange::MoveToFolder { skill, folder } => {
            move_to_folder(library, &skill, folder)?;
        }
        OrganizationChange::DeletePreset { id, revision: seen } => {
            if seen != revision {
                return Err(AppError::stale(
                    "The library changed after the deletion preview.",
                ));
            }
            preset_mut(library, &id)?;
            library.presets.retain(|preset| preset.id != id);
            for skill in &mut library.skills {
                skill.preset_ids.retain(|preset| preset != &id);
            }
        }
    }
    Ok(())
}
fn preset_mut<'a>(library: &'a mut Library, id: &PresetId) -> AppResult<&'a mut Preset> {
    library
        .presets
        .iter_mut()
        .find(|preset| &preset.id == id)
        .ok_or_else(|| AppError::not_found("Preset no longer exists."))
}
/// The preset lists of the selected skills, once the preset and every selected skill exist.
fn memberships<'a>(
    library: &'a mut Library,
    id: &PresetId,
    skills: &'a [SkillId],
) -> AppResult<impl Iterator<Item = &'a mut Vec<PresetId>>> {
    preset_mut(library, id)?;
    if skills.is_empty()
        || skills
            .iter()
            .any(|id| !library.skills.iter().any(|skill| &skill.id == id))
    {
        return Err(AppError::not_found("Select existing skills to organize."));
    }
    Ok(library
        .skills
        .iter_mut()
        .filter(|skill| skills.contains(&skill.id))
        .map(|skill| &mut skill.preset_ids))
}
/// Renumbers `items` to follow `order`, which must name each of them exactly once.
fn reorder<T, Id: Ord>(
    items: &mut [T],
    order: &[Id],
    slot: impl Fn(&mut T) -> (&Id, &mut u32),
    stale: &'static str,
) -> AppResult<()> {
    let mut requested: Vec<_> = order.iter().collect();
    requested.sort();
    requested.dedup();
    if requested.len() != order.len() || requested.len() != items.len() {
        return Err(AppError::stale(stale));
    }
    for item in items.iter_mut() {
        let (id, position) = slot(item);
        *position = order
            .iter()
            .position(|candidate| candidate == id)
            .and_then(|index| u32::try_from(index).ok())
            .ok_or_else(|| AppError::stale(stale))?;
    }
    Ok(())
}
impl State {
    pub(crate) fn read_library(&self) -> AppResult<Library> {
        Ok(library_store::load(&mut self.database.borrow_mut())?.0)
    }
    /// The library and its revision, without reading payload directories.
    pub(crate) fn revised_library(&self) -> AppResult<(Library, String)> {
        let library = self.read_library()?;
        let revision = revision(&library)?;
        Ok((library, revision))
    }
    pub(crate) fn library_snapshot(&self) -> AppResult<LibrarySnapshot> {
        let (library, details) = library_store::load(&mut self.database.borrow_mut())?;
        let revision = revision(&library)?;
        let modified = library
            .skills
            .iter()
            .filter_map(|skill| {
                let time = newest_modification(&self.skill_dir(&skill.id).ok()?)?;
                Some((skill.id.clone(), time))
            })
            .collect();
        Ok(LibrarySnapshot {
            library,
            revision,
            details,
            modified,
        })
    }
    pub(crate) fn preview_delete(&self, id: PresetId) -> AppResult<DeletePreview> {
        let (library, revision) = self.revised_library()?;
        let preset = library
            .presets
            .iter()
            .find(|preset| preset.id == id)
            .cloned()
            .ok_or_else(|| AppError::not_found("Preset no longer exists."))?;
        Ok(DeletePreview {
            preset,
            affected_skills: library
                .skills
                .iter()
                .filter(|skill| skill.preset_ids.contains(&id))
                .count(),
            revision,
        })
    }
    pub(crate) fn change_organization(
        &mut self,
        change: OrganizationChange,
    ) -> AppResult<LibrarySnapshot> {
        self.require_recovered()?;
        let (mut after, revision) = self.revised_library()?;
        apply_organization(&mut after, change, &revision)?;
        validate_organization(&after)?;
        sort_presets(&mut after.presets);
        order_folders(&mut after.folders);
        database::transaction(&mut self.database.borrow_mut(), |connection| {
            library_store::save_organization(connection, &after)
        })?;
        self.library_snapshot()
    }
    pub(crate) fn contained_skill_path(&self, path: &Path) -> AppResult<PathBuf> {
        let canonical = fs::canonicalize(path).map_err(|_| AppError::storage())?;
        let root =
            fs::canonicalize(self.library_dir().join("skills")).map_err(|_| AppError::storage())?;
        if !canonical.starts_with(&root) || canonical == root {
            return Err(AppError::validation(
                "Skill payload leaves the managed library.",
            ));
        }
        Ok(canonical)
    }
    /// The contained payload directory of a library skill.
    pub(crate) fn skill_dir(&self, id: &SkillId) -> AppResult<PathBuf> {
        self.contained_skill_path(&self.library_dir().join("skills").join(id.as_str()))
    }
    pub(crate) fn preview_skill(
        &self,
        id: SkillId,
        relative: Option<String>,
    ) -> AppResult<SkillPreview> {
        if !library_store::holds(&mut self.database.borrow_mut(), id.as_str())? {
            return Err(AppError::not_found("Skill no longer exists."));
        }
        let root = self.skill_dir(&id)?;
        let mut entries = Vec::new();
        let mut queue = vec![(root.clone(), 0)];
        while let Some((directory, depth)) = queue.pop() {
            if depth > 32 {
                return Err(AppError::validation(
                    "Preview exceeds the directory-depth limit.",
                ));
            }
            for entry in fs::read_dir(&directory).map_err(|_| AppError::storage())? {
                let entry = entry.map_err(|_| AppError::storage())?;
                let metadata =
                    fs::symlink_metadata(entry.path()).map_err(|_| AppError::storage())?;
                let path = entry
                    .path()
                    .strip_prefix(&root)
                    .map_err(|_| AppError::storage())?
                    .to_string_lossy()
                    .replace('\\', "/");
                entries.push(PreviewEntry {
                    path,
                    directory: metadata.is_dir(),
                    bytes: metadata.len(),
                });
                if entries.len() > 5000 {
                    return Err(AppError::validation("Preview exceeds the entry limit."));
                }
                if metadata.is_dir() {
                    queue.push((entry.path(), depth + 1));
                }
            }
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        let requested = relative.unwrap_or_else(|| "SKILL.md".to_owned());
        if requested.is_empty()
            || requested.contains('\\')
            || requested.split('/').any(str::is_empty)
            || Path::new(&requested)
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(AppError::validation(
                "Choose a relative file inside the skill.",
            ));
        }
        let selected = fs::canonicalize(root.join(&requested))
            .map_err(|_| AppError::not_found("Preview file no longer exists."))?;
        if !selected.starts_with(&root) {
            return Err(AppError::validation(
                "Preview path leaves the skill directory.",
            ));
        }
        let metadata = fs::metadata(&selected).map_err(|_| AppError::storage())?;
        let (text, unavailable) = if !metadata.is_file() {
            (None, Some("This entry is not a regular file.".into()))
        } else if metadata.len() > 128 * 1024 {
            (
                None,
                Some("This file is larger than the 128 KiB preview limit.".into()),
            )
        } else {
            use std::io::Read;
            let mut bytes = Vec::new();
            fs::File::open(selected)
                .map_err(|_| AppError::storage())?
                .take(128 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| AppError::storage())?;
            if bytes.len() > 128 * 1024 {
                (
                    None,
                    Some("This file grew beyond the preview limit.".into()),
                )
            } else if bytes.contains(&0) {
                (None, Some("Binary files are not previewed.".into()))
            } else {
                match String::from_utf8(bytes) {
                    Ok(text) => (Some(text), None),
                    Err(_) => (None, Some("Binary files are not previewed.".into())),
                }
            }
        };
        Ok(SkillPreview {
            skill_id: id,
            entries,
            text,
            unavailable,
        })
    }
}
