use crate::{
    git::{GitEnvironment, GitError, VerifiedGit as PlatformVerifiedGit},
    paths::AppPaths,
    portable_metadata::PortableMetadata,
};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use skillbinder_core::bootstrap::{
    BootstrapEnvironment, BootstrapError, CompletedOnboarding, GitCheckError, LibrarySnapshot,
    LibraryStatus, VerifiedGit,
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct LocalEnvironment {
    paths: AppPaths,
    git: GitEnvironment,
}

impl LocalEnvironment {
    pub fn new(paths: AppPaths) -> Self {
        Self {
            paths,
            git: GitEnvironment::default(),
        }
    }

    fn verify_storage_access(&self) -> Result<(), BootstrapError> {
        self.paths
            .create_base_directories()
            .map_err(storage_error)?;
        let probe = self.paths.data.join(".write-probe");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&probe)
            .map_err(storage_error)?;
        file.write_all(b"skillbinder").map_err(storage_error)?;
        file.sync_all().map_err(storage_error)?;
        fs::remove_file(probe).map_err(storage_error)
    }

    fn inspect_local_library(&self) -> LibrarySnapshot {
        if !self.paths.library().exists() {
            return LibrarySnapshot {
                status: LibraryStatus::NotCreated,
                current_revision: None,
            };
        }
        let Ok(Some(_)) = self.read_library_record() else {
            return LibrarySnapshot {
                status: LibraryStatus::RecoveryRequired,
                current_revision: None,
            };
        };
        let Ok(git) = self.git.verify() else {
            tracing::warn!(
                event = "library_revision_skipped",
                state = "ready",
                "library revision could not be verified and was skipped"
            );
            return LibrarySnapshot {
                status: LibraryStatus::Ready,
                current_revision: None,
            };
        };
        match self.current_revision(&git) {
            Ok(revision) => LibrarySnapshot {
                status: LibraryStatus::Ready,
                current_revision: Some(revision),
            },
            Err(_) => LibrarySnapshot {
                status: LibraryStatus::RecoveryRequired,
                current_revision: None,
            },
        }
    }

    fn create_or_open_library(&self) -> Result<CompletedOnboarding, BootstrapError> {
        self.verify_storage_access()?;
        let git = self.git.verify().map_err(map_git_error)?;

        if let Some(existing) = self.read_library_record()? {
            return Ok(CompletedOnboarding {
                library_id: existing.library_id,
                current_revision: self.current_revision(&git)?,
            });
        }

        let staging = self
            .paths
            .data
            .join("staging")
            .join("library-initialization");
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(storage_error)?;
        }
        fs::create_dir_all(staging.join("skills")).map_err(storage_error)?;

        let record = LibraryRecord {
            schema_version: 1,
            library_id: Uuid::new_v4().to_string(),
            created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            content_policy_version: 1,
        };
        PortableMetadata::new(record.library_id.clone(), record.created_at.clone())
            .write(&staging.join(".skillbinder.json"))
            .map_err(BootstrapError::RecoveryRequired)?;

        let empty_hooks = self.paths.config.join("git-hooks");
        fs::create_dir_all(&empty_hooks).map_err(storage_error)?;
        self.git
            .run(
                &git,
                [
                    "-c".to_owned(),
                    format!("core.hooksPath={}", empty_hooks.display()),
                    "init".to_owned(),
                    "--initial-branch=main".to_owned(),
                    "--".to_owned(),
                    staging.to_string_lossy().into_owned(),
                ],
            )
            .map_err(map_git_error)?;
        self.run_library_git(&git, &staging, &["add", "--", ".skillbinder.json"])?;
        self.run_library_git(
            &git,
            &staging,
            &[
                "-c",
                "user.name=SkillBinder",
                "-c",
                "user.email=local@skillbinder.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--no-verify",
                "-m",
                "Initialize SkillBinder library",
            ],
        )?;

        match fs::rename(&staging, self.paths.library()) {
            Ok(()) => {}
            Err(error) if self.paths.library().exists() => {
                return Err(BootstrapError::RecoveryRequired(format!(
                    "library appeared during setup: {error}"
                )));
            }
            Err(error) => return Err(storage_error(error)),
        }

        Ok(CompletedOnboarding {
            library_id: record.library_id,
            current_revision: self.current_revision(&git)?,
        })
    }

    fn current_revision(&self, git: &PlatformVerifiedGit) -> Result<String, BootstrapError> {
        let output = self.run_library_git(git, &self.paths.library(), &["rev-parse", "HEAD"])?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    fn run_library_git(
        &self,
        git: &PlatformVerifiedGit,
        repository: &Path,
        operation: &[&str],
    ) -> Result<std::process::Output, BootstrapError> {
        let hooks_config = format!(
            "core.hooksPath={}",
            self.paths.config.join("git-hooks").display()
        );
        let mut args = vec![
            "-c".to_owned(),
            hooks_config,
            "-c".to_owned(),
            "core.autocrlf=false".to_owned(),
            "-C".to_owned(),
            repository.to_string_lossy().into_owned(),
        ];
        args.extend(operation.iter().map(|value| (*value).to_owned()));
        self.git
            .run(git, args)
            .map_err(map_git_error)
            .map_err(Into::into)
    }

    fn read_library_record(&self) -> Result<Option<LibraryRecord>, BootstrapError> {
        let portable = self.paths.library().join(".skillbinder.json");
        if portable.is_file() {
            let metadata =
                PortableMetadata::load(&portable).map_err(BootstrapError::RecoveryRequired)?;
            return Ok(Some(LibraryRecord {
                schema_version: metadata.schema_version,
                library_id: metadata.library_id,
                created_at: metadata.created_at,
                content_policy_version: metadata.content_policy_version,
            }));
        }
        // Read the old layout so existing local libraries remain recoverable after upgrade.
        let legacy = self
            .paths
            .library()
            .join(".skillbinder")
            .join("library.json");
        if !legacy.is_file() {
            return Ok(None);
        }
        let bytes = fs::read(legacy).map_err(storage_error)?;
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| BootstrapError::RecoveryRequired(error.to_string()))
    }
}

impl BootstrapEnvironment for LocalEnvironment {
    fn verify_storage(&self) -> Result<(), BootstrapError> {
        self.verify_storage_access()
    }

    fn verify_git(&self) -> Result<VerifiedGit, GitCheckError> {
        self.git
            .verify()
            .map(|git| VerifiedGit {
                executable: git.executable.to_string_lossy().into_owned(),
                version: git.version.to_string(),
            })
            .map_err(map_git_error)
    }

    fn inspect_library(&self) -> LibrarySnapshot {
        self.inspect_local_library()
    }

    fn complete_local(&self) -> Result<CompletedOnboarding, BootstrapError> {
        self.create_or_open_library()
    }
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LibraryRecord {
    schema_version: u32,
    library_id: String,
    created_at: String,
    content_policy_version: u32,
}

fn map_git_error(error: GitError) -> GitCheckError {
    match error {
        GitError::NotFound => GitCheckError::NotFound,
        GitError::Unsupported { found, minimum } => GitCheckError::Unsupported {
            found: found.to_string(),
            minimum: minimum.to_string(),
        },
        GitError::InvalidVersion(value) => GitCheckError::InvalidVersion(value),
        GitError::Start(error) => GitCheckError::Start(error.to_string()),
        GitError::ProcessFailed(detail) => GitCheckError::ProcessFailed(detail),
        GitError::ProcessTimeout => GitCheckError::ProcessTimeout,
    }
}

fn storage_error(error: std::io::Error) -> BootstrapError {
    BootstrapError::Storage(error.to_string())
}
