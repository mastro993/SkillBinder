//! Project rows, target agents, and deployment rows in machine-local SQLite.

use super::{key, target};
use crate::{library::validate_name, worker::State};
use rusqlite::params;
use skillbinder_proto::*;
use std::path::Path;

impl State {
    pub(crate) fn projects(&self) -> AppResult<Vec<Project>> {
        let mut query = self
            .database
            .prepare("SELECT id,canonical_path,name,position FROM projects ORDER BY position,id")
            .map_err(|_| AppError::storage())?;
        let rows = query
            .query_map([], |row| {
                Ok((
                    ProjectId(row.get(0)?),
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, u32>(3)?,
                ))
            })
            .map_err(|_| AppError::storage())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AppError::storage())?;
        rows.into_iter()
            .map(|(id, path, name, position)| {
                Ok(Project {
                    agents: self.target_agents(&DeploymentTarget::Project(id.clone()))?,
                    missing: !Path::new(&path).is_dir(),
                    id,
                    name,
                    path,
                    position,
                })
            })
            .collect()
    }

    pub(crate) fn target_agents(&self, target: &DeploymentTarget) -> AppResult<Vec<String>> {
        let mut query = self
            .database
            .prepare("SELECT agent FROM target_agents WHERE target=?1 ORDER BY agent")
            .map_err(|_| AppError::storage())?;
        query
            .query_map([key(target)], |row| row.get(0))
            .map_err(|_| AppError::storage())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AppError::storage())
    }

    pub(crate) fn deployments(&self) -> AppResult<Vec<Deployment>> {
        let mut query = self
            .database
            .prepare(
                "SELECT target,skill_id,digest,paths,changes FROM deployments ORDER BY target,skill_id",
            )
            .map_err(|_| AppError::storage())?;
        let rows = query
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    SkillId(row.get(1)?),
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|_| AppError::storage())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AppError::storage())?;
        rows.into_iter()
            .map(|(key, skill_id, digest, paths, changes)| {
                Ok(Deployment {
                    target: target(key),
                    skill_id,
                    digest,
                    paths: serde_json::from_str(&paths).map_err(|_| AppError::storage())?,
                    changes: serde_json::from_str(&changes).map_err(|_| AppError::storage())?,
                })
            })
            .collect()
    }

    pub(crate) fn rename_project(&self, id: &ProjectId, name: &str) -> AppResult<()> {
        let name = validate_name(name)?;
        let changed = self
            .database
            .execute(
                "UPDATE projects SET name=?1 WHERE id=?2",
                params![name, id.as_str()],
            )
            .map_err(|_| AppError::storage())?;
        if changed == 0 {
            return Err(AppError::not_found("Project no longer exists."));
        }
        Ok(())
    }

    /// Replaces the project order with a permutation of every project.
    pub(crate) fn reorder_projects(&mut self, order: &[ProjectId]) -> AppResult<()> {
        let mut requested: Vec<_> = order.iter().collect();
        requested.sort();
        let current = self.projects()?;
        let mut stored: Vec<_> = current.iter().map(|project| &project.id).collect();
        stored.sort();
        if requested != stored {
            return Err(AppError::stale("The projects changed. Reorder them again."));
        }
        let transaction = self
            .database
            .transaction()
            .map_err(|_| AppError::storage())?;
        for (position, id) in order.iter().enumerate() {
            transaction
                .execute(
                    "UPDATE projects SET position=?1 WHERE id=?2",
                    params![position as u32, id.as_str()],
                )
                .map_err(|_| AppError::storage())?;
        }
        transaction.commit().map_err(|_| AppError::storage())
    }

    /// Forgets a project, its agents, and its deployments. Files in the folder are not touched.
    /// Returns the projects that contain it, whose scans may now include its folder.
    pub(crate) fn remove_project(&mut self, id: &ProjectId) -> AppResult<Vec<ProjectId>> {
        let projects = self.projects()?;
        let removed = projects
            .iter()
            .find(|project| &project.id == id)
            .ok_or_else(|| AppError::not_found("Project no longer exists."))?;
        let target = DeploymentTarget::Project(id.clone());
        let transaction = self
            .database
            .transaction()
            .map_err(|_| AppError::storage())?;
        for statement in [
            "DELETE FROM deployments WHERE target=?1",
            "DELETE FROM target_agents WHERE target=?1",
            "DELETE FROM projects WHERE id=?1",
        ] {
            transaction
                .execute(statement, [key(&target)])
                .map_err(|_| AppError::storage())?;
        }
        for (position, project) in projects.iter().filter(|p| &p.id != id).enumerate() {
            transaction
                .execute(
                    "UPDATE projects SET position=?1 WHERE id=?2",
                    params![position as u32, project.id.as_str()],
                )
                .map_err(|_| AppError::storage())?;
        }
        transaction.commit().map_err(|_| AppError::storage())?;
        self.importables.remove(&target);
        Ok(projects
            .iter()
            .filter(|project| {
                &project.id != id && Path::new(&removed.path).starts_with(&project.path)
            })
            .map(|project| project.id.clone())
            .collect())
    }
}
