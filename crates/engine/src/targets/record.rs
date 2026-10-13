//! Turns a target scan into deployments, and keeps deployed skills in the library.

use super::{
    drift::copy_change,
    importables, key,
    scan::{Findings, Scope},
};
use crate::{persistence::now, worker::State};
use rusqlite::{OptionalExtension, Transaction, params};
use skillbinder_proto::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

/// A workspace that exists only until its first scan is saved.
pub(crate) struct PendingWorkspace {
    pub id: WorkspaceId,
    pub name: String,
    pub path: PathBuf,
}

/// What a saved scan recorded.
pub(crate) struct Summary {
    pub in_library: usize,
    pub importable: Vec<String>,
}

impl State {
    /// Consumes a directory grant for a new workspace; an exact duplicate folder is refused.
    pub(crate) fn prepare_workspace(
        &mut self,
        grant: &GrantId,
    ) -> AppResult<(PendingWorkspace, Scope)> {
        self.require_library()?;
        let path = self.take_grant(grant)?;
        let display = path
            .to_str()
            .ok_or_else(|| AppError::validation("Directory path must be valid Unicode."))?;
        let existing = self
            .database
            .query_row(
                "SELECT name FROM workspaces WHERE canonical_path=?1",
                [display],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| AppError::storage())?;
        if let Some(name) = existing {
            return Err(AppError::validation(format!(
                "This folder is already added as {name}."
            )));
        }
        let name = path.file_name().map_or_else(
            || "Workspace".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
        let scope = Scope::Workspace {
            nested: self.nested_workspaces(&path)?,
            path: path.clone(),
        };
        let workspace = PendingWorkspace {
            id: WorkspaceId::new(),
            name,
            path,
        };
        Ok((workspace, scope))
    }

    /// What a rescan covers, or `None` when there is nothing to scan: the library is not set up,
    /// or the workspace folder is missing and keeps its last deployments.
    pub(crate) fn rescan_scope(&self, target: &DeploymentTarget) -> AppResult<Option<Scope>> {
        if self.require_library().is_err() {
            return Ok(None);
        }
        let DeploymentTarget::Workspace(id) = target else {
            return Ok(Some(Scope::Global));
        };
        // A removed workspace has nothing to scan; an unreadable folder counts as missing.
        let Some(path) = self.workspace_path(id)? else {
            return Ok(None);
        };
        if fs::read_dir(&path).is_err() {
            return Ok(None);
        }
        Ok(Some(Scope::Workspace {
            nested: self.nested_workspaces(&path)?,
            path,
        }))
    }

    pub(super) fn workspace_path(&self, id: &WorkspaceId) -> AppResult<Option<PathBuf>> {
        Ok(self
            .database
            .query_row(
                "SELECT canonical_path FROM workspaces WHERE id=?1",
                [id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| AppError::storage())?
            .map(PathBuf::from))
    }

    pub(super) fn nested_workspaces(&self, path: &Path) -> AppResult<BTreeSet<PathBuf>> {
        Ok(self
            .workspaces()?
            .into_iter()
            .map(|workspace| PathBuf::from(workspace.path))
            .filter(|other| other != path && other.starts_with(path))
            .collect())
    }

    fn require_library(&self) -> AppResult<()> {
        let step: OnboardingStep = self.setting("onboarding")?.unwrap_or_default();
        if step == OnboardingStep::Complete {
            Ok(())
        } else {
            Err(AppError::validation(
                "Finish setup before adding workspaces.",
            ))
        }
    }

    /// Saves a finished addition scan unless the addition was cancelled meanwhile. Holding the
    /// progress lock while saving makes cancellation and saving mutually exclusive.
    pub(crate) fn finish_addition(
        &mut self,
        id: &ScanId,
        pending: &PendingWorkspace,
        findings: AppResult<Findings>,
    ) -> AppResult<Option<WorkspaceAddition>> {
        let scans = self.targets.clone();
        let mut progress = scans.lock()?;
        let Some(current) = progress.addition.as_mut().filter(|addition| {
            &addition.snapshot.id == id && !addition.cancel.load(Ordering::Acquire)
        }) else {
            return Ok(None);
        };
        let target = DeploymentTarget::Workspace(pending.id.clone());
        let snapshot = &mut current.snapshot;
        snapshot.folders_checked = current.checked.load(Ordering::Relaxed);
        match findings.and_then(|findings| self.record_scan(&target, Some(pending), findings)) {
            Ok(summary) => {
                snapshot.status = ScanStatus::Finished;
                snapshot.in_library = summary.in_library;
                snapshot.importable = summary.importable;
            }
            Err(error) => {
                self.log.record(error.to_string());
                snapshot.status = ScanStatus::Failed;
                snapshot.error = Some(error.message);
            }
        }
        Ok(Some(snapshot.clone()))
    }

    /// Replaces the target's deployments with the library skills its scan found, and saves a
    /// new workspace with them, in one transaction. A recorded copy stays recorded with its
    /// digest when it changed or disappeared, and the scan records how it differs, so the user
    /// resolves the drift and removing a changed copy asks first.
    pub(crate) fn record_scan(
        &mut self,
        target: &DeploymentTarget,
        new: Option<&PendingWorkspace>,
        findings: Findings,
    ) -> AppResult<Summary> {
        // Workspaces added or removed while the scan ran decide again which copies it holds.
        let nested = match (new, target) {
            (None, DeploymentTarget::Workspace(id)) => match self.workspace_path(id)? {
                Some(path) => self.nested_workspaces(&path)?,
                None => {
                    return Ok(Summary {
                        in_library: 0,
                        importable: Vec::new(),
                    });
                }
            },
            _ => BTreeSet::new(),
        };
        let library = self.read_library()?;
        let found: BTreeMap<String, String> = findings
            .copies
            .iter()
            .map(|copy| (copy.path.clone(), copy.digest.clone()))
            .collect();
        let mut deployed: BTreeMap<SkillId, (String, Vec<String>)> = BTreeMap::new();
        let mut unmatched = Vec::new();
        for copy in findings.copies {
            if nested
                .iter()
                .any(|workspace| Path::new(&copy.path).starts_with(workspace))
            {
                continue;
            }
            match library
                .skills
                .iter()
                .find(|skill| skill.digest == copy.digest)
            {
                Some(skill) => deployed
                    .entry(skill.id.clone())
                    .or_insert_with(|| (copy.digest, Vec::new()))
                    .1
                    .push(copy.path),
                None => unmatched.push(copy),
            }
        }
        self.retain_recorded(target, &nested, &library, &mut deployed)?;
        // A recorded copy that changed belongs to its deployment's drift, not to the importables.
        let recorded: BTreeSet<&String> = deployed.values().flat_map(|(_, paths)| paths).collect();
        unmatched.retain(|copy| !recorded.contains(&copy.path));
        let transaction = self
            .database
            .transaction()
            .map_err(|_| AppError::storage())?;
        if let Some(workspace) = new {
            insert_workspace(&transaction, workspace)?;
        }
        replace_deployments(&transaction, target, &deployed, &found)?;
        add_detected_agents(&transaction, target, &findings.agents)?;
        transaction.commit().map_err(|_| AppError::storage())?;
        let importables = importables::group(target, unmatched);
        let summary = Summary {
            in_library: deployed.len(),
            importable: importables.iter().map(|i| i.slug.clone()).collect(),
        };
        self.keep_importables(target, new, importables);
        Ok(summary)
    }

    /// Adds the target's recorded copies that the scan did not match, such as changed or
    /// missing copies, while they lie outside nested workspaces and their skill stays in the
    /// library.
    fn retain_recorded(
        &self,
        target: &DeploymentTarget,
        nested: &BTreeSet<PathBuf>,
        library: &Library,
        deployed: &mut BTreeMap<SkillId, (String, Vec<String>)>,
    ) -> AppResult<()> {
        let found: BTreeSet<String> = deployed
            .values()
            .flat_map(|(_, paths)| paths.iter().cloned())
            .collect();
        for recorded in self
            .deployments()?
            .into_iter()
            .filter(|d| &d.target == target)
            .filter(|d| library.skills.iter().any(|skill| skill.id == d.skill_id))
        {
            for path in recorded.paths {
                let inside = nested
                    .iter()
                    .any(|workspace| Path::new(&path).starts_with(workspace));
                if found.contains(&path) || inside {
                    continue;
                }
                deployed
                    .entry(recorded.skill_id.clone())
                    .or_insert_with(|| (recorded.digest.clone(), Vec::new()))
                    .1
                    .push(path);
            }
        }
        Ok(())
    }

    /// Refuses a library change that drops a skill still deployed to any target.
    pub(crate) fn require_undeployed(&self, before: &Library, after: &Library) -> AppResult<()> {
        for skill in before
            .skills
            .iter()
            .filter(|old| !after.skills.iter().any(|new| new.id == old.id))
        {
            let mut query = self
                .database
                .prepare(
                    "SELECT COALESCE(p.name,'Global') FROM deployments d LEFT JOIN workspaces p ON p.id=d.target \
                     WHERE d.skill_id=?1 ORDER BY d.target<>'global',p.position",
                )
                .map_err(|_| AppError::storage())?;
            let targets = query
                .query_map([skill.id.as_str()], |row| row.get::<_, String>(0))
                .map_err(|_| AppError::storage())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| AppError::storage())?;
            if !targets.is_empty() {
                return Err(AppError::validation(format!(
                    "{} is deployed to {}. Remove those deployments before deleting it.",
                    skill.slug,
                    targets.join(", ")
                )));
            }
        }
        Ok(())
    }
}

/// Inserts a new workspace last, and takes the copies inside it away from the workspaces that
/// contain it: the deepest workspace wins.
fn insert_workspace(transaction: &Transaction, workspace: &PendingWorkspace) -> AppResult<()> {
    let path = workspace.path.display().to_string();
    transaction
        .execute(
            "INSERT INTO workspaces(id,canonical_path,name,position,created_at) \
             VALUES(?1,?2,?3,(SELECT COALESCE(MAX(position)+1,0) FROM workspaces),?4)",
            params![workspace.id.as_str(), path, workspace.name, now()],
        )
        .map_err(|_| AppError::storage())?;
    let mut query = transaction
        .prepare(
            "SELECT d.target,d.skill_id,d.paths,p.canonical_path,d.changes FROM deployments d \
             JOIN workspaces p ON p.id=d.target WHERE p.id<>?1",
        )
        .map_err(|_| AppError::storage())?;
    let rows = query
        .query_map([workspace.id.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|_| AppError::storage())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppError::storage())?;
    for (target, skill, paths, ancestor, changes) in rows {
        if !workspace.path.starts_with(&ancestor) {
            continue;
        }
        let paths: Vec<String> = serde_json::from_str(&paths).map_err(|_| AppError::storage())?;
        let outside = |path: &String| !Path::new(path).starts_with(&workspace.path);
        let kept: Vec<_> = paths.iter().filter(|path| outside(path)).collect();
        let mut changes: BTreeMap<String, CopyChange> =
            serde_json::from_str(&changes).map_err(|_| AppError::storage())?;
        changes.retain(|path, _| outside(path));
        if kept.len() == paths.len() {
            continue;
        }
        if kept.is_empty() {
            transaction.execute(
                "DELETE FROM deployments WHERE target=?1 AND skill_id=?2",
                params![target, skill],
            )
        } else {
            transaction.execute(
                "UPDATE deployments SET paths=?1,changes=?2 WHERE target=?3 AND skill_id=?4",
                params![
                    serde_json::to_string(&kept).map_err(|_| AppError::storage())?,
                    serde_json::to_string(&changes).map_err(|_| AppError::storage())?,
                    target,
                    skill
                ],
            )
        }
        .map_err(|_| AppError::storage())?;
    }
    Ok(())
}

/// How each copy differs from `digest`, given the digests the scan found at each path. A path
/// the scan did not report, such as one reached through a linked agent directory, is checked
/// directly.
fn changes(
    digest: &str,
    paths: &[String],
    found: &BTreeMap<String, String>,
) -> BTreeMap<String, CopyChange> {
    paths
        .iter()
        .filter_map(|path| {
            let change = match found.get(path) {
                Some(copy) if copy == digest => None,
                Some(_) => Some(CopyChange::Changed),
                None => copy_change(Path::new(path), digest),
            };
            Some((path.clone(), change?))
        })
        .collect()
}

fn replace_deployments(
    transaction: &Transaction,
    target: &DeploymentTarget,
    deployed: &BTreeMap<SkillId, (String, Vec<String>)>,
    found: &BTreeMap<String, String>,
) -> AppResult<()> {
    transaction
        .execute("DELETE FROM deployments WHERE target=?1", [key(target)])
        .map_err(|_| AppError::storage())?;
    for (skill, (digest, paths)) in deployed {
        let mut paths = paths.clone();
        paths.sort();
        transaction
            .execute(
                "INSERT INTO deployments(target,skill_id,digest,paths,recorded_at,changes) \
                 VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    key(target),
                    skill.as_str(),
                    digest,
                    serde_json::to_string(&paths).map_err(|_| AppError::storage())?,
                    now(),
                    serde_json::to_string(&changes(digest, &paths, found))
                        .map_err(|_| AppError::storage())?
                ],
            )
            .map_err(|_| AppError::storage())?;
    }
    Ok(())
}

/// A scan adds each agent whose skill directory it finds in the target for the first time to the
/// target agents. Agents found before stay as the user left them, so removing one keeps it removed.
fn add_detected_agents(
    transaction: &Transaction,
    target: &DeploymentTarget,
    agents: &BTreeSet<String>,
) -> AppResult<()> {
    for agent in agents {
        let first = transaction
            .execute(
                "INSERT OR IGNORE INTO detected_agents(target,agent) VALUES(?1,?2)",
                params![key(target), agent],
            )
            .map_err(|_| AppError::storage())?;
        if first > 0 {
            transaction
                .execute(
                    "INSERT OR IGNORE INTO target_agents(target,agent) VALUES(?1,?2)",
                    params![key(target), agent],
                )
                .map_err(|_| AppError::storage())?;
        }
    }
    Ok(())
}
