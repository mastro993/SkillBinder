use crate::{persistence, worker::State};
use skillbinder_proto::{AppError, AppResult, Bootstrap, ErrorCategory, Library, OnboardingStep};
use std::{collections::BTreeMap, path::PathBuf};

/// Filesystem boundaries captured once at application startup.
#[derive(Clone, Debug)]
pub struct EngineConfig {
    /// Fresh platform-local application data. Never the legacy home directory.
    pub data_dir: PathBuf,
    /// Home boundary for registry locations.
    pub home: PathBuf,
    /// Environment used solely to expand declarative registry paths.
    pub environment: BTreeMap<String, String>,
}
impl EngineConfig {
    /// Resolves the operating system's local application-data location.
    pub fn platform() -> AppResult<Self> {
        let dirs = directories::BaseDirs::new().ok_or_else(AppError::storage)?;
        Ok(Self {
            data_dir: dirs.data_local_dir().join("SkillBinder"),
            home: dirs.home_dir().to_path_buf(),
            environment: std::env::vars().collect(),
        })
    }
    /// Creates an isolated engine configuration for tests or native fixtures.
    pub fn isolated(data_dir: PathBuf, home: PathBuf) -> Self {
        Self {
            data_dir,
            home,
            environment: BTreeMap::new(),
        }
    }
}
impl State {
    pub(crate) fn bootstrap(&self) -> AppResult<Bootstrap> {
        let step = self.setting("onboarding")?.unwrap_or_default();
        Ok(Bootstrap {
            step,
            git_version: self.git.as_ref().map(|git| git.version().to_owned()),
            storage_writable: persistence::writable(&self.config.data_dir),
            library_path: self.library_dir().display().to_string(),
            logs_path: self.config.data_dir.join("logs").display().to_string(),
        })
    }
    pub(crate) fn set_onboarding(&mut self, next: OnboardingStep) -> AppResult<Bootstrap> {
        let current = self.bootstrap()?;
        if current.step == OnboardingStep::Complete {
            return Ok(current);
        }
        if next == OnboardingStep::Complete || (next as u8) > (current.step as u8) + 1 {
            return Err(AppError::validation(
                "Complete the current setup step first.",
            ));
        }
        if next > OnboardingStep::Prerequisites
            && (current.git_version.is_none() || !current.storage_writable)
        {
            return Err(AppError::new(
                ErrorCategory::Storage,
                "Git 2.39 or newer and writable storage are required.",
                "Install Git or fix storage access, then recheck.",
            ));
        }
        self.set_setting("onboarding", &next)?;
        self.bootstrap()
    }
    pub(crate) fn complete_onboarding(&mut self) -> AppResult<Bootstrap> {
        let boot = self.bootstrap()?;
        if boot.step == OnboardingStep::Complete {
            return Ok(boot);
        }
        if boot.step != OnboardingStep::Ready {
            return Err(AppError::validation(
                "Finish all setup steps before creating the library.",
            ));
        }
        if !boot.storage_writable {
            return Err(AppError::storage());
        }
        let git = self
            .git
            .as_ref()
            .ok_or_else(|| AppError::validation("Install Git 2.39 or newer, then recheck."))?;
        let target = self.library_dir();
        if target.exists() {
            self.read_library()?;
        } else {
            let staged = self
                .config
                .data_dir
                .join("staging")
                .join(format!("initialize-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(staged.join("skills")).map_err(|_| AppError::storage())?;
            let library = Library {
                schema_version: 2,
                library_id: uuid::Uuid::new_v4().to_string(),
                created_at: format_timestamp(),
                content_policy_version: 1,
                folders: Vec::new(),
                tags: Vec::new(),
                skills: Vec::new(),
            };
            persistence::write_json(&staged.join(".skillbinder.json"), &library)?;
            git.initialize(&staged)?;
            std::fs::rename(&staged, &target).map_err(|_| AppError::storage())?;
            persistence::sync_directory(&self.config.data_dir)?;
        }
        self.set_setting("onboarding", &OnboardingStep::Complete)?;
        self.bootstrap()
    }
}
fn format_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}
