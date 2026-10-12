//! Turns a target scan into deployments, and keeps deployed skills in the library.

use super::{
    key,
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

/// A project that exists only until its first scan is saved.
pub(crate) struct PendingProject {
    pub id: ProjectId,
    pub name: String,
    pub path: PathBuf,
}

/// What a saved scan recorded.
pub(crate) struct Summary {
    pub in_library: usize,
    pub importable: Vec<String>,
}

impl State {
    /// Consumes a directory grant for a new project; an exact duplicate folder is refused.
    pub(crate) fn prepare_project(
        &mut self,
        grant: &GrantId,
    ) -> AppResult<(PendingProject, Scope)> {
        self.require_library()?;
        let path = self.take_grant(grant)?;
        let display = path
            .to_str()
            .ok_or_else(|| AppError::validation("Directory path must be valid Unicode."))?;
        let existing = self
            .database
            .query_row(
                "SELECT name FROM projects WHERE canonical_path=?1",
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
            || "Project".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
        let scope = Scope::Project {
            nested: self.nested_projects(&path)?,
            path: path.clone(),
        };
        let project = PendingProject {
            id: ProjectId::new(),
            name,
            path,
        };
        Ok((project, scope))
    }

    /// What a rescan covers, or `None` when there is nothing to scan: the library is not set up,
    /// or the project folder is missing and keeps its last deployments.
    pub(crate) fn rescan_scope(&self, target: &DeploymentTarget) -> AppResult<Option<Scope>> {
        if self.require_library().is_err() {
            return Ok(None);
        }
        let DeploymentTarget::Project(id) = target else {
            return Ok(Some(Scope::Global));
        };
        // A removed project has nothing to scan; an unreadable folder counts as missing.
        let Some(path) = self.project_path(id)? else {
            return Ok(None);
        };
        if fs::read_dir(&path).is_err() {
            return Ok(None);
        }
        Ok(Some(Scope::Project {
            nested: self.nested_projects(&path)?,
            path,
        }))
    }

    fn project_path(&self, id: &ProjectId) -> AppResult<Option<PathBuf>> {
        Ok(self
            .database
            .query_row(
                "SELECT canonical_path FROM projects WHERE id=?1",
                [id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| AppError::storage())?
            .map(PathBuf::from))
    }

    fn nested_projects(&self, path: &Path) -> AppResult<BTreeSet<PathBuf>> {
        Ok(self
            .projects()?
            .into_iter()
            .map(|project| PathBuf::from(project.path))
            .filter(|other| other != path && other.starts_with(path))
            .collect())
    }

    fn require_library(&self) -> AppResult<()> {
        let step: OnboardingStep = self.setting("onboarding")?.unwrap_or_default();
        if step == OnboardingStep::Complete {
            Ok(())
        } else {
            Err(AppError::validation("Finish setup before adding projects."))
        }
    }

    /// Saves a finished addition scan unless the addition was cancelled meanwhile. Holding the
    /// progress lock while saving makes cancellation and saving mutually exclusive.
    pub(crate) fn finish_addition(
        &mut self,
        id: &ScanId,
        pending: &PendingProject,
        findings: AppResult<Findings>,
    ) -> AppResult<Option<ProjectAddition>> {
        let scans = self.targets.clone();
        let mut progress = scans.lock()?;
        let Some(current) = progress.addition.as_mut().filter(|addition| {
            &addition.snapshot.id == id && !addition.cancel.load(Ordering::Acquire)
        }) else {
            return Ok(None);
        };
        let target = DeploymentTarget::Project(pending.id.clone());
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
    /// new project with them, in one transaction.
    pub(crate) fn record_scan(
        &mut self,
        target: &DeploymentTarget,
        new: Option<&PendingProject>,
        findings: Findings,
    ) -> AppResult<Summary> {
        // Projects added or removed while the scan ran decide again which copies it holds.
        let nested = match (new, target) {
            (None, DeploymentTarget::Project(id)) => match self.project_path(id)? {
                Some(path) => self.nested_projects(&path)?,
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
        let mut deployed: BTreeMap<SkillId, (String, Vec<String>)> = BTreeMap::new();
        let mut importable: BTreeMap<String, String> = BTreeMap::new();
        for copy in findings.copies {
            if nested
                .iter()
                .any(|project| Path::new(&copy.path).starts_with(project))
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
                None => {
                    importable.entry(copy.digest).or_insert(copy.slug);
                }
            }
        }
        let transaction = self
            .database
            .transaction()
            .map_err(|_| AppError::storage())?;
        if let Some(project) = new {
            insert_project(&transaction, project)?;
        }
        replace_deployments(&transaction, target, &deployed)?;
        seed_agents(&transaction, target, &findings.agents)?;
        transaction.commit().map_err(|_| AppError::storage())?;
        let mut importable: Vec<_> = importable.into_values().collect();
        importable.sort();
        Ok(Summary {
            in_library: deployed.len(),
            importable,
        })
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
                    "SELECT COALESCE(p.name,'Global') FROM deployments d LEFT JOIN projects p ON p.id=d.target \
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

/// Inserts a new project last, and takes the copies inside it away from the projects that
/// contain it: the deepest project wins.
fn insert_project(transaction: &Transaction, project: &PendingProject) -> AppResult<()> {
    let path = project.path.display().to_string();
    transaction
        .execute(
            "INSERT INTO projects(id,canonical_path,name,position,created_at) \
             VALUES(?1,?2,?3,(SELECT COALESCE(MAX(position)+1,0) FROM projects),?4)",
            params![project.id.as_str(), path, project.name, now()],
        )
        .map_err(|_| AppError::storage())?;
    let mut query = transaction
        .prepare(
            "SELECT d.target,d.skill_id,d.paths,p.canonical_path FROM deployments d \
             JOIN projects p ON p.id=d.target WHERE p.id<>?1",
        )
        .map_err(|_| AppError::storage())?;
    let rows = query
        .query_map([project.id.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| AppError::storage())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppError::storage())?;
    for (target, skill, paths, ancestor) in rows {
        if !project.path.starts_with(&ancestor) {
            continue;
        }
        let paths: Vec<String> = serde_json::from_str(&paths).map_err(|_| AppError::storage())?;
        let kept: Vec<_> = paths
            .iter()
            .filter(|path| !Path::new(path).starts_with(&project.path))
            .collect();
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
                "UPDATE deployments SET paths=?1 WHERE target=?2 AND skill_id=?3",
                params![
                    serde_json::to_string(&kept).map_err(|_| AppError::storage())?,
                    target,
                    skill
                ],
            )
        }
        .map_err(|_| AppError::storage())?;
    }
    Ok(())
}

fn replace_deployments(
    transaction: &Transaction,
    target: &DeploymentTarget,
    deployed: &BTreeMap<SkillId, (String, Vec<String>)>,
) -> AppResult<()> {
    transaction
        .execute("DELETE FROM deployments WHERE target=?1", [key(target)])
        .map_err(|_| AppError::storage())?;
    for (skill, (digest, paths)) in deployed {
        let mut paths = paths.clone();
        paths.sort();
        transaction
            .execute(
                "INSERT INTO deployments(target,skill_id,digest,paths,recorded_at) VALUES(?1,?2,?3,?4,?5)",
                params![
                    key(target),
                    skill.as_str(),
                    digest,
                    serde_json::to_string(&paths).map_err(|_| AppError::storage())?,
                    now()
                ],
            )
            .map_err(|_| AppError::storage())?;
    }
    Ok(())
}

/// Target agents default to the agent skill directories found in the target.
fn seed_agents(
    transaction: &Transaction,
    target: &DeploymentTarget,
    agents: &BTreeSet<String>,
) -> AppResult<()> {
    let recorded: u32 = transaction
        .query_row(
            "SELECT COUNT(*) FROM target_agents WHERE target=?1",
            [key(target)],
            |row| row.get(0),
        )
        .map_err(|_| AppError::storage())?;
    if recorded > 0 {
        return Ok(());
    }
    for agent in agents {
        transaction
            .execute(
                "INSERT INTO target_agents(target,agent) VALUES(?1,?2)",
                params![key(target), agent],
            )
            .map_err(|_| AppError::storage())?;
    }
    Ok(())
}
