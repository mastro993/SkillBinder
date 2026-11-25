use crate::{
    activation::Progress,
    app_update::AppUpdates,
    catalog::{Catalogs, Previews, Repositories, Resolutions, Searches},
    command_line::CommandLine,
    database,
    git::{Git, Installer},
    git_pool::GitPool,
    grants::GrantedDirectory,
    install::Installs,
    lifecycle::EngineConfig,
    logging::{Area, Level, Logger},
    mcp_clients::{McpClients, Places},
    scan::{LibraryIndex, Scans},
    shutdown::Shutdown,
    sources::Updates,
    storage,
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
mod app_update;
mod catalog;
mod command_line;
mod install;
mod mcp_clients;
mod onboarding;
mod preview;
mod repositories;
mod skills_sh;
mod snapshot;
mod sources;
mod sync;

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
    repositories: Repositories,
    /// Explore's skills.sh search, on a thread of its own.
    searches: Searches,
    /// The folders of skills.sh results, resolved on the pool.
    resolutions: Resolutions,
    /// Runs installs' network phases, previews, resolutions, and listings off the worker.
    pool: GitPool,
    progress: Progress,
    /// App update checks, on a thread of their own.
    app_updates: AppUpdates,
    /// The `skillbinder` command on the PATH, read and changed on threads of their own.
    command_line: CommandLine,
    /// The agents Settings connects to the MCP server, checked on threads of their own.
    mcp_clients: McpClients,
    /// Stops in-flight commands when the application quits.
    shutdown: Shutdown,
    /// Where an action that spans more than one call logs its error, once.
    log: Logger,
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
    /// Your repositories as saved, whose catalog caches are kept from pruning.
    pub repositories: Repositories,
    /// Explore's skills.sh search, which snapshots read.
    pub searches: Searches,
    /// The folders of skills.sh results, which snapshots read.
    pub resolutions: Resolutions,
    /// How far a tracked change has got; passes count their pairs into it.
    pub progress: Progress,
    /// App updates, which snapshots read.
    pub app_updates: AppUpdates,
    /// The `skillbinder` command, which onboarding's command step offers.
    pub command_line: CommandLine,
    /// The agents' MCP settings, which onboarding's agents step offers to connect.
    pub mcp_clients: McpClients,
    pub grants: BTreeMap<GrantId, GrantedDirectory>,
    pub recovery_required: bool,
    /// Stops the work a quit would otherwise wait for.
    pub shutdown: Shutdown,
    lock: File,
}
impl Engine {
    /// Opens fresh storage, obtains the exclusive library writer lock, and recovers journals,
    /// logging to `log`, which [`Logger::open_session`] started for `config`. Call before
    /// creating native windows or from a blocking task.
    pub fn open(mut config: EngineConfig, log: Logger) -> AppResult<Self> {
        fs::create_dir_all(&config.data_dir)
            .map_err(storage::error_at("creating", &config.data_dir))?;
        config.data_dir = dunce::canonicalize(&config.data_dir)
            .map_err(storage::error_at("resolving", &config.data_dir))?;
        fs::create_dir_all(&config.home).map_err(storage::error_at("creating", &config.home))?;
        config.home = dunce::canonicalize(&config.home)
            .map_err(storage::error_at("resolving", &config.home))?;
        for name in ["staging", "icons"] {
            let directory = config.data_dir.join(name);
            fs::create_dir_all(&directory).map_err(storage::error_at("creating", &directory))?;
        }
        let lock_path = config.data_dir.join("library.lock");
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(storage::error_at("opening", &lock_path))?;
        FileExt::try_lock_exclusive(&lock).map_err(|error| {
            AppError::new(
                ErrorCategory::Busy,
                "Another SkillBinder process owns this library.",
                "Activate the running window.",
            )
            .because(format_args!("locking {}: {error}", lock_path.display()))
        })?;
        let database = RefCell::new(database::open(&config.data_dir.join(DATABASE))?);
        let scans = Scans::default();
        let updates = Updates::default();
        let progress = Progress::default();
        let shutdown = Shutdown::default();
        let catalog = Catalogs::open(config.catalog.clone(), &config.data_dir, shutdown.clone());
        let installs = Installs::default();
        let previews = Previews::default();
        let repositories = Repositories::default();
        let searches = Searches::new(config.catalog.clone());
        let resolutions = Resolutions::default();
        let pool = GitPool::default();
        let app_updates = AppUpdates::new(
            config.app_update.clone(),
            config.app_version,
            config.placement.clone(),
            config.data_dir.join("updates"),
            log.clone(),
        );
        let command_line = CommandLine::platform(&config, log.clone());
        command_line.refresh_in_background();
        let mcp_clients = McpClients::new(
            Places {
                home: config.home.clone(),
                environment: config.environment.clone(),
            },
            shutdown.clone(),
        );
        let (git, outdated_git) = config.find_git(shutdown.clone());
        log.git(git.as_ref(), outdated_git.as_deref());
        let mut state = State {
            config,
            database,
            git,
            outdated_git,
            installer: Installer::detect(),
            log: log.clone(),
            scans: scans.clone(),
            updates: updates.clone(),
            catalog: catalog.clone(),
            installs: installs.clone(),
            previews: previews.clone(),
            repositories: repositories.clone(),
            searches: searches.clone(),
            resolutions: resolutions.clone(),
            progress: progress.clone(),
            app_updates: app_updates.clone(),
            command_line: command_line.clone(),
            mcp_clients: mcp_clients.clone(),
            grants: BTreeMap::new(),
            recovery_required: true,
            shutdown: shutdown.clone(),
            lock,
        };
        state.recover()?;
        state.restore_app_updates()?;
        state.publish_repositories()?;
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
                        .event(Level::Warn, Area::Library, "library creation will retry")
                        .error(&error);
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
                            state.log.stopped();
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
                FileExt::unlock(&state.lock).map_err(storage::error("unlocking the library"))
            })
            .map_err(storage::error("starting the worker thread"))?;
        Ok(Self {
            owner: Arc::new(Owner {
                sender,
                thread: Mutex::new(Some(thread)),
                scans,
                updates,
                catalog,
                installs,
                previews,
                repositories,
                searches,
                resolutions,
                pool,
                progress,
                app_updates,
                command_line,
                mcp_clients,
                shutdown,
                log,
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
                if let Err(error) = &result {
                    state.log.failure(
                        Level::Error,
                        Area::App,
                        ("request failed", "request stopped at quit"),
                        error,
                    );
                }
                let _ = sender.send(result);
            })))
            .map_err(|error| {
                AppError::new(
                    ErrorCategory::Cancelled,
                    "SkillBinder is shutting down.",
                    "Restart the application.",
                )
                .because(format_args!("sending to the worker: {error}"))
            })?;
        receiver
            .await
            .map_err(storage::error("waiting for the worker"))?
    }
    /// Logs `result`'s error once, where a public method whose work runs off the worker hands it
    /// to the user (ADR 0032), as `call` does for work on the worker: under `failed`, or as
    /// `DEBUG` under `stopped` when the app is quitting.
    fn reported<T>(
        &self,
        area: Area,
        messages: (&'static str, &'static str),
        result: AppResult<T>,
    ) -> AppResult<T> {
        if let Err(error) = &result {
            self.owner.log.failure(Level::Error, area, messages, error);
        }
        result
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
            .map_err(storage::error("asking the worker to shut down"))?;
        receiver
            .await
            .map_err(storage::error("waiting for the worker to shut down"))?;
        if let Some(thread) = self
            .owner
            .thread
            .lock()
            .map_err(storage::error("taking the worker thread"))?
            .take()
        {
            thread.join().map_err(|_| {
                AppError::storage().because("joining the worker thread: it panicked")
            })??;
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
            .map_err(storage::error("waiting for the scan"))?;
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
}
#[cfg(test)]
mod test_state;
#[cfg(all(test, unix))]
mod tests;
