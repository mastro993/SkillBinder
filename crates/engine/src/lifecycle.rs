use crate::{
    app_update::{
        AppPlacement, RELEASE_KEY, STAMPED, TEAM, UpdatePayload, UpdateSource, running_version,
    },
    catalog::{BASE_URL, CatalogSource},
    git::Git,
    onboarding::{self, Facts, Offers},
    persistence,
    shutdown::Shutdown,
    worker::State,
};
use skillbinder_proto::{
    AppError, AppResult, AppVersion, Bootstrap, ImportSelection, OnboardingStep,
};
use std::{collections::BTreeMap, path::PathBuf};

/// Device setting saved once onboarding's import is applied or skipped.
const FINISHED: &str = "onboarding_finished";
/// Device setting saved once the Git step is skipped.
const GIT_SKIPPED: &str = "git_skipped";
/// Device setting saved once the command step is skipped or its Install succeeds.
const COMMAND_LINE_ASKED: &str = "command_line_asked";
/// Device setting saved once the agents step is skipped, continued, or set up.
const MCP_ASKED: &str = "mcp_asked";
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

/// Which URLs Explore's repositories accept.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepositoryUrls {
    /// HTTPS without credentials, SSH, and SCP-style URLs.
    Remote,
    /// Also `file://` URLs, so tests and fixtures can add local repositories.
    Local,
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
    /// Where Explore's catalog is fetched from.
    pub catalog: CatalogSource,
    /// Which URLs Explore's repositories accept.
    pub repository_urls: RepositoryUrls,
    /// The version app update checks compare against: the running build's.
    pub app_version: AppVersion,
    /// Where app update checks read their manifests; `None` turns them off.
    pub app_update: Option<UpdateSource>,
    /// Where the running app is, and where Move to Applications puts it.
    pub placement: AppPlacement,
    /// The folder the macOS command line tool's `usr/local/bin` and Homebrew prefixes are under:
    /// `/`, or a fixture's own, so a native check never links the user's real command.
    pub system_root: PathBuf,
}
impl EngineConfig {
    /// Resolves the operating system's local application-data location.
    pub fn platform() -> AppResult<Self> {
        let dirs = directories::BaseDirs::new().ok_or_else(AppError::storage)?;
        let base = url::Url::parse(BASE_URL).map_err(|_| AppError::storage())?;
        let environment: BTreeMap<_, _> = std::env::vars().collect();
        Ok(Self {
            placement: AppPlacement::platform(dirs.home_dir(), &environment),
            system_root: PathBuf::from("/"),
            data_dir: dirs.data_local_dir().join("SkillBinder"),
            home: dirs.home_dir().to_path_buf(),
            environment,
            git: GitProfile::User,
            git_search: GitSearch::System,
            catalog: CatalogSource::Http(base.clone()),
            repository_urls: RepositoryUrls::Remote,
            app_version: running_version(),
            // Only a build `build.yml` stamped as a release checks for app updates.
            app_update: STAMPED.then(|| UpdateSource {
                site: CatalogSource::Http(base),
                public_key: RELEASE_KEY.to_owned(),
                payload: if cfg!(windows) {
                    UpdatePayload::Setup
                } else if cfg!(target_os = "linux") {
                    UpdatePayload::AppImage
                } else {
                    UpdatePayload::DiskImage {
                        team: TEAM.to_owned(),
                    }
                },
            }),
        })
    }
    /// Creates an isolated engine configuration for tests or native fixtures. Git reads only
    /// `home`'s configuration, so a fixture never reaches the user's credentials or identity, and
    /// the catalog comes from `<data_dir>/catalog-source`, which is missing, so offline, until a
    /// fixture writes it. Repositories may be local `file://` ones. App updates are off until a
    /// fixture names a source. No bundle, setup-installed executable, or AppImage runs, Move to
    /// Applications would use the fixture's `<data_dir>/fixture-applications` and
    /// `<home>/Applications`, and the applications menu is under `<home>/.local/share`. The
    /// command line tool's macOS link goes under `<data_dir>/fixture-root`.
    pub fn isolated(data_dir: PathBuf, home: PathBuf) -> Self {
        Self {
            placement: AppPlacement {
                bundle: None,
                executable: None,
                applications: data_dir.join("fixture-applications"),
                user_applications: home.join("Applications"),
                appimage: None,
                data_home: home.join(".local/share"),
            },
            catalog: CatalogSource::Directory(data_dir.join("catalog-source")),
            system_root: data_dir.join("fixture-root"),
            data_dir,
            home,
            environment: BTreeMap::new(),
            git: GitProfile::Home,
            git_search: GitSearch::System,
            repository_urls: RepositoryUrls::Local,
            app_version: running_version(),
            app_update: None,
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
        let finished = self.onboarding_finished()?;
        Ok(Bootstrap {
            onboarding: onboarding::derive(Facts {
                storage_writable,
                git_found: self.git.is_some(),
                git_skipped: self.setting(GIT_SKIPPED)?.unwrap_or(false),
                agents_chosen: self.agent_choices()?.is_some(),
                offers: (!finished).then(|| self.offers()).transpose()?,
                finished,
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
    /// What the command and agents steps read. An agent counts once chosen, or while the choice
    /// is not saved, once found, since Choose your agents starts with every agent ticked.
    fn offers(&self) -> AppResult<Offers> {
        let saved = self.agent_choices()?.is_some();
        let agents = self.agents()?;
        let connectable = self.mcp_clients.connectable().into_iter().any(|client| {
            agents.iter().any(|agent| {
                agent.id == client.agent() && (agent.chosen || (!saved && agent.detected))
            })
        });
        Ok(Offers {
            command_line: self.command_line.status().state,
            connectable,
            command_line_asked: self.setting(COMMAND_LINE_ASKED)?.unwrap_or(false),
            mcp_asked: self.setting(MCP_ASKED)?.unwrap_or(false),
        })
    }
    /// Whether onboarding finished: its import was applied or skipped.
    pub(crate) fn onboarding_finished(&self) -> AppResult<bool> {
        Ok(self.setting(FINISHED)?.unwrap_or(false))
    }
    /// Carries data saved under the stored onboarding position over, once: a finished onboarding
    /// stays finished, choosing every detected agent at open, and any earlier position starts the
    /// derived steps.
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
    /// Settles the command step, skipped or installed (ADR 0027). Repeating it is harmless.
    pub(crate) fn settle_command_line(&self) -> AppResult<Bootstrap> {
        self.settle_step(COMMAND_LINE_ASKED)
    }
    /// Settles the agents step, skipped, continued, or set up (ADR 0027). Repeating it is
    /// harmless.
    pub(crate) fn settle_connect_agents(&self) -> AppResult<Bootstrap> {
        self.settle_step(MCP_ASKED)
    }
    /// Saves the step's fact, then creates the library when the import comes next.
    fn settle_step(&self, fact: &str) -> AppResult<Bootstrap> {
        self.set_setting(fact, &true)?;
        self.ensure_library()?;
        self.ensure_repository()?;
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
