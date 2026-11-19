use crate::{
    activation::Progress,
    catalog::{Catalogs, Previews, Resolutions, Searches},
    database,
    git::{Git, Installer},
    git_pool::GitPool,
    grants::GrantedDirectory,
    install::Installs,
    lifecycle::EngineConfig,
    logging::Logger,
    reconcile::RECONCILED_HEAD,
    scan::{LibraryIndex, Scans},
    shutdown::Shutdown,
    sources::Updates,
};
use diesel::SqliteConnection;
use fs2::FileExt;
use skillbinder_proto::*;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread::JoinHandle,
};
use tokio::sync::oneshot;

mod activation;
mod catalog;
mod install;
mod onboarding;
mod preview;
mod skills_sh;
mod sources;

const DATABASE: &str = "skillbinder.db";
type Job = Box<dyn FnOnce(&mut State) + Send>;
enum Message {
    Execute(Job),
    Shutdown(oneshot::Sender<()>),
}
struct Owner {
    sender: mpsc::Sender<Message>,
    thread: Mutex<Option<JoinHandle<AppResult<()>>>>,
    scans: Scans,
    updates: Updates,
    catalog: Catalogs,
    installs: Installs,
    previews: Previews,
    /// Explore's skills.sh search, on a thread of its own.
    searches: Searches,
    /// The folders of skills.sh results, resolved on the pool.
    resolutions: Resolutions,
    /// Runs installs' network phases, previews, and resolutions off the worker.
    pool: GitPool,
    progress: Progress,
    /// Stops in-flight commands when the application quits.
    shutdown: Shutdown,
}
/// Cloneable typed handle to the embedded engine worker.
#[derive(Clone)]
pub struct Engine {
    owner: Arc<Owner>,
}
pub(crate) struct State {
    pub config: EngineConfig,
    pub database: RefCell<SqliteConnection>,
    pub git: Option<Git>,
    /// The version of a Git older than 2.39, found while no usable one is.
    pub outdated_git: Option<String>,
    /// How this computer installs Git, and the installer started last.
    pub installer: Installer,
    pub log: Logger,
    pub scans: Scans,
    pub updates: Updates,
    /// Explore's catalog, which snapshots read.
    pub catalog: Catalogs,
    /// Installs from Explore; their caches are kept from pruning while they run.
    pub installs: Installs,
    /// The open detail dialog's preview, whose catalog cache is kept from pruning.
    pub previews: Previews,
    /// Explore's skills.sh search, which snapshots read.
    pub searches: Searches,
    /// The folders of skills.sh results, which snapshots read.
    pub resolutions: Resolutions,
    /// How far a tracked change has got; passes count their pairs into it.
    pub progress: Progress,
    pub grants: BTreeMap<GrantId, GrantedDirectory>,
    pub recovery_required: bool,
    /// Stops the work a quit would otherwise wait for.
    pub shutdown: Shutdown,
    lock: File,
}
impl Engine {
    /// Opens fresh storage, obtains the exclusive library writer lock, and recovers journals.
    /// Call before creating native windows or from a blocking task.
    pub fn open(mut config: EngineConfig) -> AppResult<Self> {
        fs::create_dir_all(&config.data_dir).map_err(|_| AppError::storage())?;
        config.data_dir = fs::canonicalize(&config.data_dir).map_err(|_| AppError::storage())?;
        fs::create_dir_all(&config.home).map_err(|_| AppError::storage())?;
        config.home = fs::canonicalize(&config.home).map_err(|_| AppError::storage())?;
        for name in ["staging", "logs", "icons"] {
            fs::create_dir_all(config.data_dir.join(name)).map_err(|_| AppError::storage())?;
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(config.data_dir.join("library.lock"))
            .map_err(|_| AppError::storage())?;
        FileExt::try_lock_exclusive(&lock).map_err(|_| {
            AppError::new(
                ErrorCategory::Busy,
                "Another SkillBinder process owns this library.",
                "Activate the running window.",
            )
        })?;
        let log = Logger::start(config.data_dir.join("logs"), &config.home)
            .map_err(|_| AppError::storage())?;
        let database = RefCell::new(database::open(&config.data_dir.join(DATABASE))?);
        let scans = Scans::default();
        let updates = Updates::default();
        let progress = Progress::default();
        let shutdown = Shutdown::default();
        let catalog = Catalogs::open(config.catalog.clone(), &config.data_dir, shutdown.clone());
        let installs = Installs::default();
        let previews = Previews::default();
        let searches = Searches::new(config.catalog.clone());
        let resolutions = Resolutions::default();
        let pool = GitPool::default();
        let (git, outdated_git) = config.find_git(shutdown.clone());
        let mut state = State {
            config,
            database,
            git,
            outdated_git,
            installer: Installer::detect(),
            log,
            scans: scans.clone(),
            updates: updates.clone(),
            catalog: catalog.clone(),
            installs: installs.clone(),
            previews: previews.clone(),
            searches: searches.clone(),
            resolutions: resolutions.clone(),
            progress: progress.clone(),
            grants: BTreeMap::new(),
            recovery_required: true,
            shutdown: shutdown.clone(),
            lock,
        };
        state.recover()?;
        state.forget_onboarding_position()?;
        // Forgetting an agent folder hands its copies to your agents, so they are chosen first.
        state.choose_agents_of_finished_onboarding()?;
        state.forget_agent_folders()?;
        let (sender, receiver) = mpsc::channel();
        // Install fetches end before the library lock is released.
        let workers = pool.clone();
        let thread = std::thread::Builder::new()
            .name("skillbinder-engine".into())
            .spawn(move || {
                // Engine::open returns before this runs; queued requests wait for it.
                if let Err(error) = state
                    .ensure_library()
                    .and_then(|()| state.ensure_repository())
                {
                    state
                        .log
                        .record(format!("Library creation will retry: {error}"));
                }
                if state.library_dir().exists() {
                    state.catch_up();
                }
                for message in receiver {
                    match message {
                        Message::Execute(job) => job(&mut state),
                        Message::Shutdown(done) => {
                            state.scans.wait_for_shutdown();
                            state.updates.wait_for_shutdown();
                            workers.wait_for_shutdown();
                            state.log.flush();
                            let _ = done.send(());
                            break;
                        }
                    }
                }
                state.scans.wait_for_shutdown();
                state.updates.wait_for_shutdown();
                workers.wait_for_shutdown();
                state.log.flush();
                drop(state.database);
                FileExt::unlock(&state.lock).map_err(|_| AppError::storage())
            })
            .map_err(|_| AppError::storage())?;
        Ok(Self {
            owner: Arc::new(Owner {
                sender,
                thread: Mutex::new(Some(thread)),
                scans,
                updates,
                catalog,
                installs,
                previews,
                searches,
                resolutions,
                pool,
                progress,
                shutdown,
            }),
        })
    }
    pub(crate) async fn call<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut State) -> AppResult<T> + Send + 'static,
    ) -> AppResult<T> {
        let (sender, receiver) = oneshot::channel();
        self.owner
            .sender
            .send(Message::Execute(Box::new(move |state| {
                let result = operation(state);
                if let Err(error) = &result
                    && error.category != ErrorCategory::Cancelled
                {
                    state.log.record(error.to_string());
                }
                let _ = sender.send(result);
            })))
            .map_err(|_| {
                AppError::new(
                    ErrorCategory::Cancelled,
                    "SkillBinder is shutting down.",
                    "Restart the application.",
                )
            })?;
        receiver.await.map_err(|_| AppError::storage())?
    }
    /// Cancels scanning and in-flight Git commands, drains committed mutations, flushes logs,
    /// and joins the worker.
    pub async fn shutdown(&self) -> AppResult<()> {
        // A command waiting on the network stops now, instead of holding the quit for its
        // timeout: the native quit path drains the engine on the main thread.
        self.owner.shutdown.cancel();
        let (sender, receiver) = oneshot::channel();
        self.owner
            .sender
            .send(Message::Shutdown(sender))
            .map_err(|_| AppError::storage())?;
        receiver.await.map_err(|_| AppError::storage())?;
        if let Some(thread) = self
            .owner
            .thread
            .lock()
            .map_err(|_| AppError::storage())?
            .take()
        {
            thread.join().map_err(|_| AppError::storage())??;
        }
        Ok(())
    }
    /// Captures state from the durable worker and the independent scan owner.
    pub async fn snapshot(&self) -> AppResult<AppSnapshot> {
        self.call(State::snapshot).await
    }
    /// Issues a short-lived capability for a native-picked directory.
    pub async fn grant_directory(&self, path: PathBuf) -> AppResult<DirectoryGrant> {
        self.call(move |state| state.grant_directory(path)).await
    }
    /// Scans the agents' global skill directories and every workspace in the background. A running scan is replaced, so the
    /// new one covers the current workspaces; the previous results stay until it completes.
    pub async fn start_scan(&self) -> AppResult<()> {
        self.scan(true).await
    }
    /// Like [`Self::start_scan`], but keeps a running scan instead of replacing it, so repeated
    /// refreshes cannot keep a slow scan from completing.
    pub async fn refresh_scan(&self) -> AppResult<()> {
        self.scan(false).await
    }
    async fn scan(&self, replace: bool) -> AppResult<()> {
        let (config, reach, index, log) = self
            .call(|state| {
                state.apply_pending();
                Ok((
                    state.config.clone(),
                    state.reach()?,
                    state.library_index()?,
                    state.log.clone(),
                ))
            })
            .await?;
        self.owner.scans.start(config, reach, index, log, replace)
    }
    /// The last completed scan and whether another one is running, without waiting for a
    /// durable mutation.
    pub fn current_scan(&self) -> AppResult<(Option<ScanSnapshot>, bool)> {
        self.owner.scans.current()
    }
    /// Whether a source check is running, so the client can publish the header's state without
    /// waiting for the check to finish.
    pub fn check_running(&self) -> AppResult<bool> {
        self.owner.updates.running()
    }
    /// Scans the agents' global skill directories and every workspace to completion, replacing a
    /// running scan, then plans every found skill not in library: the version `versions` chooses
    /// for it, else the preferred one (ADR 0009).
    pub async fn prepare_found_import(&self, versions: Vec<CandidateId>) -> AppResult<FoundImport> {
        self.start_scan().await?;
        self.owner
            .scans
            .settled()?
            .await
            .map_err(|_| AppError::storage())?;
        self.call(move |state| state.prepare_found(&versions)).await
    }
    /// Plans `version` in place of the version plan `plan` holds of that skill, under a new plan
    /// id that keeps the first plan's library revision and expiry.
    pub async fn prepare_import_version(
        &self,
        plan: PlanId,
        version: CandidateId,
    ) -> AppResult<ImportPlan> {
        self.call(move |state| state.prepare_version(&plan, version))
            .await
    }
    /// Reads a bounded contained skill preview.
    pub async fn preview_skill(
        &self,
        id: SkillId,
        path: Option<String>,
    ) -> AppResult<SkillPreview> {
        self.call(move |state| state.preview_skill(id, path)).await
    }
    /// Captures preset deletion consequences and the required revision.
    pub async fn preview_delete(&self, id: PresetId) -> AppResult<DeletePreview> {
        self.call(move |state| state.preview_delete(id)).await
    }
    /// Validates and fetches a remote before saving its configuration.
    pub async fn connect_remote(&self, remote: RemoteConfig) -> AppResult<SyncSnapshot> {
        self.call(move |state| {
            state.require_recovered()?;
            let git = state
                .git
                .as_ref()
                .ok_or_else(|| AppError::validation("Git is unavailable."))?;
            let snapshot = git.connect(&state.library_dir(), &remote)?;
            state.set_setting("remote", &Some(remote))?;
            state.set_setting("sync", &snapshot)?;
            Ok(snapshot)
        })
        .await
    }
    /// Performs one explicitly requested Git operation.
    pub async fn synchronize(&self, action: SyncAction) -> AppResult<SyncSnapshot> {
        self.call(move |state| state.synchronize(action)).await
    }
    /// Saves validated machine-local appearance preferences.
    pub async fn save_preferences(&self, mut preferences: Preferences) -> AppResult<()> {
        if !preferences.sidebar_width.is_finite() {
            return Err(AppError::validation("Sidebar width must be finite."));
        }
        preferences.sidebar_width = preferences.sidebar_width.clamp(192.0, 400.0);
        self.call(move |state| state.set_setting("preferences", &preferences))
            .await
    }
}
impl State {
    /// The digest and slug of each library skill, which put found skills in library. A blocked
    /// library skill marks nothing.
    pub(crate) fn library_index(&self) -> AppResult<LibraryIndex> {
        let (library, details) =
            crate::library_store::load(&mut self.database.borrow_mut(), &self.log)?;
        let mut index = LibraryIndex::default();
        for skill in library.skills {
            if details.get(&skill.id).map(|details| details.validation)
                == Some(ValidationStatus::Blocked)
            {
                continue;
            }
            let normalized = crate::library::normalized(skill.id.as_str());
            index.slugs.insert(normalized, skill.id.clone());
            index.digests.insert(skill.digest, skill.id);
        }
        Ok(index)
    }
    fn snapshot(&mut self) -> AppResult<AppSnapshot> {
        let bootstrap = self.bootstrap()?;
        let library = if bootstrap.onboarding.finished {
            Some(self.library_snapshot()?)
        } else {
            None
        };
        let mut sync: SyncSnapshot = self.setting("sync")?.unwrap_or_default();
        if library.is_some()
            && let Some(git) = &self.git
            && self.library_dir().join(".git").exists()
        {
            let remote: Option<RemoteConfig> = self.setting("remote")?.flatten();
            let mut current = git.status(&self.library_dir(), remote.as_ref(), false)?;
            current.refreshed_at = sync.refreshed_at;
            sync = current;
        }
        let (scan, scanning) = self.scans.current()?;
        let updates = match &library {
            Some(library) => self.updates.current(&library.library.skills)?,
            None => BTreeMap::new(),
        };
        Ok(AppSnapshot {
            checking: self.updates.running()?,
            generation: 0,
            background_error: None,
            bootstrap,
            library,
            home: self.config.home.display().to_string(),
            agents: self.agents()?,
            workspaces: self.workspaces()?,
            scan,
            scanning,
            updates,
            explore: catalog::explore(
                &self.catalog,
                &self.installs,
                &self.previews,
                &self.searches,
                &self.resolutions,
            ),
            sync,
            // A snapshot is a worker job, so no tracked change is in flight while it runs.
            activation: None,
            preferences: self.setting("preferences")?.unwrap_or_default(),
        })
    }
    fn synchronize(&mut self, action: SyncAction) -> AppResult<SyncSnapshot> {
        self.require_recovered()?;
        let remote: Option<RemoteConfig> = self.setting("remote")?.flatten();
        let git = self
            .git
            .as_ref()
            .ok_or_else(|| AppError::validation("Git is unavailable."))?;
        let repo = self.library_dir();
        let current = matches!(action, SyncAction::Push)
            && git.revision(&repo, "HEAD").ok()
                == Some(self.setting::<Option<String>>(RECONCILED_HEAD)?.flatten());
        // Pending records are committed only when nothing was pulled: written after a pull, they
        // would replace pulled records with this device's older ones before reconciling.
        let result = git.perform_with(&repo, remote.as_ref(), action, &|| {
            self.write_pending_records()
        });
        // A Push only commits what the database already holds. A Pull or Sync, including one
        // that fast-forwarded before its push failed, reconciles what moved.
        let caught_up = match action {
            SyncAction::Push if current => git
                .revision(&repo, "HEAD")
                .and_then(|head| self.set_setting(RECONCILED_HEAD, &head)),
            SyncAction::Pull | SyncAction::Sync => {
                let reconciled = self.reconcile_head();
                // Pulled content rewrites library copies, even after a partial reconcile.
                self.apply_library();
                // One that fills an empty library adopts the skills your agents already have.
                self.adopt_agent_copies();
                reconciled
            }
            _ => Ok(()),
        };
        // The recorded commit stays behind on failure, so the next launch retries; the Git
        // result, success or failure, is what the user sees.
        if let Err(error) = caught_up {
            self.log
                .record(format!("Library reconciliation will retry: {error}"));
        }
        let snapshot = result?;
        if matches!(action, SyncAction::Disconnect) {
            self.set_setting("remote", &Option::<RemoteConfig>::None)?;
        }
        self.set_setting("sync", &snapshot)?;
        Ok(snapshot)
    }
}
/// Explore's projection, from the shared catalog, installs, and previews.
#[cfg(test)]
mod test_state;
#[cfg(all(test, unix))]
mod tests;
