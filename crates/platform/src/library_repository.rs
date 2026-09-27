use crate::{
    git::GitEnvironment,
    paths::AppPaths,
    payload_filesystem::FilesystemPayloadSource,
    portable_metadata::{PortableMetadata, PortableSkill},
};
use serde_json::json;
use skillbinder_core::{
    discovery::LibraryCatalog,
    import::{ImportError, ImportPlan, LibraryRecord, LibraryRepository, LibrarySkillPreview},
    library::{Manifest, PayloadModel, ValidationLimits, inspect_payload},
    library_maintenance::ConflictResolution,
};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};

const PREVIEW_CONTENT_LIMIT: usize = 128 * 1024;
const PREVIEW_MAX_DEPTH: usize = 32;

pub struct FilesystemLibraryRepository {
    pub paths: AppPaths,
    git: GitEnvironment,
}
impl FilesystemLibraryRepository {
    pub fn new(paths: AppPaths) -> Self {
        Self {
            paths,
            git: GitEnvironment::default(),
        }
    }
    fn root(&self) -> PathBuf {
        self.paths.library()
    }
    fn staging(&self, plan: &str, skill: &str) -> PathBuf {
        self.paths
            .data
            .join("staging/import")
            .join(plan)
            .join(skill)
    }
    fn map(error: impl ToString) -> ImportError {
        ImportError::Library(error.to_string())
    }

    pub(crate) fn metadata(&self) -> Result<PortableMetadata, ImportError> {
        PortableMetadata::load(&self.root().join(".skillbinder.json")).map_err(Self::map)
    }

    pub(crate) fn write_metadata(&self, metadata: &PortableMetadata) -> Result<(), ImportError> {
        metadata
            .write(&self.root().join(".skillbinder.json"))
            .map_err(Self::map)
    }

    fn payload_directory(&self, skill: &str, slug: &str) -> Result<String, ImportError> {
        let metadata = self.metadata()?;
        let directories = payload_directories(
            metadata
                .skills
                .values()
                .map(|record| (record.id.as_str(), record.slug.as_str()))
                .chain(std::iter::once((skill, slug))),
        );
        directories
            .get(skill)
            .cloned()
            .ok_or_else(|| Self::map("payload directory could not be resolved"))
    }

    fn rename_payload(&self, from: &str, to: &str) -> Result<(), ImportError> {
        if from == to {
            return Ok(());
        }
        let skills = self.root().join("skills");
        let source = skills.join(from);
        if fs::symlink_metadata(&source).is_err() {
            return Ok(());
        }
        let target = skills.join(to);
        if fs::symlink_metadata(&target).is_ok() {
            return Err(ImportError::Validation("destination already exists".into()));
        }
        fs::rename(source, target).map_err(Self::map)
    }
}

/// The payload directory of every skill, keyed by skill id. A skill owns the directory named after
/// its slug, unless several skills share one, in which case the lowest id keeps the bare slug and
/// the others carry an id suffix. The name an agent reads is the frontmatter name and the metadata
/// slug, so only this directory name can differ.
fn payload_records(metadata: &PortableMetadata) -> Vec<(&str, &str)> {
    metadata
        .skills
        .values()
        .map(|record| (record.id.as_str(), record.slug.as_str()))
        .collect()
}

pub fn payload_directories<'a>(
    skills: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> BTreeMap<String, String> {
    let skills: Vec<(&str, &str)> = skills.into_iter().collect();
    let mut owners: BTreeMap<&str, &str> = BTreeMap::new();
    for &(id, slug) in &skills {
        owners
            .entry(slug)
            .and_modify(|owner| {
                if id < *owner {
                    *owner = id;
                }
            })
            .or_insert(id);
    }
    skills
        .into_iter()
        .map(|(id, slug)| {
            let suffix = match owners.get(slug) {
                Some(owner) if *owner == id => String::new(),
                _ => format!("-{}", id.chars().take(8).collect::<String>()),
            };
            (id.to_owned(), format!("{slug}{suffix}"))
        })
        .collect()
}

fn preview_path(path: &str) -> Result<&Path, ImportError> {
    let candidate = Path::new(path);
    if path.is_empty()
        || path.contains('\\')
        || path.split('/').any(|part| part.is_empty())
        || !candidate
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(ImportError::Validation("invalid skill file path".into()));
    }
    Ok(candidate)
}

