//! Typed operations with immutable projections, coalesced subscriptions, and stale-read rejection.

use skillbinder_engine::Engine;
use skillbinder_proto::*;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{runtime::Handle, sync::watch};

/// How long a source check holds before the next refresh checks the skill again.
const UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);

struct Shared {
    engine: Engine,
    runtime: Handle,
    snapshots: watch::Sender<Arc<AppSnapshot>>,
    requested: AtomicU64,
    /// Completed scans whose found copies sources were already learned from and checked after.
    checked_scans: AtomicU64,
}
/// Cloneable in-process client. Navigation never owns or cancels engine mutations.
#[derive(Clone)]
pub struct Client {
    shared: Arc<Shared>,
}
impl Client {
    /// Creates the first projection, starts scan progress coordination, and refreshes the app.
    pub async fn new(engine: Engine, runtime: Handle) -> AppResult<Self> {
        let initial = engine.snapshot().await?;
        let (snapshots, _) = watch::channel(Arc::new(initial));
        let client = Self {
            shared: Arc::new(Shared {
                engine,
                runtime,
                snapshots,
                requested: AtomicU64::new(0),
                checked_scans: AtomicU64::new(0),
            }),
        };
        client.start_refresh();
        client.refresh_app();
        Ok(client)
    }
    /// Returns the latest complete projection without blocking on storage.
    pub fn snapshot(&self) -> Arc<AppSnapshot> {
        self.shared.snapshots.borrow().clone()
    }
    /// Subscribes to coalesced projections. Slow consumers receive the latest value.
    pub fn subscribe(&self) -> watch::Receiver<Arc<AppSnapshot>> {
        self.shared.snapshots.subscribe()
    }
    /// Returns the executable-owned runtime for native view tasks.
    pub fn runtime(&self) -> Handle {
        self.shared.runtime.clone()
    }
    /// Refreshes the projection and rejects reads superseded by a newer request.
    pub async fn refresh(&self) -> AppResult<()> {
        let generation = self.shared.requested.fetch_add(1, Ordering::AcqRel) + 1;
        let snapshot = self.shared.engine.snapshot().await?;
        self.shared.publish_full(generation, snapshot)
    }
    fn start_refresh(&self) {
        let weak = Arc::downgrade(&self.shared);
        self.shared.runtime.spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(250));
            loop {
                interval.tick().await;
                let Some(shared) = weak.upgrade() else {
                    break;
                };
                let client = Self { shared };
                if let Err(error) = client.shared.publish_progress() {
                    client.background_error(error);
                }
                client.check_after_scan();
            }
        });
    }
    /// Once a scan completes, learns sources from its found copies and checks for updates.
    fn check_after_scan(&self) {
        let Ok(completed) = self.shared.engine.completed_scans() else {
            return;
        };
        let previous = self.shared.checked_scans.swap(completed, Ordering::AcqRel);
        if previous != completed {
            let client = self.clone();
            self.shared.runtime.spawn(async move {
                if !client.check_updates().await {
                    // Another check was running and saw the library before this scan; marking
                    // the scan unchecked again retries it at the next poll.
                    let _ = client.shared.checked_scans.compare_exchange(
                        completed,
                        previous,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    );
                }
            });
        }
    }
    fn background_error(&self, error: AppError) {
        self.shared.snapshots.send_modify(|current| {
            let mut next = (**current).clone();
            next.background_error = Some(error);
            next.generation = current.generation + 1;
            *current = Arc::new(next);
        });
    }
    async fn refresh_preserving_outcome<T>(&self, result: AppResult<T>) -> AppResult<T> {
        let refreshed = self.refresh().await;
        match result {
            Ok(value) => {
                if let Err(error) = refreshed {
                    self.background_error(error);
                }
                Ok(value)
            }
            Err(error) => Err(error),
        }
    }
    /// Rechecks prerequisites.
    pub async fn bootstrap(&self) -> AppResult<Bootstrap> {
        self.refresh_preserving_outcome(self.shared.engine.bootstrap().await)
            .await
    }
    /// Advances or revisits an allowed onboarding step.
    pub async fn set_onboarding(&self, step: OnboardingStep) -> AppResult<Bootstrap> {
        self.refresh_preserving_outcome(self.shared.engine.set_onboarding(step).await)
            .await
    }
    /// Creates the local library atomically.
    pub async fn complete_onboarding(&self) -> AppResult<Bootstrap> {
        self.refresh_preserving_outcome(self.shared.engine.complete_onboarding().await)
            .await
    }
    /// Issues a capability from native directory selection.
    pub async fn grant_directory(&self, path: PathBuf) -> AppResult<DirectoryGrant> {
        self.shared.engine.grant_directory(path).await
    }
    /// Saves a granted folder's projects as workspaces, then scans again so they are covered.
    pub async fn add_workspaces(&self, grant: GrantId) -> AppResult<Vec<Workspace>> {
        let added = self
            .refresh_preserving_outcome(self.shared.engine.add_workspaces(grant).await)
            .await?;
        self.spawn_scan();
        Ok(added)
    }
    /// Renames a workspace.
    pub async fn rename_workspace(&self, id: WorkspaceId, name: String) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.rename_workspace(id, name).await)
            .await
    }
    /// Sets or clears a workspace glyph's color.
    pub async fn set_workspace_color(
        &self,
        id: WorkspaceId,
        color: Option<PresetColor>,
    ) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.set_workspace_color(id, color).await)
            .await
    }
    /// Sets a workspace glyph, or restores the default folder glyph, and removes its image icon.
    pub async fn set_workspace_icon(
        &self,
        id: WorkspaceId,
        icon: Option<PresetIcon>,
    ) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.set_workspace_icon(id, icon).await)
            .await
    }
    /// Copies a picked image into application storage and makes it the workspace icon.
    pub async fn set_workspace_image(&self, id: WorkspaceId, image: PathBuf) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.set_workspace_image(id, image).await)
            .await
    }
    /// Saves a new workspace order.
    pub async fn reorder_workspaces(&self, order: Vec<WorkspaceId>) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.reorder_workspaces(order).await)
            .await
    }
    /// Forgets a workspace, then scans again so its skills are no longer listed.
    pub async fn remove_workspace(&self, id: WorkspaceId) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.remove_workspace(id).await)
            .await?;
        self.spawn_scan();
        Ok(())
    }
    /// Scans in the background; failures surface as the background error.
    pub fn spawn_scan(&self) {
        let client = self.clone();
        self.shared.runtime.spawn(async move {
            if let Err(error) = client.start_scan().await {
                client.background_error(error);
            }
        });
    }
    /// Refreshes the app in the background, on launch and when the window regains focus: scans
    /// the agents' global skill directories and every workspace, then refreshes the remote's Git status.
    /// The scan's completion checks the sources not checked within the hour.
    /// A running scan is kept rather than replaced, so frequent focus changes cannot keep a slow
    /// scan from completing.
    pub fn refresh_app(&self) {
        let snapshot = self.snapshot();
        if snapshot.library.is_none() {
            return;
        }
        let remote = snapshot.sync.remote.is_some();
        let client = self.clone();
        self.shared.runtime.spawn(async move {
            let scanned = client.shared.engine.refresh_scan().await;
            if let Err(error) = client.refresh_preserving_outcome(scanned).await {
                client.background_error(error);
            }
            if remote && let Err(error) = client.synchronize(SyncAction::Refresh).await {
                client.background_error(error);
            }
        });
    }
    /// Checks the sources due for a check, then publishes their update states. `false` only
    /// when another check was running, which leaves nothing new to publish; a library with no
    /// source due counts as checked.
    async fn check_updates(&self) -> bool {
        let checked = self
            .shared
            .engine
            .check_updates(UPDATE_CHECK_INTERVAL)
            .await;
        if matches!(checked, Ok(false)) {
            return false;
        }
        match self.refresh_preserving_outcome(checked).await {
            Ok(ran) => ran,
            Err(error) => {
                self.background_error(error);
                true
            }
        }
    }
    /// Checks every source now, whatever the interval since its last check, and publishes the
    /// results. A check already running leaves this with nothing to publish.
    pub async fn check_now(&self) -> AppResult<()> {
        let checked = self
            .shared
            .engine
            .check_updates(std::time::Duration::ZERO)
            .await;
        self.refresh_preserving_outcome(checked).await.map(|_| ())
    }

    /// Updates one library skill from its source, without the rescan a whole batch ends with, so a
    /// caller updating several skills can report each one as it lands.
    pub async fn update_skill(&self, id: SkillId) -> AppResult<UpdateOutcome> {
        self.refresh_preserving_outcome(self.shared.engine.update_skills(vec![id]).await)
            .await
    }

    /// Replaces the selected skills' content with their sources' current content, then scans
    /// again, since the agents' copies no longer match the updated skills.
    pub async fn update_skills(&self, ids: Vec<SkillId>) -> AppResult<UpdateOutcome> {
        let outcome = self
            .refresh_preserving_outcome(self.shared.engine.update_skills(ids).await)
            .await?;
        self.spawn_scan();
        Ok(outcome)
    }
    /// Restores a drifted skill's recorded content, then scans again, since the agents' copies
    /// may match it again.
    pub async fn reinstall_skill(&self, id: SkillId) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.reinstall_skill(id).await)
            .await?;
        self.spawn_scan();
        Ok(())
    }
    /// Scans the agents' global skill directories and every workspace, replacing a running scan.
    pub async fn start_scan(&self) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.start_scan().await)
            .await
    }
    /// Prepares selected candidate identities for review.
    pub async fn prepare_import(
        &self,
        ids: Vec<CandidateId>,
        acknowledge_invalid: bool,
    ) -> AppResult<ImportPlan> {
        self.shared
            .engine
            .prepare_import(ids, acknowledge_invalid)
            .await
    }
    /// Applies a plan with durable replay and outcome retention, then scans again so the
    /// imported skills are found in the library; that scan's completion checks their sources.
    pub async fn apply_import(&self, plan: PlanId) -> AppResult<ImportOutcome> {
        let outcome = self
            .refresh_preserving_outcome(self.shared.engine.apply_import(plan).await)
            .await?;
        self.spawn_scan();
        Ok(outcome)
    }
    /// Dismisses the last retained import result.
    pub async fn dismiss_import(&self) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.dismiss_import().await)
            .await
    }
    /// Reads a bounded skill preview.
    pub async fn preview_skill(
        &self,
        id: SkillId,
        path: Option<String>,
    ) -> AppResult<SkillPreview> {
        self.shared.engine.preview_skill(id, path).await
    }
    /// Reads revision-checked deletion consequences.
    pub async fn preview_delete(&self, id: PresetId) -> AppResult<DeletePreview> {
        self.shared.engine.preview_delete(id).await
    }
    /// Applies preset or membership edits.
    pub async fn change_organization(
        &self,
        change: OrganizationChange,
    ) -> AppResult<LibrarySnapshot> {
        self.refresh_preserving_outcome(self.shared.engine.change_organization(change).await)
            .await
    }
    /// Connects and fetches a remote without implicit commits.
    pub async fn connect_remote(&self, remote: RemoteConfig) -> AppResult<SyncSnapshot> {
        self.refresh_preserving_outcome(self.shared.engine.connect_remote(remote).await)
            .await
    }
    /// Performs explicit synchronization or remote refresh.
    pub async fn synchronize(&self, action: SyncAction) -> AppResult<SyncSnapshot> {
        self.refresh_preserving_outcome(self.shared.engine.synchronize(action).await)
            .await
    }
    /// Stores bounded machine-local preferences.
    pub async fn save_preferences(&self, preferences: Preferences) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.save_preferences(preferences).await)
            .await
    }
}

