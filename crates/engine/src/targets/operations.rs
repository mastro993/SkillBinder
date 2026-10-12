//! Public engine operations for Global and projects. Scans run on blocking threads; the worker
//! only reads inputs and saves results.

use super::scan::{self, Control};
use crate::worker::Engine;
use skillbinder_proto::*;
use std::sync::atomic::AtomicUsize;

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| AppError::storage())?
}

impl Engine {
    /// Scans a granted folder and saves it as a project with its deployments in one
    /// transaction. An expired grant, a duplicate folder, or another running addition is an
    /// error. Once the scan starts, the returned addition reports success or a safe failure;
    /// `None` means the addition was cancelled and nothing was saved.
    pub async fn add_project(&self, grant: GrantId) -> AppResult<Option<ProjectAddition>> {
        let (pending, scope, config, log) = self
            .call(move |state| {
                let (pending, scope) = state.prepare_project(&grant)?;
                Ok((pending, scope, state.config.clone(), state.log.clone()))
            })
            .await?;
        let addition = self
            .targets()
            .begin_addition(pending.name.clone(), pending.path.display().to_string())?;
        let id = addition.snapshot.id.clone();
        let (cancel, checked) = (addition.cancel.clone(), addition.checked.clone());
        let findings = blocking(move || {
            let control = Control {
                cancel: &cancel,
                checked: &checked,
                log: &log,
            };
            scan::scan(&config, &scope, &control)
        })
        .await;
        self.call(move |state| state.finish_addition(&id, &pending, findings))
            .await
    }

    /// Discards the running addition and returns whether one was discarded; a scan that
    /// already saved its project is kept.
    pub fn cancel_project_addition(&self) -> AppResult<bool> {
        self.targets().cancel_addition()
    }

    /// Clears a finished or failed addition.
    pub fn dismiss_project_addition(&self) -> AppResult<()> {
        self.targets().dismiss_addition()
    }

    /// The project being added and the targets being rescanned, without waiting on the worker.
    pub fn target_progress(&self) -> AppResult<(Option<ProjectAddition>, Vec<DeploymentTarget>)> {
        self.targets().current()
    }

    /// Rescans one target and replaces its recorded deployments. A request made while a rescan
    /// of the target runs makes that rescan run once more, and so does a deployment write that
    /// overlapped it. A project whose folder is missing or unreadable keeps its deployments,
    /// and a removed project is skipped.
    pub async fn rescan_target(&self, target: DeploymentTarget) -> AppResult<()> {
        let Some(_rescan) = self.targets().begin_rescan(&target)? else {
            return Ok(());
        };
        loop {
            let saved = self.rescan_once(target.clone()).await?;
            if saved && !self.targets().take_rerun(&target)? {
                return Ok(());
            }
        }
    }

    /// Scans once; `false` when a deployment write overlapped the scan and nothing was saved.
    async fn rescan_once(&self, target: DeploymentTarget) -> AppResult<bool> {
        let request = target.clone();
        let prepared = self
            .call(move |state| {
                Ok(state.rescan_scope(&request)?.map(|scope| {
                    let writes = state.targets.writes();
                    (scope, state.config.clone(), state.log.clone(), writes)
                }))
            })
            .await?;
        let Some((scope, config, log, writes)) = prepared else {
            return Ok(true);
        };
        let cancel = self.targets().stopping();
        let findings = blocking(move || {
            let control = Control {
                cancel: &cancel,
                checked: &AtomicUsize::new(0),
                log: &log,
            };
            scan::scan(&config, &scope, &control)
        })
        .await?;
        self.call(move |state| {
            if state.targets.writes() != writes {
                return Ok(false);
            }
            state.record_scan(&target, None, findings).map(|_| true)
        })
        .await
    }

    /// Replaces the agents whose skill directories a target writes to.
    pub async fn set_target_agents(
        &self,
        target: DeploymentTarget,
        agents: Vec<String>,
    ) -> AppResult<()> {
        self.call(move |state| state.set_target_agents(&target, &agents))
            .await
    }

    /// Copies library skills into the target's agent skill directories and records them.
    pub async fn add_deployments(
        &self,
        target: DeploymentTarget,
        skills: Vec<SkillId>,
    ) -> AppResult<()> {
        self.call(move |state| state.add_deployments(&target, &skills))
            .await
    }

    /// Deletes a deployment's recorded copies; a changed copy is deleted only when `discard`
    /// lists it.
    pub async fn remove_deployment(
        &self,
        target: DeploymentTarget,
        skill: SkillId,
        discard: Vec<String>,
    ) -> AppResult<Removal> {
        self.call(move |state| state.remove_deployment(&target, &skill, &discard))
            .await
    }

    /// Resolves the deployment the user was `shown`, with its `expected` drift, using the
    /// remedy the user picked. A deployment whose copies or drift changed meanwhile is stale and
    /// nothing is written.
    pub async fn resolve_drift(
        &self,
        shown: Deployment,
        expected: Drift,
        remedy: Remedy,
    ) -> AppResult<()> {
        self.call(move |state| state.resolve_drift(&shown, expected, remedy))
            .await
    }

    /// Renames a project; names need not be unique.
    pub async fn rename_project(&self, id: ProjectId, name: String) -> AppResult<()> {
        self.call(move |state| state.rename_project(&id, &name))
            .await
    }

    /// Replaces the project order with a permutation of every project.
    pub async fn reorder_projects(&self, order: Vec<ProjectId>) -> AppResult<()> {
        self.call(move |state| state.reorder_projects(&order)).await
    }

    /// Forgets a project and its deployments without touching its files. Returns the projects
    /// that contain it; their next rescan covers its folder again.
    pub async fn remove_project(&self, id: ProjectId) -> AppResult<Vec<ProjectId>> {
        self.call(move |state| state.remove_project(&id)).await
    }
}
