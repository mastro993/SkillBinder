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

struct Shared {
    engine: Engine,
    runtime: Handle,
    snapshots: watch::Sender<Arc<AppSnapshot>>,
    requested: AtomicU64,
}
/// Cloneable in-process client. Navigation never owns or cancels engine mutations.
#[derive(Clone)]
pub struct Client {
    shared: Arc<Shared>,
}
impl Client {
    /// Creates the first projection and starts scan and remote refresh coordination.
    pub async fn new(engine: Engine, runtime: Handle) -> AppResult<Self> {
        let initial = engine.snapshot().await?;
        let (snapshots, _) = watch::channel(Arc::new(initial));
        let client = Self {
            shared: Arc::new(Shared {
                engine,
                runtime,
                snapshots,
                requested: AtomicU64::new(0),
            }),
        };
        client.start_refresh();
        client.rescan_on_launch();
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
                if let Err(error) = shared.publish_progress() {
                    Self { shared }.background_error(error);
                }
            }
        });
        let weak = Arc::downgrade(&self.shared);
        self.shared.runtime.spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                let Some(shared) = weak.upgrade() else {
                    break;
                };
                let client = Self { shared };
                if client.snapshot().sync.remote.is_some()
                    && let Err(error) = client.synchronize(SyncAction::Refresh).await
                {
                    client.background_error(error);
                }
            }
        });
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
    /// Scans a granted folder and saves it as a project; progress is published while it runs.
    pub async fn add_project(&self, grant: GrantId) -> AppResult<Option<ProjectAddition>> {
        self.refresh_preserving_outcome(self.shared.engine.add_project(grant).await)
            .await
    }
    /// Discards the project being added; `false` when its scan had already been saved.
    pub async fn cancel_project_addition(&self) -> AppResult<bool> {
        self.refresh_preserving_outcome(self.shared.engine.cancel_project_addition())
            .await
    }
    /// Clears a finished or failed project addition.
    pub async fn dismiss_project_addition(&self) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.dismiss_project_addition())
            .await
    }
    /// Rescans Global or a project in the background and records its deployments.
    pub async fn rescan_target(&self, target: DeploymentTarget) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.rescan_target(target).await)
            .await
    }
    /// Renames a project.
    pub async fn rename_project(&self, id: ProjectId, name: String) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.rename_project(id, name).await)
            .await
    }
    /// Saves a new project order.
    pub async fn reorder_projects(&self, order: Vec<ProjectId>) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.reorder_projects(order).await)
            .await
    }
    /// Forgets a project and its deployments, then rescans the projects that contain it.
    pub async fn remove_project(&self, id: ProjectId) -> AppResult<()> {
        let containing = self
            .refresh_preserving_outcome(self.shared.engine.remove_project(id).await)
            .await?;
        for id in containing {
            self.spawn_rescan(DeploymentTarget::Project(id));
        }
        Ok(())
    }
    /// Replaces the agents whose skill directories a target writes to.
    pub async fn set_target_agents(
        &self,
        target: DeploymentTarget,
        agents: Vec<String>,
    ) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.set_target_agents(target, agents).await)
            .await
    }
    /// Copies library skills into a target and records the deployments.
    pub async fn add_deployments(
        &self,
        target: DeploymentTarget,
        skills: Vec<SkillId>,
    ) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.add_deployments(target, skills).await)
            .await
    }
    /// Deletes a deployment's copies; a changed copy is kept unless `discard` lists it.
    pub async fn remove_deployment(
        &self,
        target: DeploymentTarget,
        skill: SkillId,
        discard: Vec<String>,
    ) -> AppResult<Removal> {
        self.refresh_preserving_outcome(
            self.shared
                .engine
                .remove_deployment(target, skill, discard)
                .await,
        )
        .await
    }
    /// Resolves a drifted deployment with the remedy the user picked.
    pub async fn resolve_drift(
        &self,
        shown: Deployment,
        expected: Drift,
        remedy: Remedy,
    ) -> AppResult<()> {
        self.refresh_preserving_outcome(
            self.shared
                .engine
                .resolve_drift(shown, expected, remedy)
                .await,
        )
        .await
    }
    /// Rescans in the background; failures surface as the background error.
    pub fn spawn_rescan(&self, target: DeploymentTarget) {
        let client = self.clone();
        self.shared.runtime.spawn(async move {
            if let Err(error) = client.rescan_target(target).await {
                client.background_error(error);
            }
        });
    }
    /// Rescans Global, then each project, one at a time.
    fn rescan_on_launch(&self) {
        let snapshot = self.snapshot();
        if snapshot.library.is_none() {
            return;
        }
        let targets: Vec<_> = std::iter::once(DeploymentTarget::Global)
            .chain(
                snapshot
                    .projects
                    .iter()
                    .map(|project| DeploymentTarget::Project(project.id.clone())),
            )
            .collect();
        let client = self.clone();
        self.shared.runtime.spawn(async move {
            for target in targets {
                if let Err(error) = client.rescan_target(target).await {
                    client.background_error(error);
                }
            }
        });
    }
    /// Starts or resumes one scan worker.
    pub async fn start_scan(&self) -> AppResult<ScanSnapshot> {
        self.refresh_preserving_outcome(self.shared.engine.start_scan().await)
            .await
    }
    /// Requests scan cancellation without losing findings.
    pub async fn cancel_scan(&self) -> AppResult<ScanSnapshot> {
        self.refresh_preserving_outcome(self.shared.engine.cancel_scan().await)
            .await
    }
    /// Prepares selected candidate identities for review.
    pub async fn prepare_import(
        &self,
        scan: ScanId,
        ids: Vec<CandidateId>,
        acknowledge_invalid: bool,
    ) -> AppResult<ImportPlan> {
        self.shared
            .engine
            .prepare_import(scan, ids, acknowledge_invalid)
            .await
    }
    /// Applies a plan with durable replay and outcome retention.
    pub async fn apply_import(&self, plan: PlanId) -> AppResult<ImportOutcome> {
        self.refresh_preserving_outcome(self.shared.engine.apply_import(plan).await)
            .await
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
    pub async fn preview_delete(&self, id: FolderId) -> AppResult<DeletePreview> {
        self.shared.engine.preview_delete(id).await
    }
    /// Applies folder or assignment edits.
    pub async fn change_organization(
        &self,
        change: OrganizationChange,
    ) -> AppResult<LibrarySnapshot> {
        self.refresh_preserving_outcome(self.shared.engine.change_organization(change).await)
            .await
    }
    /// Keeps one reviewed shared-slug copy, backing up the others.
    pub async fn resolve_conflict(
        &self,
        slug: String,
        keep: SkillId,
        expected: Vec<SkillId>,
        revision: String,
    ) -> AppResult<LibrarySnapshot> {
        self.refresh_preserving_outcome(
            self.shared
                .engine
                .resolve_conflict(slug, keep, expected, revision)
                .await,
        )
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
            match (self.engine.current_scan(), self.engine.target_progress()) {
                (Ok(scan), Ok((addition, rescanning))) => {
                    snapshot.scan = scan;
                    snapshot.project_addition = addition;
                    snapshot.rescanning = rescanning;
                }
                (Err(error), _) | (_, Err(error)) => {
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
    /// Publishes Discovery and target scan progress without waiting on the worker.
    fn publish_progress(&self) -> AppResult<()> {
        let mut result = Ok(());
        self.snapshots.send_if_modified(|current| {
            let (scan, (addition, rescanning)) =
                match (self.engine.current_scan(), self.engine.target_progress()) {
                    (Ok(scan), Ok(targets)) => (scan, targets),
                    (Err(error), _) | (_, Err(error)) => {
                        result = Err(error);
                        return false;
                    }
                };
            if current.scan == scan
                && current.project_addition == addition
                && current.rescanning == rescanning
            {
                return false;
            }
            let mut next = (**current).clone();
            next.scan = scan;
            next.project_addition = addition;
            next.rescanning = rescanning;
            next.generation = current.generation + 1;
            *current = Arc::new(next);
            true
        });
        result
    }
}

#[cfg(test)]
mod tests;