fn preview_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<String>,
    last_edited_at: &mut Option<u32>,
    remaining_entries: &mut usize,
    depth: usize,
) -> Result<(), ImportError> {
    for entry in fs::read_dir(directory).map_err(FilesystemLibraryRepository::map)? {
        if *remaining_entries == 0 {
            return Err(ImportError::Validation(format!(
                "skill exceeds preview limit of {} entries",
                ValidationLimits::default().entries
            )));
        }
        *remaining_entries -= 1;
        let entry = entry.map_err(FilesystemLibraryRepository::map)?;
        let file_type = entry
            .file_type()
            .map_err(FilesystemLibraryRepository::map)?;
        if file_type.is_dir() {
            if depth >= PREVIEW_MAX_DEPTH {
                return Err(ImportError::Validation(format!(
                    "skill exceeds preview depth limit of {PREVIEW_MAX_DEPTH}"
                )));
            }
            preview_files(
                root,
                &entry.path(),
                files,
                last_edited_at,
                remaining_entries,
                depth + 1,
            )?;
        } else if file_type.is_file() {
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(FilesystemLibraryRepository::map)?
                .to_str()
                .ok_or_else(|| ImportError::Library("skill has a non-UTF-8 filename".into()))?
                .replace(std::path::MAIN_SEPARATOR, "/");
            files.push(relative);
            let modified = entry
                .metadata()
                .map_err(FilesystemLibraryRepository::map)?
                .modified()
                .map_err(FilesystemLibraryRepository::map)?;
            if let Ok(seconds) = modified.duration_since(UNIX_EPOCH)
                && let Ok(seconds) = u32::try_from(seconds.as_secs())
            {
                *last_edited_at = Some(last_edited_at.unwrap_or_default().max(seconds));
            }
        }
    }
    Ok(())
}

