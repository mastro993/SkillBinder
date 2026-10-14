//! Workspace rows in machine-local SQLite.

use super::projects::{is_agent_folder, projects};
use crate::{
    discovery::registry::{global_skill_dirs, workspace_skill_dirs},
    library::validate_name,
    persistence::now,
    worker::State,
};
use rusqlite::params;
use skillbinder_proto::*;
use std::path::Path;

impl State {
    /// Consumes a directory grant and saves the picked folder's projects as the last
    /// workspaces: the folder itself and each direct child that has an agent skill directory, or
    /// the folder alone when none has one. Agent folders, such as the home folder, and folders
    /// already added are skipped; when nothing is left to add, the addition is refused, naming
    /// the home folder when it is one of the skipped agent folders.
    pub(crate) fn add_workspaces(&mut self, grant: &GrantId) -> AppResult<Vec<Workspace>> {
        let step: OnboardingStep = self.setting("onboarding")?.unwrap_or_default();
        if step != OnboardingStep::Complete {
            return Err(AppError::validation(
                "Finish setup before adding workspaces.",
            ));
        }
        let picked = self.take_grant(grant)?;
        let picked_name = name(&picked);
        if picked.to_str().is_none() {
            return Err(AppError::validation(
                "Directory path must be valid Unicode.",
            ));
        }
        let existing = self.workspaces()?;
        let global_dirs = global_skill_dirs(&self.config)?;
        let projects = projects(&picked, &workspace_skill_dirs()?, &global_dirs);
        if projects.add.is_empty() {
            return Err(AppError::validation(
                if projects.skipped.contains(&self.config.home) {
                    "Your home folder's skills are shown under Agents."
                } else {
                    "This folder's skills are shown under Agents."
                },
            ));
        }
        let found: Vec<String> = projects
            .add
            .into_iter()
            .filter_map(|path| path.to_str().map(str::to_owned))
            .collect();
        let added: Vec<_> = found
            .iter()
            .filter(|path| !existing.iter().any(|workspace| &workspace.path == *path))
            .enumerate()
            .map(|(index, path)| Workspace {
                id: WorkspaceId::new(),
                name: name(Path::new(path)),
                path: path.clone(),
                position: (existing.len() + index) as u32,
                missing: false,
            })
            .collect();
        if added.is_empty() {
            return Err(AppError::validation(
                // Only the picked folder itself is named; its projects are not what was picked.
                match existing.iter().find(|workspace| {
                    found == [workspace.path.clone()] && Path::new(&workspace.path) == picked
                }) {
                    Some(workspace) => {
                        format!("This folder is already added as {}.", workspace.name)
                    }
                    None => format!("Every project in {picked_name} is already added."),
                },
            ));
        }
        let transaction = self
            .database
            .transaction()
            .map_err(|_| AppError::storage())?;
        for workspace in &added {
            transaction
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
        }
        transaction.commit().map_err(|_| AppError::storage())?;
        Ok(added)
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

    /// Forgets every stored workspace that is an agent folder, such as the home folder, whose
    /// skills are shown under Agents. Files in the folders are not touched.
    pub(crate) fn forget_agent_folders(&mut self) -> AppResult<()> {
        let (skill_dirs, global_dirs) = (workspace_skill_dirs()?, global_skill_dirs(&self.config)?);
        let refused: Vec<_> = self
            .workspaces()?
            .into_iter()
            .filter(|workspace| {
                is_agent_folder(Path::new(&workspace.path), &skill_dirs, &global_dirs)
            })
            .collect();
        for workspace in refused {
            self.log
                .record(format!("Forgot agent folder workspace {}", workspace.path));
            self.remove_workspace(&workspace.id)?;
        }
        Ok(())
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

/// A new workspace's name: its folder's last path component.
fn name(path: &Path) -> String {
    path.file_name().map_or_else(
        || "Workspace".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}
