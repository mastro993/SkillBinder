use crate::{
    git::GitEnvironment,
    paths::AppPaths,
    payload_filesystem::FilesystemPayloadSource,
    portable_metadata::{PortableMetadata, PortableSkill},
};
use serde_json::json;
use skillbinder_core::{
    discovery::LibraryCatalog,
    import::{ImportError, ImportPlan, LibraryRecord, LibraryRepository},
    library::{Manifest, PayloadModel, ValidationLimits, inspect_payload},
};
use std::{ffi::OsString, fs, path::PathBuf};

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

    fn metadata(&self) -> Result<PortableMetadata, ImportError> {
        let path = self.root().join(".skillbinder.json");
        if path.is_file() {
            return PortableMetadata::load(&path).map_err(Self::map);
        }
        let legacy = self.root().join(".skillbinder/library.json");
        let bytes = fs::read(&legacy).map_err(Self::map)?;
        let record: serde_json::Value = serde_json::from_slice(&bytes).map_err(Self::map)?;
        let library_id = record
            .get("libraryId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| Self::map("library metadata has no library id"))?;
        let created_at = record
            .get("createdAt")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let mut metadata = PortableMetadata::new(library_id.to_owned(), created_at.to_owned());
        let skills = self.root().join(".skillbinder/skills");
        let manifests = self.root().join(".skillbinder/manifests");
        if skills.is_dir() && manifests.is_dir() {
            for item in fs::read_dir(skills).map_err(Self::map)? {
                let record_path = item.map_err(Self::map)?.path();
                if record_path.extension().and_then(|value| value.to_str()) != Some("json") {
                    continue;
                }
                let skill = record_path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_owned();
                let record: serde_json::Value =
                    serde_json::from_slice(&fs::read(&record_path).map_err(Self::map)?)
                        .map_err(Self::map)?;
                let manifest_path = manifests.join(format!("{skill}.json"));
                let manifest = serde_json::from_slice(&fs::read(manifest_path).map_err(Self::map)?)
                    .map_err(Self::map)?;
                metadata.skills.insert(
                    skill.clone(),
                    PortableSkill {
                        id: skill,
                        slug: record
                            .get("slug")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        display_name: record
                            .get("displayName")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned),
                        folder_id: record
                            .get("folderId")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned),
                        tag_ids: record
                            .get("tagIds")
                            .and_then(serde_json::Value::as_array)
                            .map(|values| {
                                values
                                    .iter()
                                    .filter_map(serde_json::Value::as_str)
                                    .map(str::to_owned)
                                    .collect()
                            })
                            .unwrap_or_default(),
                        upstream_bindings: record
                            .get("upstreamBindings")
                            .and_then(serde_json::Value::as_array)
                            .cloned()
                            .unwrap_or_default(),
                        manifest,
                    },
                );
            }
        }
        Ok(metadata)
    }

    fn write_metadata(&self, metadata: &PortableMetadata) -> Result<(), ImportError> {
        metadata
            .write(&self.root().join(".skillbinder.json"))
            .map_err(Self::map)
    }
}
impl LibraryRepository for FilesystemLibraryRepository {
    fn catalog(&self) -> Result<Vec<LibraryRecord>, ImportError> {
        let portable = self.root().join(".skillbinder.json");
        if portable.is_file() {
            let metadata = PortableMetadata::load(&portable).map_err(Self::map)?;
            return Ok(metadata
                .skills
                .into_values()
                .map(|skill| LibraryRecord {
                    skill_id: skill.id,
                    slug: skill.slug,
                    digest: skill.manifest.digest.clone(),
                    manifest: skill.manifest,
                })
                .collect());
        }
        let dir = self.root().join(".skillbinder/manifests");
        let mut out = Vec::new();
        if !dir.exists() {
            return Ok(out);
        }
        for item in fs::read_dir(dir).map_err(Self::map)? {
            let path = item.map_err(Self::map)?.path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let manifest: Manifest =
                serde_json::from_slice(&fs::read(&path).map_err(Self::map)?).map_err(Self::map)?;
            let skill_id = path
                .file_stem()
                .and_then(|x| x.to_str())
                .unwrap_or_default()
                .to_owned();
            let record_path = self
                .root()
                .join(".skillbinder/skills")
                .join(format!("{skill_id}.json"));
            let record: serde_json::Value =
                serde_json::from_slice(&fs::read(record_path).map_err(Self::map)?)
                    .map_err(Self::map)?;
            let slug = record
                .get("slug")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_owned();
            out.push(LibraryRecord {
                skill_id,
                slug,
                digest: manifest.digest.clone(),
                manifest,
            });
        }
        Ok(out)
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
        let target = self.root().join("skills").join(skill).join(slug);
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
        let mut metadata = self.metadata()?;
        metadata.skills.insert(
            skill.to_owned(),
            PortableSkill {
                id: skill.to_owned(),
                slug: slug.to_owned(),
                display_name: None,
                folder_id: None,
                tag_ids: Vec::new(),
                upstream_bindings: Vec::new(),
                manifest: model.manifest.clone(),
            },
        );
        self.write_metadata(&metadata)?;
        Ok(())
    }
    fn remove_record_and_manifest(&self, skill: &str) -> Result<(), ImportError> {
        let portable = self.root().join(".skillbinder.json");
        if portable.is_file() {
            let mut metadata = PortableMetadata::load(&portable).map_err(Self::map)?;
            metadata.skills.remove(skill);
            return self.write_metadata(&metadata);
        }
        let root = self.root();
        for relative in [
            format!(".skillbinder/skills/{skill}.json"),
            format!(".skillbinder/manifests/{skill}.json"),
        ] {
            let path = root.join(relative);
            if path.exists() {
                fs::remove_file(path).map_err(Self::map)?;
            }
        }
        Ok(())
    }
    fn rollback_import(&self, plan: &str, moved: &[(String, String)]) -> Result<(), ImportError> {
        for (skill, slug) in moved {
            let target = self.root().join("skills").join(skill).join(slug);
            if fs::symlink_metadata(&target).is_ok() {
                fs::remove_dir_all(&target).map_err(Self::map)?;
            }
            if let Some(parent) = target.parent() {
                let _ = fs::remove_dir(parent);
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
                    OsString::from(".skillbinder"),
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
        let items: Vec<_> = plan
            .items
            .iter()
            .map(|item| {
                json!({
                    "skillId": item.skill_id,
                    "slug": item.selection.slug,
                    "targetPath": self.root().join("skills").join(&item.skill_id).join(&item.selection.slug),
                })
            })
            .collect();
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
