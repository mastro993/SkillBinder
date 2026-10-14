//! Workspace rows in machine-local SQLite.

use crate::{library::validate_name, persistence::now, worker::State};
use rusqlite::{OptionalExtension, params};
use skillbinder_proto::*;
use std::path::Path;

impl State {
    /// Consumes a directory grant and saves its folder as the last workspace; an exact duplicate
    /// folder is refused.
    pub(crate) fn add_workspace(&mut self, grant: &GrantId) -> AppResult<Workspace> {
        let step: OnboardingStep = self.setting("onboarding")?.unwrap_or_default();
        if step != OnboardingStep::Complete {
            return Err(AppError::validation(
                "Finish setup before adding workspaces.",
            ));
        }
        let path = self.take_grant(grant)?;
        let path = path
            .to_str()
            .ok_or_else(|| AppError::validation("Directory path must be valid Unicode."))?
            .to_owned();
        let existing = self
            .database
            .query_row(
                "SELECT name FROM workspaces WHERE canonical_path=?1",
                [&path],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| AppError::storage())?;
        if let Some(name) = existing {
            return Err(AppError::validation(format!(
                "This folder is already added as {name}."
            )));
        }
        let workspace = Workspace {
            id: WorkspaceId::new(),
            name: Path::new(&path).file_name().map_or_else(
                || "Workspace".to_owned(),
                |name| name.to_string_lossy().into_owned(),
            ),
            position: self.workspaces()?.len() as u32,
            missing: false,
            path,
        };
        self.database
            .execute(
                "INSERT INTO workspaces(id,canonical_path,name,position,created_at) VALUES(?1,?2,?3,?4,?5)",
                params![
                    workspace.id.as_str(),
                    workspace.path,
                    workspace.name,
                    workspace.position,
                    now()
                ],
            )
            .map_err(|_| AppError::storage())?;
        Ok(workspace)
    }

    pub(crate) fn workspaces(&self) -> AppResult<Vec<Workspace>> {
        let mut query = self
            .database
            .prepare("SELECT id,canonical_path,name,position FROM workspaces ORDER BY position,id")
            .map_err(|_| AppError::storage())?;
        let rows = query
            .query_map([], |row| {
                Ok((
                    WorkspaceId(row.get(0)?),
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
                Ok(Workspace {
                    missing: !Path::new(&path).is_dir(),
                    id,
                    name,
                    path,
                    position,
                })
            })
            .collect()
    }

    pub(crate) fn rename_workspace(&self, id: &WorkspaceId, name: &str) -> AppResult<()> {
        let name = validate_name(name)?;
        let changed = self
            .database
            .execute(
                "UPDATE workspaces SET name=?1 WHERE id=?2",
                params![name, id.as_str()],
            )
            .map_err(|_| AppError::storage())?;
        if changed == 0 {
            return Err(AppError::not_found("Workspace no longer exists."));
        }
        Ok(())
    }

    /// Replaces the workspace order with a permutation of every workspace.
    pub(crate) fn reorder_workspaces(&mut self, order: &[WorkspaceId]) -> AppResult<()> {
        let mut requested: Vec<_> = order.iter().collect();
        requested.sort();
        let current = self.workspaces()?;
        let mut stored: Vec<_> = current.iter().map(|workspace| &workspace.id).collect();
        stored.sort();
        if requested != stored {
            return Err(AppError::stale(
                "The workspaces changed. Reorder them again.",
            ));
        }
        let transaction = self
            .database
            .transaction()
            .map_err(|_| AppError::storage())?;
        for (position, id) in order.iter().enumerate() {
            transaction
                .execute(
                    "UPDATE workspaces SET position=?1 WHERE id=?2",
                    params![position as u32, id.as_str()],
                )
                .map_err(|_| AppError::storage())?;
        }
        transaction.commit().map_err(|_| AppError::storage())
    }

    /// Forgets a workspace. Files in the folder are not touched.
    pub(crate) fn remove_workspace(&mut self, id: &WorkspaceId) -> AppResult<()> {
        let workspaces = self.workspaces()?;
        if !workspaces.iter().any(|workspace| &workspace.id == id) {
            return Err(AppError::not_found("Workspace no longer exists."));
        }
        let transaction = self
            .database
            .transaction()
            .map_err(|_| AppError::storage())?;
        transaction
            .execute("DELETE FROM workspaces WHERE id=?1", [id.as_str()])
            .map_err(|_| AppError::storage())?;
        for (position, workspace) in workspaces.iter().filter(|p| &p.id != id).enumerate() {
            transaction
                .execute(
                    "UPDATE workspaces SET position=?1 WHERE id=?2",
                    params![position as u32, workspace.id.as_str()],
                )
                .map_err(|_| AppError::storage())?;
        }
        transaction.commit().map_err(|_| AppError::storage())
    }
}
