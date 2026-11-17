use crate::{
    git::Git,
    onboarding::{self, Facts},
    persistence,
    shutdown::Shutdown,
    worker::State,
};
use skillbinder_proto::{AppError, AppResult, Bootstrap, ImportSelection, OnboardingStep};
use std::{collections::BTreeMap, path::PathBuf};

/// Device setting saved once onboarding's import is applied or skipped.
const FINISHED: &str = "onboarding_finished";
/// Device setting saved once the Git step is skipped.
const GIT_SKIPPED: &str = "git_skipped";
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

/// Where the engine looks for Git.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GitSearch {
    /// Every absolute `PATH` entry, then the usual install locations.
    System,
    /// Only this directory, so a fixture can run without Git and gain it later.
    Directory(PathBuf),
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
    /// Where Git is looked for.
    pub git_search: GitSearch,
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
            git_search: GitSearch::System,
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
            git_search: GitSearch::System,
        }
    }
    /// The usable Git where [`Self::git_search`] says to look, or else the version of an older
    /// one found there, if any.
    pub(crate) fn find_git(&self, shutdown: Shutdown) -> (Option<Git>, Option<String>) {
        let home = match self.git {
            GitProfile::User => None,
            GitProfile::Home => Some(self.home.as_path()),
        };
        match Git::discover(&self.git_search, home, shutdown) {
            Ok(git) => (Some(git), None),
            Err(outdated) => (None, outdated),
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
                git_skipped: self.setting(GIT_SKIPPED)?.unwrap_or(false),
                finished: self.onboarding_finished()?,
            }),
            git_version: self.git.as_ref().map(|git| git.version().to_owned()),
            outdated_git: self.outdated_git.clone(),
            git_install: self.installer.offer().clone(),
            git_install_failed: self.installer.failed(),
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
    /// Skips the Git step, so onboarding goes on without Git (ADR 0011). Repeating it is harmless.
    pub(crate) fn skip_git(&self) -> AppResult<Bootstrap> {
        self.set_setting(GIT_SKIPPED, &true)?;
        self.ensure_library()?;
        self.bootstrap()
    }
    /// Looks for Git again while none was found, noting how the Git installer started last
    /// ended.
    pub(crate) fn recheck_git(&mut self) {
        self.installer.poll();
        if self.git.is_none() {
            (self.git, self.outdated_git) = self.config.find_git(self.shutdown.clone());
        }
    }
    /// Creates the library as a plain directory on the way into onboarding's import, once the
    /// earlier steps are done. Repeating it, or calling it at any other step, changes nothing.
    pub(crate) fn ensure_library(&self) -> AppResult<()> {
        let target = self.library_dir();
        if target.exists() || self.bootstrap()?.onboarding.current() != Some(OnboardingStep::Import)
        {
            return Ok(());
        }
        let staged = self
            .config
            .data_dir
            .join("staging")
            .join(format!("initialize-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(staged.join("skills")).map_err(|_| AppError::storage())?;
        std::fs::rename(&staged, &target).map_err(|_| AppError::storage())?;
        persistence::sync_directory(&self.config.data_dir)
    }
    /// Makes an existing library a repository whenever Git is found and it is not one yet
    /// (ADR 0011). Its skills and records stay uncommitted until Sync or Push.
    pub(crate) fn ensure_repository(&self) -> AppResult<()> {
        let library = self.library_dir();
        match &self.git {
            Some(git) if library.is_dir() => git.initialize(&library),
            _ => Ok(()),
        }
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
