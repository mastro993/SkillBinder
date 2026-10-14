use crate::{
    persistence::{Journal, Move},
    worker::State,
};
use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};
use skillbinder_proto::*;
use std::{
    collections::{BTreeMap, BTreeSet},
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
fn portable_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.ends_with([' ', '.'])
        && !value
            .chars()
            .any(|c| c.is_control() || "/\\<>:\"|?*".contains(c))
        && !matches!(
            value
                .split('.')
                .next()
                .unwrap_or("")
                .to_ascii_uppercase()
                .as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        )
}
fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
pub(crate) fn validate_library(library: &Library) -> AppResult<()> {
    if library.schema_version != 3
        || library.content_policy_version != 1
        || !valid_id(&library.library_id)
    {
        return Err(AppError::validation(
            "The library metadata uses an unsupported schema or identity.",
        ));
    }
    let mut preset_ids = BTreeSet::new();
    let mut preset_names = BTreeSet::new();
    for preset in &library.presets {
        if !valid_id(preset.id.as_str())
            || !preset_ids.insert(preset.id.clone())
            || !preset_names.insert(normalized(&validate_name(&preset.name)?))
        {
            return Err(AppError::validation(
                "Preset identities and names must be unique.",
            ));
        }
    }
    let mut tag_ids = BTreeSet::new();
    let mut tag_names = BTreeSet::new();
    for tag in &library.tags {
        if !valid_id(&tag.id)
            || !tag_ids.insert(tag.id.clone())
            || !tag_names.insert(normalized(&validate_name(&tag.name)?))
        {
            return Err(AppError::validation(
                "Tag identities and names must be unique.",
            ));
        }
    }
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for skill in &library.skills {
        if !valid_id(skill.id.as_str())
            || !ids.insert(&skill.id)
            || !portable_component(&skill.slug)
            || !known_presets(&skill.preset_ids, &preset_ids)
            || skill.tag_ids.iter().any(|id| !tag_ids.contains(id))
            || skill.digest.strip_prefix("sha256:").is_none_or(|digest| {
                digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit())
            })
        {
            return Err(AppError::validation(
                "A skill record contains invalid names, assignments, or payload metadata.",
            ));
        }
        if !paths.insert(normalized(&payload_relative(library, skill))) {
            return Err(AppError::validation(
                "Skill directories collide after portable name normalization.",
            ));
        }
    }
    Ok(())
}
/// Whether `ids` names distinct existing presets.
fn known_presets(ids: &[PresetId], presets: &BTreeSet<PresetId>) -> bool {
    let mut seen = BTreeSet::new();
    ids.iter().all(|id| presets.contains(id) && seen.insert(id))
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
pub(crate) fn payload_relative(library: &Library, skill: &Skill) -> String {
    let first = library
        .skills
        .iter()
        .filter(|other| other.slug == skill.slug)
        .min_by_key(|other| &other.id);
    if first.is_none_or(|first| first.id == skill.id) {
        format!("skills/{}", skill.slug)
    } else {
        format!(
            "skills/{}-{}",
            skill.slug,
            skill.id.as_str().chars().take(8).collect::<String>()
        )
    }
}
/// Orders presets by user position; duplicate positions from remote edits fall back to name.
fn sort_presets(presets: &mut [Preset]) {
    presets.sort_by_cached_key(|preset| {
        (preset.position, normalized(&preset.name), preset.id.clone())
    });
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
        OrganizationChange::ReorderPresets { order } => reorder_presets(library, &order)?,
        OrganizationChange::SetPresetColor { id, color } => {
            preset_mut(library, &id)?.color = color;
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
fn reorder_presets(library: &mut Library, order: &[PresetId]) -> AppResult<()> {
    let mut requested: Vec<_> = order.iter().collect();
    requested.sort();
    let mut current: Vec<_> = library.presets.iter().map(|preset| &preset.id).collect();
    current.sort();
    if requested != current {
        return Err(AppError::stale("The presets changed. Reorder them again."));
    }
    for preset in &mut library.presets {
        preset.position = order
            .iter()
            .position(|id| id == &preset.id)
            .unwrap_or_default() as u32;
    }
    Ok(())
}
impl State {
    pub(crate) fn read_library(&self) -> AppResult<Library> {
        let path = self.library_dir().join(".skillbinder.json");
        let metadata = fs::symlink_metadata(&path).map_err(|_| AppError::storage())?;
        if !metadata.is_file() || metadata.len() > 10 * 1024 * 1024 {
            return Err(AppError::validation(
                "Library metadata is not a bounded regular file.",
            ));
        }
        let mut library: Library =
            serde_json::from_slice(&fs::read(path).map_err(|_| AppError::storage())?)
                .map_err(|_| AppError::validation("Library metadata could not be read."))?;
        validate_library(&library)?;
        sort_presets(&mut library.presets);
        Ok(library)
    }
    pub(crate) fn library_snapshot(&self) -> AppResult<LibrarySnapshot> {
        let library = self.read_library()?;
        let revision = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&library).map_err(|_| AppError::storage())?)
        );
        let mut details = BTreeMap::new();
        let mut modified = BTreeMap::new();
        for skill in &library.skills {
            let payload = self
                .contained_skill_path(&self.library_dir().join(payload_relative(&library, skill)));
            if let Some(time) = payload.ok().and_then(|dir| newest_modification(&dir)) {
                modified.insert(skill.id.clone(), time);
            }
            let serialized = self
                .database
                .query_row(
                    "SELECT validation FROM skill_metadata WHERE skill_id=?1",
                    [skill.id.as_str()],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|_| AppError::storage())?;
            if let Some(serialized) = serialized {
                details.insert(
                    skill.id.clone(),
                    serde_json::from_str(&serialized).map_err(|_| AppError::storage())?,
                );
            }
        }
        Ok(LibrarySnapshot {
            library,
            revision,
            details,
            modified,
        })
    }
    pub(crate) fn preview_delete(&self, id: PresetId) -> AppResult<DeletePreview> {
        let snapshot = self.library_snapshot()?;
        let preset = snapshot
            .library
            .presets
            .iter()
            .find(|preset| preset.id == id)
            .cloned()
            .ok_or_else(|| AppError::not_found("Preset no longer exists."))?;
        Ok(DeletePreview {
            preset,
            affected_skills: snapshot
                .library
                .skills
                .iter()
                .filter(|skill| skill.preset_ids.contains(&id))
                .count(),
            revision: snapshot.revision,
        })
    }
    pub(crate) fn change_organization(
        &mut self,
        change: OrganizationChange,
    ) -> AppResult<LibrarySnapshot> {
        self.require_recovered()?;
        let before = self.library_snapshot()?;
        let mut after = before.library.clone();
        apply_organization(&mut after, change, &before.revision)?;
        validate_library(&after)?;
        sort_presets(&mut after.presets);
        for (position, preset) in after.presets.iter_mut().enumerate() {
            preset.position = position as u32;
        }
        self.commit(Journal {
            id: uuid::Uuid::new_v4().to_string(),
            before: before.library,
            after,
            moves: Vec::new(),
            completed_moves: 0,
            progress: Default::default(),
            details: Vec::new(),
            outcome: None,
        })?;
        self.library_snapshot()
    }
    pub(crate) fn resolve_conflict(
        &mut self,
        slug: String,
        keep: SkillId,
        mut expected: Vec<SkillId>,
        revision: String,
    ) -> AppResult<LibrarySnapshot> {
        self.require_recovered()?;
        let before = self.library_snapshot()?;
        if before.revision != revision {
            return Err(AppError::stale(
                "The library changed after the resolution preview.",
            ));
        }
        let copies: Vec<_> = before
            .library
            .skills
            .iter()
            .filter(|skill| skill.slug == slug)
            .cloned()
            .collect();
        let mut ids: Vec<_> = copies.iter().map(|skill| skill.id.clone()).collect();
        ids.sort();
        expected.sort();
        if copies.len() < 2 || ids != expected || !ids.contains(&keep) {
            return Err(AppError::stale(
                "The shared copies changed. Review them again.",
            ));
        }
        let operation = uuid::Uuid::new_v4().to_string();
        let backup = self
            .config
            .data_dir
            .join("backups/resolutions")
            .join(&operation);
        let mut moves = Vec::new();
        for skill in copies.iter().filter(|skill| skill.id != keep) {
            let path = self
                .library_dir()
                .join(payload_relative(&before.library, skill));
            self.contained_skill_path(&path)?;
            moves.push(Move {
                from: path,
                to: backup.join(skill.id.as_str()),
            });
        }
        let winner = copies
            .iter()
            .find(|skill| skill.id == keep)
            .ok_or_else(|| AppError::not_found("Chosen skill no longer exists."))?;
        let from = self
            .library_dir()
            .join(payload_relative(&before.library, winner));
        self.contained_skill_path(&from)?;
        let to = self.library_dir().join("skills").join(&slug);
        if from != to {
            moves.push(Move { from, to });
        }
        let mut after = before.library.clone();
        after
            .skills
            .retain(|skill| skill.slug != slug || skill.id == keep);
        self.commit(Journal {
            id: operation,
            before: before.library,
            after,
            moves,
            completed_moves: 0,
            progress: Default::default(),
            details: Vec::new(),
            outcome: None,
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
    pub(crate) fn preview_skill(
        &self,
        id: SkillId,
        relative: Option<String>,
    ) -> AppResult<SkillPreview> {
        let library = self.read_library()?;
        let skill = library
            .skills
            .iter()
            .find(|skill| skill.id == id)
            .ok_or_else(|| AppError::not_found("Skill no longer exists."))?;
        let root =
            self.contained_skill_path(&self.library_dir().join(payload_relative(&library, skill)))?;
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