impl LibraryRepository for FilesystemLibraryRepository {
    fn catalog(&self) -> Result<Vec<LibraryRecord>, ImportError> {
        let metadata = self.metadata()?;
        let directories = payload_directories(payload_records(&metadata));
        Ok(metadata
            .skills
            .into_values()
            .map(|skill| LibraryRecord {
                payload_directory: directories.get(&skill.id).cloned().unwrap_or_default(),
                skill_id: skill.id,
                slug: skill.slug,
                digest: skill.digest,
                file_count: skill.file_count,
                total_bytes: skill.total_bytes,
            })
            .collect())
    }
    fn organization_snapshot(
        &self,
    ) -> Result<skillbinder_core::library::organization::Organization, ImportError> {
        self.read_organization()
    }
    fn change_organization(
        &self,
        change: skillbinder_core::library::organization::Change,
        expected_revision: Option<&str>,
    ) -> Result<skillbinder_core::library::organization::Organization, ImportError> {
        self.write_organization(change, expected_revision)
    }
    fn catalog_with_organization(
        &self,
    ) -> Result<
        (
            Vec<LibraryRecord>,
            skillbinder_core::library::organization::Organization,
        ),
        ImportError,
    > {
        let _guard = crate::organization::lock()?;
        Ok((self.catalog()?, self.read_organization()?))
    }
    fn skill_preview(
        &self,
        skill_id: &str,
        path: Option<&str>,
    ) -> Result<LibrarySkillPreview, ImportError> {
        let selected = path.unwrap_or("SKILL.md");
        let relative = preview_path(selected)?;
        let metadata = self.metadata()?;
        if metadata
            .skills
            .get(skill_id)
            .is_none_or(|skill| skill.id != skill_id)
        {
            return Err(ImportError::Validation(
                "skill is not in the library".into(),
            ));
        }
        let directory = payload_directories(payload_records(&metadata))
            .remove(skill_id)
            .ok_or_else(|| Self::map("payload directory could not be resolved"))?;
        let directory_path = preview_path(&directory)?;
        if directory_path.components().count() != 1 {
            return Err(ImportError::Validation("invalid skill directory".into()));
        }
        let library_root = fs::canonicalize(self.root()).map_err(Self::map)?;
        let skills_path = self.root().join("skills");
        if !fs::symlink_metadata(&skills_path)
            .map_err(Self::map)?
            .file_type()
            .is_dir()
        {
            return Err(ImportError::Validation(
                "skills directory is not a directory".into(),
            ));
        }
        let skills_root = fs::canonicalize(skills_path).map_err(Self::map)?;
        if !skills_root.starts_with(&library_root) {
            return Err(ImportError::Validation(
                "skill directory leaves the library".into(),
            ));
        }
        let skill_path = skills_root.join(directory_path);
        if !fs::symlink_metadata(&skill_path)
            .map_err(Self::map)?
            .file_type()
            .is_dir()
        {
            return Err(ImportError::Validation(
                "skill directory is not a directory".into(),
            ));
        }
        let skill_root = fs::canonicalize(&skill_path).map_err(Self::map)?;
        if !skill_root.starts_with(&skills_root) {
            return Err(ImportError::Validation(
                "skill directory leaves the library".into(),
            ));
        }
        let mut files = Vec::new();
        let mut last_edited_at = None;
        let mut remaining_entries = ValidationLimits::default().entries;
        preview_files(
            &skill_root,
            &skill_root,
            &mut files,
            &mut last_edited_at,
            &mut remaining_entries,
            0,
        )?;
        files.sort();
        let (content, unavailable_reason) = if files.iter().any(|file| file == selected) {
            let resolved = fs::canonicalize(skill_root.join(relative)).map_err(Self::map)?;
            if !resolved.starts_with(&skill_root) {
                return Err(ImportError::Validation(
                    "skill file leaves its directory".into(),
                ));
            }
            let file = fs::File::open(resolved).map_err(Self::map)?;
            if file.metadata().map_err(Self::map)?.len() > PREVIEW_CONTENT_LIMIT as u64 {
                (
                    None,
                    Some("File is too large to preview (128 KiB limit).".into()),
                )
            } else {
                let mut bytes = Vec::new();
                file.take(PREVIEW_CONTENT_LIMIT as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(Self::map)?;
                if bytes.len() > PREVIEW_CONTENT_LIMIT {
                    (
                        None,
                        Some("File is too large to preview (128 KiB limit).".into()),
                    )
                } else if bytes.contains(&0) {
                    (None, Some("Binary file cannot be previewed.".into()))
                } else {
                    match String::from_utf8(bytes) {
                        Ok(text) => (Some(text), None),
                        Err(_) => (None, Some("Binary file cannot be previewed.".into())),
                    }
                }
            }
        } else {
            (None, Some("File is not available to preview.".into()))
        };
        Ok(LibrarySkillPreview {
            skill_id: skill_id.to_owned(),
            last_edited_at,
            files,
            path: selected.to_owned(),
            content,
            unavailable_reason,
        })
    }
    fn stage_payload(
        &self,
        plan: &str,
        skill: &str,
        model: &PayloadModel,
    ) -> Result<(), ImportError> {
        let dir = self.staging(plan, skill);
        fs::create_dir_all(&dir).map_err(Self::map)?;
        for entry in &model.entries {
            let path = dir.join(entry.path.replace('/', std::path::MAIN_SEPARATOR_STR));
            match entry.kind {
                skillbinder_core::library::ManifestKind::Directory => {
                    fs::create_dir_all(path).map_err(Self::map)?;
                }
                skillbinder_core::library::ManifestKind::File => {
                    if let Some(bytes) = &entry.content {
                        if let Some(parent) = path.parent() {
                            fs::create_dir_all(parent).map_err(Self::map)?;
                        }
                        fs::write(&path, bytes).map_err(Self::map)?;
                        #[cfg(unix)]
                        if entry.executable {
                            use std::os::unix::fs::PermissionsExt;
                            let mut permissions =
                                fs::metadata(&path).map_err(Self::map)?.permissions();
                            permissions.set_mode(permissions.mode() | 0o111);
                            fs::set_permissions(&path, permissions).map_err(Self::map)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn verify_staged_manifest(
        &self,
        plan: &str,
        skill: &str,
        manifest: &Manifest,
    ) -> Result<(), ImportError> {
        let staged = self.staging(plan, skill);
        let model = inspect_payload(
            &staged,
            &FilesystemPayloadSource,
            &ValidationLimits::default(),
        )
        .map_err(|error| Self::map(error.to_string()))?;
        if !model.manifest.equivalent(manifest) {
            return Err(ImportError::SourceChanged);
        }
        Ok(())
    }
    fn move_staged_payload(&self, plan: &str, skill: &str, slug: &str) -> Result<(), ImportError> {
        let metadata = self.metadata()?;
        let records = payload_records(&metadata);
        let before = payload_directories(records.iter().copied());
        let after = payload_directories(
            records
                .iter()
                .copied()
                .chain(std::iter::once((skill, slug))),
        );
        for (id, _) in &records {
            if let (Some(from), Some(to)) = (before.get(*id), after.get(*id)) {
                self.rename_payload(from, to)?;
            }
        }
        let target = self.root().join("skills").join(
            after
                .get(skill)
                .ok_or_else(|| Self::map("payload directory could not be resolved"))?,
        );
        if fs::symlink_metadata(&target).is_ok() {
            return Err(ImportError::Validation("destination already exists".into()));
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(Self::map)?;
        }
        fs::rename(self.staging(plan, skill), target).map_err(Self::map)?;
        Ok(())
    }

    fn write_record_and_manifest(
        &self,
        skill: &str,
        slug: &str,
        model: &PayloadModel,
    ) -> Result<(), ImportError> {
        let _guard = crate::organization::lock()?;
        let mut metadata = self.metadata()?;
        metadata.skills.insert(
            skill.to_owned(),
            PortableSkill::from_manifest(skill.to_owned(), slug.to_owned(), &model.manifest),
        );
        self.write_metadata(&metadata)?;
        Ok(())
    }
    fn remove_record_and_manifest(&self, skill: &str) -> Result<(), ImportError> {
        let _guard = crate::organization::lock()?;
        let mut metadata = self.metadata()?;
        metadata.skills.remove(skill);
        self.write_metadata(&metadata)
    }
    fn rollback_import(&self, plan: &str, moved: &[(String, String)]) -> Result<(), ImportError> {
        for (skill, slug) in moved {
            let target = self
                .root()
                .join("skills")
                .join(self.payload_directory(skill, slug)?);
            if fs::symlink_metadata(&target).is_ok() {
                fs::remove_dir_all(&target).map_err(Self::map)?;
            }
        }
        let metadata = self.metadata()?;
        let all = payload_directories(payload_records(&metadata).iter().copied());
        let remaining: Vec<(&str, &str)> = metadata
            .skills
            .values()
            .filter(|record| !moved.iter().any(|(skill, _)| skill == &record.id))
            .map(|record| (record.id.as_str(), record.slug.as_str()))
            .collect();
        let restored = payload_directories(remaining.iter().copied());
        for (id, _) in &remaining {
            if let (Some(from), Some(to)) = (all.get(*id), restored.get(*id)) {
                self.rename_payload(from, to)?;
            }
        }
        self.delete_staged(plan)
    }
    fn delete_staged(&self, plan: &str) -> Result<(), ImportError> {
        let path = self.paths.data.join("staging/import").join(plan);
        if path.exists() {
            fs::remove_dir_all(path).map_err(Self::map)?;
        }
        Ok(())
    }
    fn current_revision(&self) -> Result<Option<String>, ImportError> {
        let git = self
            .git
            .verify()
            .map_err(|error| ImportError::Library(format!("git revision unavailable: {error}")))?;
        let output = self
            .git
            .run(
                &git,
                [
                    OsString::from("-C"),
                    self.root().into_os_string(),
                    OsString::from("rev-parse"),
                    OsString::from("HEAD"),
                ],
            )
            .map_err(|error| ImportError::Library(format!("git revision unavailable: {error}")))?;
        let revision = String::from_utf8(output.stdout)
            .map_err(|_| ImportError::Library("git revision output was not UTF-8".into()))?
            .trim()
            .to_owned();
        if revision.is_empty() {
            return Err(ImportError::Library("git revision output was empty".into()));
        }
        Ok(Some(revision))
    }
    fn pending_resolution(&self) -> Result<Option<ConflictResolution>, ImportError> {
        let journals = self.paths.data.join("journals");
        let entries = match fs::read_dir(&journals) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(Self::map(error)),
        };
        let mut pending: Vec<PathBuf> = entries
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<PathBuf>, _>>()
            .map_err(Self::map)?
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("resolve-") && name.ends_with(".json"))
            })
            .collect();
        pending.sort();
        match pending.first() {
            Some(path) => {
                let bytes = fs::read(path).map_err(Self::map)?;
                serde_json::from_slice::<ConflictResolution>(&bytes)
                    .map(Some)
                    .map_err(Self::map)
            }
            None => Ok(None),
        }
    }

    fn write_resolution_journal(&self, resolution: &ConflictResolution) -> Result<(), ImportError> {
        let journals = self.paths.data.join("journals");
        fs::create_dir_all(&journals).map_err(Self::map)?;
        let path = journals.join(format!("resolve-{}.json", resolution.operation_id));
        let temporary = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(resolution).map_err(Self::map)?;
        fs::write(&temporary, bytes).map_err(Self::map)?;
        fs::rename(temporary, path).map_err(Self::map)
    }

    fn remove_resolution_journal(&self, operation_id: &str) -> Result<(), ImportError> {
        let path = self
            .paths
            .data
            .join("journals")
            .join(format!("resolve-{operation_id}.json"));
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(Self::map(error)),
        }
    }

    fn apply_resolution(&self, resolution: &ConflictResolution) -> Result<String, ImportError> {
        let _guard = crate::organization::lock()?;
        let mut metadata = self.metadata()?;
        let before = payload_directories(payload_records(&metadata));
        for skill in &resolution.drop_skill_ids {
            metadata.skills.remove(skill);
        }
        let after = payload_directories(payload_records(&metadata));
        let backup = self
            .paths
            .data
            .join("backups/resolutions")
            .join(&resolution.operation_id);
        let skills = self.root().join("skills");
        for skill in &resolution.drop_skill_ids {
            let Some(name) = before.get(skill) else {
                continue;
            };
            let discarded = skills.join(name);
            if fs::symlink_metadata(&discarded).is_err() {
                continue;
            }
            fs::create_dir_all(&backup).map_err(Self::map)?;
            fs::rename(&discarded, backup.join(name)).map_err(Self::map)?;
        }
        let kept = after
            .get(&resolution.keep_skill_id)
            .cloned()
            .ok_or_else(|| Self::map("the kept skill is missing from the library"))?;
        if let Some(current) = before.get(&resolution.keep_skill_id) {
            self.rename_payload(current, &kept)?;
        }
        self.write_metadata(&metadata)?;
        debug_assert_eq!(kept, resolution.slug);
        Ok(kept)
    }

    fn has_uncommitted_changes(&self) -> Result<bool, ImportError> {
        let git = self
            .git
            .verify()
            .map_err(|error| ImportError::Library(format!("git status unavailable: {error}")))?;
        let output = self
            .git
            .run(
                &git,
                [
                    OsString::from("-C"),
                    self.root().into_os_string(),
                    OsString::from("status"),
                    OsString::from("--porcelain"),
                    OsString::from("--"),
                    OsString::from("skills"),
                    OsString::from(".skillbinder.json"),
                ],
            )
            .map_err(|error| ImportError::Library(format!("git status unavailable: {error}")))?;
        Ok(!output.stdout.is_empty())
    }
    fn write_import_journal(&self, plan: &ImportPlan) -> Result<(), ImportError> {
        let path = self
            .paths
            .data
            .join("journals")
            .join(format!("import-{}.json", plan.id));
        fs::create_dir_all(path.parent().unwrap()).map_err(Self::map)?;
        let mut items = Vec::with_capacity(plan.items.len());
        for item in &plan.items {
            items.push(json!({
                "skillId": item.skill_id,
                "slug": item.selection.slug,
                "targetPath": self
                    .root()
                    .join("skills")
                    .join(self.payload_directory(&item.skill_id, &item.selection.slug)?),
            }));
        }
        fs::write(
            path,
            serde_json::to_vec(&json!({
                "planId": plan.id,
                "libraryRevision": plan.library_revision,
                "items": items,
            }))
            .map_err(Self::map)?,
        )
        .map_err(Self::map)?;
        Ok(())
    }
    fn remove_import_journal(&self, plan_id: &str) -> Result<(), ImportError> {
        let path = self
            .paths
            .data
            .join("journals")
            .join(format!("import-{plan_id}.json"));
        if path.exists() {
            fs::remove_file(path).map_err(Self::map)?;
        }
        Ok(())
    }
    fn unresolved_import_journal(&self) -> Result<Option<String>, ImportError> {
        let dir = self.paths.data.join("journals");
        if !dir.exists() {
            return Ok(None);
        }
        let found = fs::read_dir(dir)
            .map_err(Self::map)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .any(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("import-") && name.ends_with(".json"))
            });
        Ok(found.then(|| "unresolved import journal".to_owned()))
    }
}
impl LibraryCatalog for FilesystemLibraryRepository {
    fn matching_payload(&self, digest: &str) -> Option<(String, String)> {
        self.catalog()
            .ok()?
            .into_iter()
            .find(|record| record.digest == digest)
            .map(|record| (record.skill_id, record.slug))
    }
    fn slug_owner(&self, slug: &str) -> Option<(String, String)> {
        self.catalog()
            .ok()?
            .into_iter()
            .find(|record| record.slug == slug)
            .map(|record| (record.skill_id, record.slug))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_core::library_maintenance::ConflictResolution;
    use std::path::Path;

    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "skillbinder-{name}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn library_in(root: &TempDir) -> (AppPaths, FilesystemLibraryRepository) {
        let paths = AppPaths::new(
            root.path().join("data"),
            root.path().join("config"),
            root.path().join("cache"),
        );
        paths.create_base_directories().expect("base directories");
        let library = FilesystemLibraryRepository::new(paths.clone());
        (paths, library)
    }