impl Shared {
    fn publish_full(&self, generation: u64, mut snapshot: AppSnapshot) -> AppResult<()> {
        let mut result = Ok(());
        self.snapshots.send_if_modified(|current| {
            if generation != self.requested.load(Ordering::Acquire) {
                return false;
            }
            match self.engine.current_scan() {
                Ok((scan, scanning)) => {
                    snapshot.scan = scan;
                    snapshot.scanning = scanning;
                }
                Err(error) => {
                    result = Err(error);
                    return false;
                }
            }
            snapshot.generation = current.generation + 1;
            *current = Arc::new(snapshot.clone());
            true
        });
        result
    }
    /// Publishes scan progress without waiting on the worker.
    fn publish_progress(&self) -> AppResult<()> {
        let mut result = Ok(());
        self.snapshots.send_if_modified(|current| {
            let (scan, scanning) = match self.engine.current_scan() {
                Ok(scan) => scan,
                Err(error) => {
                    result = Err(error);
                    return false;
                }
            };
            if current.scan == scan && current.scanning == scanning {
                return false;
            }
            let mut next = (**current).clone();
            next.scan = scan;
            next.scanning = scanning;
            next.generation = current.generation + 1;
            *current = Arc::new(next);
            true
        });
        result
    }
}

#[cfg(test)]
mod tests;
