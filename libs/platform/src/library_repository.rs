use crate::{git::GitEnvironment, paths::AppPaths, payload_filesystem::FilesystemPayloadSource};
use serde_json::json;
use skillbinder_core::{
    discovery::scan::LibraryCatalog,
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
}
impl LibraryRepository for FilesystemLibraryRepository {
    fn catalog(&self) -> Result<Vec<LibraryRecord>, ImportError> {
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
        let meta = self.root().join(".skillbinder");
        fs::create_dir_all(meta.join("skills")).map_err(Self::map)?;
        fs::create_dir_all(meta.join("manifests")).map_err(Self::map)?;
        fs::write(meta.join("skills").join(format!("{skill}.json")),serde_json::to_vec_pretty(&json!({"schemaVersion":1,"id":skill,"slug":slug,"displayName":null,"folderId":null,"tagIds":[],"upstreamBindings":[]})).map_err(Self::map)?).map_err(Self::map)?;
        fs::write(
            meta.join("manifests").join(format!("{skill}.json")),
            serde_json::to_vec_pretty(&model.manifest).map_err(Self::map)?,
        )
        .map_err(Self::map)?;
        Ok(())
    }
    fn remove_record_and_manifest(&self, skill: &str) -> Result<(), ImportError> {
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