    fn write_payload(directory: &Path, content: &str) {
        fs::create_dir_all(directory).expect("payload directory");
        fs::write(directory.join("SKILL.md"), content).expect("payload file");
    }

    fn record(id: &str, slug: &str) -> PortableSkill {
        PortableSkill {
            id: id.to_owned(),
            slug: slug.to_owned(),
            display_name: None,
            folder_id: None,
            tag_ids: Vec::new(),
            upstream_bindings: Vec::new(),
            digest: format!("sha256:{id}"),
            file_count: 1,
            total_bytes: 1,
        }
    }

    fn write_metadata(library: &Path, records: &[PortableSkill]) {
        let mut metadata = PortableMetadata::new("library".into(), "created".into());
        for skill in records {
            metadata.skills.insert(skill.id.clone(), skill.clone());
        }
        metadata
            .write(&library.join(".skillbinder.json"))
            .expect("library metadata");
    }

    fn directory_names(skills: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(skills)
            .expect("skills directory")
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn preview_lists_files_reads_content_and_uses_latest_file_mtime() {
        let root = TempDir::new("skill-preview");
        let (paths, library) = library_in(&root);
        let skill = paths.library().join("skills/review");
        write_payload(&skill, "# Review\n");
        fs::create_dir_all(skill.join("assets")).unwrap();
        fs::write(skill.join("assets/guide.md"), "Guide\n").unwrap();
        write_metadata(&paths.library(), &[record("skill-1", "review")]);
        for (file, seconds) in [
            ("SKILL.md", 1_700_000_000),
            ("assets/guide.md", 1_800_000_000),
        ] {
            fs::File::open(skill.join(file))
                .unwrap()
                .set_times(
                    fs::FileTimes::new()
                        .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(seconds)),
                )
                .unwrap();
        }

        let preview = library.skill_preview("skill-1", None).unwrap();
        assert_eq!(preview.skill_id, "skill-1");
        assert_eq!(preview.path, "SKILL.md");
        assert_eq!(preview.content.as_deref(), Some("# Review\n"));
        assert_eq!(preview.last_edited_at, Some(1_800_000_000));
        assert_eq!(preview.files, ["SKILL.md", "assets/guide.md"]);
        assert_eq!(preview.unavailable_reason, None);

        let nested = library
            .skill_preview("skill-1", Some("assets/guide.md"))
            .unwrap();
        assert_eq!(nested.content.as_deref(), Some("Guide\n"));
        assert_eq!(nested.path, "assets/guide.md");
    }

