use crate::{
    discovery::Scans, git::Git, lifecycle::EngineConfig, logging::Logger, persistence,
    roots::GrantedDirectory,
};
use fs2::FileExt;
use skillbinder_proto::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread::JoinHandle,
};
use tokio::sync::oneshot;

type Job = Box<dyn FnOnce(&mut State) + Send>;
enum Message {
    Execute(Job),
    Shutdown(oneshot::Sender<()>),
}
struct Owner {
    sender: mpsc::Sender<Message>,
    thread: Mutex<Option<JoinHandle<AppResult<()>>>>,
    scans: Scans,
}
/// Cloneable typed handle to the embedded engine worker.
#[derive(Clone)]
pub struct Engine {
    owner: Arc<Owner>,
}
pub(crate) struct State {
    pub config: EngineConfig,
    pub database: rusqlite::Connection,
    pub git: Option<Git>,
    pub log: Logger,
    pub scans: Scans,
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
        for name in ["staging", "journals", "backups", "logs"] {
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
        let database = persistence::open(&config.data_dir.join("state.sqlite3"))?;
        let scans = Scans::default();
        let mut state = State {
            config,
            database,
            git: Git::discover().ok(),
            log,
            scans: scans.clone(),
            grants: BTreeMap::new(),
            recovery_required: true,
            lock,
        };
        state.recover()?;
        if state.library_dir().join(".git").exists()
            && let Some(git) = &state.git
        {
            git.recover(&state.library_dir())?;
        }
        let (sender, receiver) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("skillbinder-engine".into())
            .spawn(move || {
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
            }),
        })
    }
    async fn call<T: Send + 'static>(
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
    /// Cancels discovery, drains committed mutations, flushes logs, and joins the worker.
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
    /// Consumes a directory capability to register a project boundary.
    pub async fn register_root(&self, grant: GrantId, label: String) -> AppResult<ProjectRoot> {
        self.call(move |state| state.register_root(grant, label))
            .await
    }
    /// Renames or enables a registered root.
    pub async fn update_root(&self, id: RootId, label: String, enabled: bool) -> AppResult<()> {
        self.call(move |state| state.update_root(id, label, enabled))
            .await
    }
    /// Removes a project boundary without touching source files.
    pub async fn remove_root(&self, id: RootId) -> AppResult<()> {
        self.call(move |state| state.remove_root(id)).await
    }
    /// Starts discovery or returns the existing active run.
    pub async fn start_scan(&self) -> AppResult<ScanSnapshot> {
        if let Some(run) = self
            .owner
            .scans
            .current()?
            .filter(|run| run.status == ScanStatus::Running)
        {
            return Ok(run);
        }
        let (config, roots, managed, log) = self
            .call(|state| {
                let library = state.read_library()?;
                Ok((
                    state.config.clone(),
                    state.roots()?,
                    library
                        .skills
                        .iter()
                        .filter_map(|skill| {
                            let path = state
                                .library_dir()
                                .join(crate::library::payload_relative(&library, skill));
                            match state
                                .contained_skill_path(&path)
                                .and_then(|path| crate::payload::inspect(&path))
                            {
                                Ok(inspection)
                                    if inspection.status != ValidationStatus::Blocked =>
                                {
                                    Some(inspection.manifest.digest)
                                }
                                Ok(_) => None,
                                Err(error) => {
                                    state.log.record(format!(
                                        "Managed discovery inspection {}: {error}",
                                        path.display()
                                    ));
                                    None
                                }
                            }
                        })
                        .collect::<BTreeSet<_>>(),
                    state.log.clone(),
                ))
            })
            .await?;
        self.owner.scans.start(config, roots, managed, log)
    }
    /// Requests cooperative cancellation while retaining partial findings.
    pub async fn cancel_scan(&self) -> AppResult<ScanSnapshot> {
        self.owner.scans.cancel()
    }
    /// Returns retained scan state without waiting for a durable mutation.
    pub fn current_scan(&self) -> AppResult<Option<ScanSnapshot>> {
        self.owner.scans.current()
    }
    /// Re-inspects selected opaque candidates into an immutable review plan.
    pub async fn prepare_import(
        &self,
        scan: ScanId,
        ids: Vec<CandidateId>,
        acknowledge_invalid: bool,
    ) -> AppResult<ImportPlan> {
        self.call(move |state| state.prepare_import(scan, ids, acknowledge_invalid))
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
    /// Captures folder deletion consequences and the required revision.
    pub async fn preview_delete(&self, id: FolderId) -> AppResult<DeletePreview> {
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
    /// Backs up losing copies and keeps the reviewed shared-slug winner.
    pub async fn resolve_conflict(
        &self,
        slug: String,
        keep: SkillId,
        expected: Vec<SkillId>,
        revision: String,
    ) -> AppResult<LibrarySnapshot> {
        self.call(move |state| state.resolve_conflict(slug, keep, expected, revision))
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
    fn snapshot(&mut self) -> AppResult<AppSnapshot> {
        let bootstrap = self.bootstrap()?;
        if bootstrap.step == OnboardingStep::Complete {
            self.refresh_index()?;
        }
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
        Ok(AppSnapshot {
            generation: 0,
            background_error: None,
            bootstrap,
            library,
            roots: self.roots()?,
            scan: self.scans.current()?,
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
        let snapshot = git.perform(&self.library_dir(), remote.as_ref(), action)?;
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
            data_dir.join("journals"),
            data_dir.join("staging"),
            data_dir.join("library/skills"),
            data_dir.join("backups"),
            data_dir.join("logs"),
        ] {
            fs::create_dir_all(path).unwrap();
        }
        Self {
            config: EngineConfig::isolated(data_dir.clone(), home.clone()),
            database: persistence::open(&data_dir.join("state.sqlite3")).unwrap(),
            git: None,
            log: Logger::start(data_dir.join("logs"), &home).unwrap(),
            scans: Scans::default(),
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
