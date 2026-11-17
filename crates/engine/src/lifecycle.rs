use crate::{
    onboarding::{self, Facts},
    persistence,
    worker::State,
};
use skillbinder_proto::{AppError, AppResult, Bootstrap, ImportSelection, OnboardingStep};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Device setting saved once onboarding's import is applied or skipped.
const FINISHED: &str = "onboarding_finished";
/// Device setting that stored the onboarding position before steps were derived from facts.
const LEGACY_POSITION: &str = "onboarding";

/// Whose Git configuration the engine's Git commands read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GitProfile {
    /// The user's own, with their credential helpers, SSH setup, and URL rewrites.
    User,
    /// Only the configured home's, never the user's or the system's.
    Home,
}

/// Filesystem boundaries captured once at application startup.
#[derive(Clone, Debug)]
pub struct EngineConfig {
    /// Fresh platform-local application data. Never the legacy home directory.
    pub data_dir: PathBuf,
    /// Home boundary for registry locations.
    pub home: PathBuf,
    /// Environment used solely to expand declarative registry paths.
    pub environment: BTreeMap<String, String>,
    /// Whose Git configuration Git commands read.
    pub git: GitProfile,
}
impl EngineConfig {
    /// Resolves the operating system's local application-data location.
    pub fn platform() -> AppResult<Self> {
        let dirs = directories::BaseDirs::new().ok_or_else(AppError::storage)?;
        Ok(Self {
            data_dir: dirs.data_local_dir().join("SkillBinder"),
            home: dirs.home_dir().to_path_buf(),
            environment: std::env::vars().collect(),
            git: GitProfile::User,
        })
    }
    /// Creates an isolated engine configuration for tests or native fixtures. Git reads only
    /// `home`'s configuration, so a fixture never reaches the user's credentials or identity.
    pub fn isolated(data_dir: PathBuf, home: PathBuf) -> Self {
        Self {
            data_dir,
            home,
            environment: BTreeMap::new(),
            git: GitProfile::Home,
        }
    }
    pub(crate) fn git_home(&self) -> Option<&Path> {
        match self.git {
            GitProfile::User => None,
            GitProfile::Home => Some(&self.home),
        }
    }
}
impl State {
    pub(crate) fn bootstrap(&self) -> AppResult<Bootstrap> {
        let storage_writable = persistence::writable(&self.config.data_dir);
        Ok(Bootstrap {
            onboarding: onboarding::derive(Facts {
                storage_writable,
                git_found: self.git.is_some(),
                finished: self.onboarding_finished()?,
            }),
            git_version: self.git.as_ref().map(|git| git.version().to_owned()),
            storage_writable,
            data_path: self.config.data_dir.display().to_string(),
            library_path: self.library_dir().display().to_string(),
            logs_path: self.config.data_dir.join("logs").display().to_string(),
        })
    }
    /// Whether onboarding finished: its import was applied or skipped.
    pub(crate) fn onboarding_finished(&self) -> AppResult<bool> {
        Ok(self.setting(FINISHED)?.unwrap_or(false))
    }
    /// Carries data saved under the stored onboarding position over, once: a finished onboarding
    /// stays finished, and any earlier position starts the derived steps.
    pub(crate) fn forget_onboarding_position(&self) -> AppResult<()> {
        let Some(position) = self.setting::<String>(LEGACY_POSITION)? else {
            return Ok(());
        };
        if position == "Complete" {
            self.set_setting(FINISHED, &true)?;
        }
        self.remove_setting(LEGACY_POSITION)
    }
    /// Creates the library on the way into onboarding's import, once the earlier steps are done.
    /// Repeating it, or calling it at any other step, changes nothing.
    pub(crate) fn ensure_library(&self) -> AppResult<()> {
        if self.bootstrap()?.onboarding.current() != Some(OnboardingStep::Import) {
            return Ok(());
        }
        let git = self
            .git
            .as_ref()
            .ok_or_else(|| AppError::validation("Install Git 2.39 or newer, then recheck."))?;
        let target = self.library_dir();
        if target.exists() {
            return git.initialize(&target);
        }
        let staged = self
            .config
            .data_dir
            .join("staging")
            .join(format!("initialize-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(staged.join("skills")).map_err(|_| AppError::storage())?;
        git.initialize(&staged)?;
        std::fs::rename(&staged, &target).map_err(|_| AppError::storage())?;
        persistence::sync_directory(&self.config.data_dir)
    }
    /// Finishes onboarding at its import, first applying the reviewed first import, if any; a
    /// failed import leaves onboarding at its import. Refused while an earlier step is not done.
    pub(crate) fn finish_onboarding(
        &mut self,
        import: Option<ImportSelection>,
    ) -> AppResult<Bootstrap> {
        let boot = self.bootstrap()?;
        if boot.onboarding.finished {
            return Ok(boot);
        }
        if boot.onboarding.current() != Some(OnboardingStep::Import) {
            return Err(AppError::validation(
                "Finish the earlier setup steps first.",
            ));
        }
        self.ensure_library()?;
        if let Some(import) = import {
            self.apply_import(import.plan, import.skills, import.acknowledge_invalid)?;
        }
        self.set_setting(FINISHED, &true)?;
        self.adopt_agent_copies();
        self.bootstrap()
    }
}
/// The current UTC time in RFC 3339, as import times record it.
pub(crate) fn format_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}