    #[test]
    fn preview_rejects_unknown_skills_and_escaping_paths() {
        let root = TempDir::new("skill-preview-paths");
        let (paths, library) = library_in(&root);
        write_payload(&paths.library().join("skills/review"), "safe");
        write_metadata(&paths.library(), &[record("skill-1", "review")]);

        assert!(matches!(
            library.skill_preview("missing", None),
            Err(ImportError::Validation(_))
        ));
        for path in [
            "../secret",
            "assets/../../secret",
            "/etc/passwd",
            "assets\\..\\secret",
        ] {
            assert!(matches!(
                library.skill_preview("skill-1", Some(path)),
                Err(ImportError::Validation(_))
            ));
        }
        write_metadata(&paths.library(), &[record("skill-1", "../outside")]);
        assert!(matches!(
            library.skill_preview("skill-1", None),
            Err(ImportError::Validation(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn preview_does_not_follow_file_or_directory_symlinks() {
        use std::os::unix::fs::symlink;

        let root = TempDir::new("skill-preview-symlink");
        let (paths, library) = library_in(&root);
        let skill = paths.library().join("skills/review");
        write_payload(&skill, "safe");
        let outside = root.path().join("outside");
        write_payload(&outside, "private");
        symlink(outside.join("SKILL.md"), skill.join("secret.md")).unwrap();
        symlink(&outside, skill.join("linked-directory")).unwrap();
        write_metadata(&paths.library(), &[record("skill-1", "review")]);

        let preview = library.skill_preview("skill-1", Some("secret.md")).unwrap();
        assert_eq!(preview.files, ["SKILL.md"]);
        assert_eq!(preview.content, None);
        assert!(preview.unavailable_reason.is_some());

        fs::remove_dir_all(&skill).unwrap();
        symlink(&outside, &skill).unwrap();
        assert!(matches!(
            library.skill_preview("skill-1", None),
            Err(ImportError::Validation(_))
        ));

        fs::remove_file(&skill).unwrap();
        fs::remove_dir(paths.library().join("skills")).unwrap();
        symlink(&outside, paths.library().join("skills")).unwrap();
        assert!(matches!(
            library.skill_preview("skill-1", None),
            Err(ImportError::Validation(_))
        ));
    }

    #[test]
    fn preview_reports_binary_and_oversized_files_without_returning_bytes() {
        let root = TempDir::new("skill-preview-unavailable");
        let (paths, library) = library_in(&root);
        let skill = paths.library().join("skills/review");
        write_payload(&skill, "safe");
        fs::write(skill.join("binary.dat"), [0, 1, 2]).unwrap();
        fs::write(
            skill.join("large.txt"),
            vec![b'a'; PREVIEW_CONTENT_LIMIT + 1],
        )
        .unwrap();
        write_metadata(&paths.library(), &[record("skill-1", "review")]);

        for path in ["binary.dat", "large.txt"] {
            let preview = library.skill_preview("skill-1", Some(path)).unwrap();
            assert_eq!(preview.content, None);
            assert!(preview.unavailable_reason.is_some());
            assert!(preview.files.contains(&path.to_owned()));
        }
    }

    #[test]
    fn preview_stops_after_the_library_entry_limit() {
        let root = TempDir::new("skill-preview-entries");
        let (paths, library) = library_in(&root);
        let skill = paths.library().join("skills/review");
        write_payload(&skill, "safe");
        write_metadata(&paths.library(), &[record("skill-1", "review")]);
        let limit = ValidationLimits::default().entries;
        for index in 1..limit {
            fs::write(skill.join(format!("file-{index}")), []).unwrap();
        }
        assert_eq!(
            library.skill_preview("skill-1", None).unwrap().files.len(),
            limit
        );

        fs::write(skill.join("one-too-many"), []).unwrap();
        assert_eq!(
            library.skill_preview("skill-1", None).unwrap_err(),
            ImportError::Validation(format!("skill exceeds preview limit of {limit} entries"))
        );
    }

    #[test]
    fn preview_stops_before_descending_beyond_the_depth_limit() {
        let root = TempDir::new("skill-preview-depth");
        let (paths, library) = library_in(&root);
        let skill = paths.library().join("skills/review");
        write_payload(&skill, "safe");
        write_metadata(&paths.library(), &[record("skill-1", "review")]);
        let mut deepest = skill;
        for _ in 0..PREVIEW_MAX_DEPTH {
            deepest = deepest.join("nested");
            fs::create_dir(&deepest).unwrap();
        }
        assert_eq!(
            library.skill_preview("skill-1", None).unwrap().files,
            ["SKILL.md"]
        );

        fs::create_dir(deepest.join("one-too-deep")).unwrap();
        assert_eq!(
            library.skill_preview("skill-1", None).unwrap_err(),
            ImportError::Validation(format!(
                "skill exceeds preview depth limit of {PREVIEW_MAX_DEPTH}"
            ))
        );
    }

    #[test]
    fn a_slug_names_its_own_directory_until_two_skills_share_it() {
        let shared = [
            ("aaaa1111-0000-0000-0000-000000000000", "caveman"),
            ("bbbb2222-0000-0000-0000-000000000000", "apple-design"),
            ("cccc3333-0000-0000-0000-000000000000", "caveman"),
        ];
        let directories = payload_directories(shared);
        assert_eq!(
            directories["aaaa1111-0000-0000-0000-000000000000"],
            "caveman"
        );
        assert_eq!(
            directories["bbbb2222-0000-0000-0000-000000000000"],
            "apple-design"
        );
        assert_eq!(
            directories["cccc3333-0000-0000-0000-000000000000"],
            "caveman-cccc3333"
        );

        let reversed = payload_directories(shared.into_iter().rev());
        assert_eq!(reversed, directories);
    }

    fn shared_slug_library(
        root: &TempDir,
        keeper: &str,
        drop: &str,
    ) -> (AppPaths, FilesystemLibraryRepository) {
        let (paths, library) = library_in(root);
        let skills = paths.library().join("skills");
        write_payload(&skills.join("caveman"), "first copy");
        write_payload(&skills.join(format!("caveman-{drop}")), "second copy");
        write_metadata(
            &paths.library(),
            &[record("aaaa1111", "caveman"), record("bbbb2222", "caveman")],
        );
        let _ = (keeper, drop);
        (paths, library)
    }

    #[test]
    fn keeping_the_suffixed_copy_promotes_it_to_the_bare_slug() {
        let root = TempDir::new("library-resolution");
        let (paths, library) = shared_slug_library(&root, "bbbb2222", "bbbb2222");
        let skills = paths.library().join("skills");
        let resolution = ConflictResolution {
            operation_id: "operation".into(),
            slug: "caveman".into(),
            keep_skill_id: "bbbb2222".into(),
            drop_skill_ids: vec!["aaaa1111".into()],
        };

        let kept = library.apply_resolution(&resolution).expect("resolution");

        assert_eq!(kept, "caveman");
        assert_eq!(
            fs::read_to_string(skills.join("caveman/SKILL.md")).unwrap(),
            "second copy"
        );
        assert!(fs::symlink_metadata(skills.join("caveman-bbbb2222")).is_err());
        assert_eq!(
            fs::read_to_string(
                paths
                    .data
                    .join("backups/resolutions/operation/caveman/SKILL.md")
            )
            .unwrap(),
            "first copy"
        );
        let catalog = library.catalog().expect("catalog");
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].skill_id, "bbbb2222");
        assert_eq!(catalog[0].payload_directory, "caveman");
        assert_eq!(directory_names(&skills), vec!["caveman".to_owned()]);
    }

    #[test]
    fn applying_a_resolution_again_changes_nothing() {
        let root = TempDir::new("library-resolution-resume");
        let (paths, library) = shared_slug_library(&root, "bbbb2222", "bbbb2222");
        let resolution = ConflictResolution {
            operation_id: "operation".into(),
            slug: "caveman".into(),
            keep_skill_id: "bbbb2222".into(),
            drop_skill_ids: vec!["aaaa1111".into()],
        };

        library.apply_resolution(&resolution).expect("first");
        let kept = library.apply_resolution(&resolution).expect("second");

        assert_eq!(kept, "caveman");
        assert_eq!(
            directory_names(&paths.library().join("skills")),
            vec!["caveman".to_owned()]
        );
        assert_eq!(library.catalog().expect("catalog").len(), 1);
    }

    #[test]
    fn a_discarded_copy_that_is_already_gone_still_settles_the_library() {
        let root = TempDir::new("library-resolution-missing");
        let (paths, library) = library_in(&root);
        let skills = paths.library().join("skills");
        write_payload(&skills.join("caveman-bbbb2222"), "second copy");
        write_metadata(
            &paths.library(),
            &[record("aaaa1111", "caveman"), record("bbbb2222", "caveman")],
        );
        let resolution = ConflictResolution {
            operation_id: "operation".into(),
            slug: "caveman".into(),
            keep_skill_id: "bbbb2222".into(),
            drop_skill_ids: vec!["aaaa1111".into()],
        };

        let kept = library.apply_resolution(&resolution).expect("resolution");

        assert_eq!(kept, "caveman");
        assert_eq!(
            fs::read_to_string(skills.join("caveman/SKILL.md")).unwrap(),
            "second copy"
        );
        let catalog = library.catalog().expect("catalog");
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].payload_directory, "caveman");
    }

    #[test]
    fn a_joining_skill_that_owns_the_bare_slug_moves_the_current_owner_aside() {
        let root = TempDir::new("library-displacement");
        let (paths, library) = library_in(&root);
        let skills = paths.library().join("skills");
        write_payload(&skills.join("caveman"), "bare owner");
        write_payload(&skills.join("caveman-cccc3333"), "suffixed owner");
        write_metadata(
            &paths.library(),
            &[record("bbbb2222", "caveman"), record("cccc3333", "caveman")],
        );
        write_payload(&paths.data.join("staging/import/plan/aaaa1111"), "joining");

        library
            .move_staged_payload("plan", "aaaa1111", "caveman")
            .expect("move joining payload");

        assert_eq!(
            fs::read_to_string(skills.join("caveman/SKILL.md")).unwrap(),
            "joining"
        );
        assert_eq!(
            fs::read_to_string(skills.join("caveman-bbbb2222/SKILL.md")).unwrap(),
            "bare owner"
        );
        assert_eq!(
            fs::read_to_string(skills.join("caveman-cccc3333/SKILL.md")).unwrap(),
            "suffixed owner"
        );

        let mut expected: Vec<String> = payload_directories([
            ("aaaa1111", "caveman"),
            ("bbbb2222", "caveman"),
            ("cccc3333", "caveman"),
        ])
        .into_values()
        .collect();
        expected.sort();
        assert_eq!(directory_names(&skills), expected);
    }
}
