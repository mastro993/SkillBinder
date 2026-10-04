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
    if library.schema_version != 2
        || library.content_policy_version != 1
        || !valid_id(&library.library_id)
    {
        return Err(AppError::validation(
            "The library metadata uses an unsupported schema or identity.",
        ));
    }
    let mut folder_ids = BTreeSet::new();
    let mut folder_names = BTreeSet::new();
    for folder in &library.folders {
        if !valid_id(folder.id.as_str())
            || !folder_ids.insert(folder.id.clone())
            || !folder_names.insert(normalized(&validate_name(&folder.name)?))
        {
            return Err(AppError::validation(
                "Folder identities and names must be unique.",
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
            || skill
                .folder_id
                .as_ref()
                .is_some_and(|folder| !folder_ids.contains(folder))
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
impl State {
    pub(crate) fn read_library(&self) -> AppResult<Library> {
        let path = self.library_dir().join(".skillbinder.json");
        let metadata = fs::symlink_metadata(&path).map_err(|_| AppError::storage())?;
        if !metadata.is_file() || metadata.len() > 10 * 1024 * 1024 {
            return Err(AppError::validation(
                "Library metadata is not a bounded regular file.",
            ));
        }
        let library: Library =
            serde_json::from_slice(&fs::read(path).map_err(|_| AppError::storage())?)
                .map_err(|_| AppError::validation("Library metadata could not be read."))?;
        validate_library(&library)?;
        Ok(library)
    }
    pub(crate) fn library_snapshot(&self) -> AppResult<LibrarySnapshot> {
        let library = self.read_library()?;
        let revision = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&library).map_err(|_| AppError::storage())?)
        );
        let mut details = BTreeMap::new();
        for skill in &library.skills {
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
        })
    }
    pub(crate) fn preview_delete(&self, id: FolderId) -> AppResult<DeletePreview> {
        let snapshot = self.library_snapshot()?;
        let folder = snapshot
            .library
            .folders
            .iter()
            .find(|folder| folder.id == id)
            .cloned()
            .ok_or_else(|| AppError::not_found("Folder no longer exists."))?;
        Ok(DeletePreview {
            folder,
            affected_skills: snapshot
                .library
                .skills
                .iter()
                .filter(|skill| skill.folder_id.as_ref() == Some(&id))
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
        match change {
            OrganizationChange::CreateFolder { name } => after.folders.push(Folder {
                id: FolderId::new(),
                name: validate_name(&name)?,
            }),
            OrganizationChange::RenameFolder { id, name } => {
                let folder = after
                    .folders
                    .iter_mut()
                    .find(|folder| folder.id == id)
                    .ok_or_else(|| AppError::not_found("Folder no longer exists."))?;
                folder.name = validate_name(&name)?;
            }
            OrganizationChange::Assign { skills, folder } => {
                if folder
                    .as_ref()
                    .is_some_and(|id| !after.folders.iter().any(|folder| &folder.id == id))
                {
                    return Err(AppError::not_found("Folder no longer exists."));
                }
                if skills.is_empty()
                    || skills
                        .iter()
                        .any(|id| !after.skills.iter().any(|skill| &skill.id == id))
                {
                    return Err(AppError::not_found("Select existing skills to organize."));
                }
                for skill in &mut after.skills {
                    if skills.contains(&skill.id) {
                        skill.folder_id = folder.clone();
                    }
                }
            }
            OrganizationChange::DeleteFolder { id, revision } => {
                if revision != before.revision {
                    return Err(AppError::stale(
                        "The library changed after the deletion preview.",
                    ));
                }
                if !after.folders.iter().any(|folder| folder.id == id) {
                    return Err(AppError::not_found("Folder no longer exists."));
                }
                after.folders.retain(|folder| folder.id != id);
                for skill in &mut after.skills {
                    if skill.folder_id.as_ref() == Some(&id) {
                        skill.folder_id = None;
                    }
                }
            }
        }
        validate_library(&after)?;
        after.folders.sort_by_key(|folder| normalized(&folder.name));
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
