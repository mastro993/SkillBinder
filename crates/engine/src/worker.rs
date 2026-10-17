use crate::{
    database,
    discovery::{Managed, Scans},
    git::Git,
    grants::GrantedDirectory,
    lifecycle::EngineConfig,
    logging::Logger,
    reconcile::RECONCILED_HEAD,
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
    pub log: Logger,
    pub scans: Scans,
    pub updates: Updates,
    pub grants: BTreeMap<GrantId, GrantedDirectory>,
    pub recovery_required: bool,
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
        for name in ["staging", "logs"] {
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
        let mut state = State {
            config,
            database,
            git: Git::discover().ok(),
            log,
            scans: scans.clone(),
            updates: updates.clone(),
            grants: BTreeMap::new(),
            recovery_required: true,
            lock,
        };
        state.recover()?;
        state.forget_agent_folders()?;
        let (sender, receiver) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("skillbinder-engine".into())
            .spawn(move || {
                // Engine::open returns before this runs; queued requests wait for it.
                if state.library_dir().exists() {
                    state.catch_up();
                }
                for message in receiver {
                    match message {
                        Message::Execute(job) => job(&mut state),
                        Message::Shutdown(done) => {
                            state.scans.wait_for_shutdown();
                            state.log.flush();
                            let _ = done.send(());
                            break;
                        }
                    }
                }
                state.scans.wait_for_shutdown();
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
    /// Cancels scanning, drains committed mutations, flushes logs, and joins the worker.
    pub async fn shutdown(&self) -> AppResult<()> {
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
    /// Rechecks prerequisites and persisted onboarding progress.
    pub async fn bootstrap(&self) -> AppResult<Bootstrap> {
        self.call(|state| {
            state.git = Git::discover().ok();
            state.bootstrap()
        })
        .await
    }
    /// Persists one legal onboarding transition.
    pub async fn set_onboarding(&self, step: OnboardingStep) -> AppResult<Bootstrap> {
        self.call(move |state| state.set_onboarding(step)).await
    }
    /// Atomically creates the local library at the final onboarding step.
    pub async fn complete_onboarding(&self) -> AppResult<Bootstrap> {
        self.call(State::complete_onboarding).await
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
        let (config, workspaces, managed, log) = self
            .call(|state| {
                Ok((
                    state.config.clone(),
                    state
                        .workspaces()?
                        .into_iter()
                        .filter(|workspace| !workspace.missing)
                        .map(|workspace| (workspace.id, PathBuf::from(workspace.path)))
                        .collect(),
                    state.managed()?,
                    state.log.clone(),
                ))
            })
            .await?;
        self.owner
            .scans
            .start(config, workspaces, managed, log, replace)
    }
    /// The last completed scan and whether another one is running, without waiting for a
    /// durable mutation.
    pub fn current_scan(&self) -> AppResult<(Option<ScanSnapshot>, bool)> {
        self.owner.scans.current()
    }
    /// Checks the sources of the library skills not checked for their current content within
    /// `max_age`, on a thread of its own, and waits for the results. Does nothing while another
    /// check runs.
    pub async fn check_updates(&self, max_age: std::time::Duration) -> AppResult<()> {
        let Some((git, data_dir, due, log)) = self
            .call(move |state| {
                let (Some(git), OnboardingStep::Complete) =
                    (state.git.clone(), state.bootstrap()?.step)
                else {
                    return Ok(None);
                };
                let skills = state.read_library()?.skills;
                Ok(state
                    .updates
                    .begin(&skills, max_age.as_secs())?
                    .map(|due| (git, state.config.data_dir.clone(), due, state.log.clone())))
            })
            .await?
        else {
            return Ok(());
        };
        let updates = self.owner.updates.clone();
        let (done, finished) = oneshot::channel();
        let spawned = std::thread::Builder::new()
            .name("skillbinder-updates".into())
            .spawn({
                let updates = updates.clone();
                move || {
                    // A check that panics still ends, so later checks can start.
                    let results = std::panic::catch_unwind(|| {
                        crate::sources::check(&git, &data_dir, due, &log)
                    });
                    updates.finish(results.unwrap_or_default());
                    let _ = done.send(());
                }
            });
        if spawned.is_err() {
            updates.finish(Vec::new());
            return Err(AppError::storage());
        }
        finished.await.map_err(|_| AppError::storage())
    }
    /// Replaces each selected library skill's content with its source's current content.
    pub async fn update_skills(&self, ids: Vec<SkillId>) -> AppResult<UpdateOutcome> {
        self.call(move |state| state.update_skills(ids)).await
    }
    /// Re-inspects selected opaque candidates into an immutable review plan.
    pub async fn prepare_import(
        &self,
        ids: Vec<CandidateId>,
        acknowledge_invalid: bool,
    ) -> AppResult<ImportPlan> {
        self.call(move |state| state.prepare_import(ids, acknowledge_invalid))
            .await
    }
    /// Applies a reviewed plan, or returns its durable previous outcome.
    pub async fn apply_import(&self, plan: PlanId) -> AppResult<ImportOutcome> {
        self.call(move |state| state.apply_import(plan)).await
    }
    /// Dismisses the retained terminal outcome.
    pub async fn dismiss_import(&self) -> AppResult<()> {
        self.call(|state| state.set_setting("import_outcome", &Option::<ImportOutcome>::None))
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
    /// Applies a validated, journaled organization change.
    pub async fn change_organization(
        &self,
        change: OrganizationChange,
    ) -> AppResult<LibrarySnapshot> {
        self.call(move |state| state.change_organization(change))
            .await
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
    /// The digest and slug of each library skill, which mark found skills as installed. Blocked
    /// payloads mark nothing by content.
    fn managed(&self) -> AppResult<Managed> {
        let (library, details) = crate::library_store::load(&mut self.database.borrow_mut())?;
        let mut managed = Managed::default();
        for skill in library.skills {
            let normalized = crate::library::normalized(skill.id.as_str());
            managed.slugs.insert(normalized, skill.id.clone());
            if details.get(&skill.id).map(|details| details.validation)
                != Some(ValidationStatus::Blocked)
            {
                managed.digests.insert(skill.digest, skill.id);
            }
        }
        Ok(managed)
    }
    fn snapshot(&mut self) -> AppResult<AppSnapshot> {
        let bootstrap = self.bootstrap()?;
        let library = if bootstrap.step == OnboardingStep::Complete {
            Some(self.library_snapshot()?)
        } else {
            None
        };
        let mut sync: SyncSnapshot = self.setting("sync")?.unwrap_or_default();
        if library.is_some()
            && let Some(git) = &self.git
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
            generation: 0,
            background_error: None,
            bootstrap,
            library,
            home: self.config.home.display().to_string(),
            agents: crate::discovery::registry::agent_options(&self.config)?,
            workspaces: self.workspaces()?,
            scan,
            scanning,
            updates,
            sync,
            import_outcome: self.setting("import_outcome")?.flatten(),
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
        let result = git.perform(&repo, remote.as_ref(), action);
        // A Push only commits what the database already holds. A Pull or Sync, including one
        // that fast-forwarded before its push failed, reconciles what moved.
        let caught_up = match action {
            SyncAction::Push if current => git
                .revision(&repo, "HEAD")
                .and_then(|head| self.set_setting(RECONCILED_HEAD, &head)),
            SyncAction::Pull | SyncAction::Sync => self.reconcile_head(),
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
#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "Test fixture failures must abort the test."
)]
impl State {
    pub(crate) fn isolated_test_state(data_dir: PathBuf) -> Self {
        let home = data_dir.join("home");
        for path in [
            home.clone(),
            data_dir.join("staging"),
            data_dir.join("library/skills"),
            data_dir.join("logs"),
        ] {
            fs::create_dir_all(path).unwrap();
        }
        Self {
            config: EngineConfig::isolated(data_dir.clone(), home.clone()),
            database: RefCell::new(database::open(&data_dir.join(DATABASE)).unwrap()),
            git: None,
            log: Logger::start(data_dir.join("logs"), &home).unwrap(),
            scans: Scans::default(),
            updates: Updates::default(),
            grants: BTreeMap::new(),
            recovery_required: false,
            lock: File::create(data_dir.join("test.lock")).unwrap(),
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[tokio::test]
    #[expect(
        clippy::unwrap_used,
        reason = "Fixture failures must abort the regression test."
    )]
    async fn shutdown_releases_lock_while_a_child_retains_the_descriptor() {
        let temp = tempfile::tempdir().unwrap();
        let config = EngineConfig::isolated(temp.path().join("data"), temp.path().join("home"));
        let engine = Engine::open(config.clone()).unwrap();
        let inherited_descriptor = engine
            .call(|state| state.lock.try_clone().map_err(|_| AppError::storage()))
            .await
            .unwrap();
        let mut child = std::process::Command::new("/bin/cat")
            .stdin(std::process::Stdio::piped())
            .stdout(inherited_descriptor)
            .spawn()
            .unwrap();
        assert!(
            matches!(Engine::open(config.clone()), Err(error) if error.category == ErrorCategory::Busy)
        );

        engine.shutdown().await.unwrap();
        assert!(child.try_wait().unwrap().is_none());
        let reopened = Engine::open(config);
        drop(child.stdin.take());
        assert!(child.wait().unwrap().success());
        reopened.unwrap().shutdown().await.unwrap();
    }
}
