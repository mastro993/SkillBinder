//! Workspace rows in machine-local SQLite.

use super::{
    activation::{self, Held},
    icons,
    projects::{is_agent_folder, projects},
};
use crate::{
    database,
    discovery::registry::{agent_options, global_skill_dirs, workspace_skill_dirs},
    library::validate_name,
    library_store::{self, from_text, to_text},
    logging::Logger,
    persistence::{now, sync_directory},
    schema::workspaces,
    worker::State,
};
use diesel::prelude::*;
use skillbinder_proto::*;
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

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
                color: None,
                icon: None,
                image: None,
                dirs: BTreeSet::new(),
                paths: Default::default(),
                skills: Default::default(),
                overlaps: BTreeSet::new(),
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
        let slugs = library_store::slugs(&mut self.database.borrow_mut())?;
        let held = added
            .iter()
            .map(|workspace| activation::held_in(Path::new(&workspace.path), &slugs))
            .collect::<AppResult<Vec<Held>>>()?;
        // One row per statement: a single multi-row insert would exceed SQLite's bound-parameter
        // limit when a picked folder holds thousands of projects.
        self.change_places(|state| {
            database::transaction(&mut state.database.borrow_mut(), |connection| {
                for (workspace, held) in added.iter().zip(&held) {
                    diesel::insert_into(workspaces::table)
                        .values((
                            workspaces::id.eq(workspace.id.as_str()),
                            workspaces::canonical_path.eq(&workspace.path),
                            workspaces::name.eq(&workspace.name),
                            workspaces::position.eq(i64::from(workspace.position)),
                            workspaces::created_at.eq(now() as i64),
                        ))
                        .execute(connection)
                        .map_err(|_| AppError::storage())?;
                    let skills: Vec<_> = held
                        .skills
                        .iter()
                        .map(|(skill, targets)| (skill, Some(targets)))
                        .collect();
                    activation::save(connection, &workspace.id, &held.dirs, &skills)?;
                }
                Ok(())
            })
        })?;
        let ids: BTreeSet<_> = added.into_iter().map(|workspace| workspace.id).collect();
        Ok(self
            .workspaces()?
            .into_iter()
            .filter(|workspace| ids.contains(&workspace.id))
            .collect())
    }

    pub(crate) fn workspaces(&self) -> AppResult<Vec<Workspace>> {
        let agents = agent_options(&self.config)?;
        self.workspaces_for(&self.read_library()?, &agents)
    }

    /// The workspaces in order, each marking the skills it holds for an agent that `library`'s
    /// active preset holds them for too.
    pub(crate) fn workspaces_for(
        &self,
        library: &Library,
        agents: &[AgentOption],
    ) -> AppResult<Vec<Workspace>> {
        let mut held = activation::load(&mut self.database.borrow_mut())?;
        let rows = workspaces::table
            .select((
                workspaces::id,
                workspaces::canonical_path,
                workspaces::name,
                workspaces::position,
                workspaces::color,
                workspaces::icon,
                workspaces::icon_image,
            ))
            .order((workspaces::position, workspaces::id))
            .load::<(
                String,
                String,
                String,
                i64,
                Option<String>,
                String,
                Option<String>,
            )>(&mut *self.database.borrow_mut())
            .map_err(|_| AppError::storage())?;
        let mut workspaces = rows
            .into_iter()
            .map(|(id, path, name, position, color, icon, image)| {
                let color = stored_color(color.as_deref(), &name, &self.log);
                let icon = stored_icon(&icon, &name, &self.log);
                let image = image.and_then(|file| self.stored_image(&file));
                let Held { dirs, skills } = held.remove(&id).unwrap_or_default();
                let paths = activation::paths(Path::new(&path), &dirs);
                Ok(Workspace {
                    missing: !Path::new(&path).is_dir(),
                    id: WorkspaceId(id),
                    name,
                    path,
                    position: u32::try_from(position).map_err(|_| AppError::storage())?,
                    color,
                    icon,
                    image,
                    dirs,
                    paths,
                    skills,
                    overlaps: BTreeSet::new(),
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        crate::activation::mark_overlaps(&mut workspaces, library, agents);
        Ok(workspaces)
    }

    /// Sets or clears the workspace glyph's color.
    pub(crate) fn set_workspace_color(
        &self,
        id: &WorkspaceId,
        color: Option<PresetColor>,
    ) -> AppResult<()> {
        let color = color.as_ref().map(to_text).transpose()?;
        let changed = diesel::update(workspaces::table.find(id.as_str()))
            .set(workspaces::color.eq(color))
            .execute(&mut *self.database.borrow_mut())
            .map_err(|_| AppError::storage())?;
        if changed == 0 {
            return Err(AppError::not_found("Workspace no longer exists."));
        }
        Ok(())
    }

    /// Sets the workspace glyph, or restores the default folder glyph, and removes any image
    /// icon it had.
    pub(crate) fn set_workspace_icon(
        &mut self,
        id: &WorkspaceId,
        icon: Option<PresetIcon>,
    ) -> AppResult<()> {
        let previous = self.workspace_icon_file(id)?;
        let changed = diesel::update(workspaces::table.find(id.as_str()))
            .set((
                workspaces::icon.eq(icon_text(icon)?),
                workspaces::icon_image.eq(None::<String>),
            ))
            .execute(&mut *self.database.borrow_mut())
            .map_err(|_| AppError::storage())?;
        if changed == 0 {
            return Err(AppError::not_found("Workspace no longer exists."));
        }
        self.discard_icon(previous);
        Ok(())
    }

    /// Copies the picked image into application storage and makes it the workspace icon. The
    /// picked file is never referenced after this call.
    pub(crate) fn set_workspace_image(&mut self, id: &WorkspaceId, source: &Path) -> AppResult<()> {
        let previous = self.workspace_icon_file(id)?;
        let (bytes, extension) = icons::read(source)?;
        let icons = self.icons_dir();
        fs::create_dir_all(&icons).map_err(|_| AppError::storage())?;
        let name = format!("workspace-{}.{}", uuid::Uuid::new_v4(), extension);
        let target = icons.join(&name);
        let mut temporary = tempfile::NamedTempFile::new_in(self.config.data_dir.join("staging"))
            .map_err(|_| AppError::storage())?;
        temporary
            .write_all(&bytes)
            .and_then(|()| temporary.as_file().sync_all())
            .map_err(|_| AppError::storage())?;
        temporary
            .persist(&target)
            .map_err(|_| AppError::storage())?;
        if let Err(error) = sync_directory(&icons) {
            let _ = fs::remove_file(&target);
            return Err(error);
        }
        let changed = diesel::update(workspaces::table.find(id.as_str()))
            .set(workspaces::icon_image.eq(Some(name.clone())))
            .execute(&mut *self.database.borrow_mut())
            .map_err(|_| AppError::storage());
        match changed {
            Ok(0) => {
                let _ = fs::remove_file(&target);
                Err(AppError::not_found("Workspace no longer exists."))
            }
            Ok(_) => {
                self.discard_icon(previous);
                Ok(())
            }
            Err(error) => {
                let _ = fs::remove_file(&target);
                Err(error)
            }
        }
    }

    pub(crate) fn rename_workspace(&self, id: &WorkspaceId, name: &str) -> AppResult<()> {
        let name = validate_name(name)?;
        let changed = diesel::update(workspaces::table.find(id.as_str()))
            .set(workspaces::name.eq(name))
            .execute(&mut *self.database.borrow_mut())
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
        database::transaction(&mut self.database.borrow_mut(), |connection| {
            order
                .iter()
                .enumerate()
                .try_for_each(|(position, id)| set_position(connection, id, position))
        })
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

    /// Forgets a workspace, the skills it holds, and the copy records held for it alone. Files in
    /// the folder are not touched: its managed copies stay there, unmanaged.
    pub(crate) fn remove_workspace(&mut self, id: &WorkspaceId) -> AppResult<()> {
        let image = self.workspace_icon_file(id)?;
        let workspaces = self.workspaces()?;
        let handed = self.handed_over(id)?;
        database::transaction(&mut self.database.borrow_mut(), |connection| {
            // The rest of its records cascade away with it.
            crate::activation::claim(connection, &handed)?;
            diesel::delete(workspaces::table.find(id.as_str()))
                .execute(connection)
                .map_err(|_| AppError::storage())?;
            workspaces
                .iter()
                .filter(|p| &p.id != id)
                .enumerate()
                .try_for_each(|(position, workspace)| {
                    set_position(connection, &workspace.id, position)
                })
        })?;
        self.discard_icon(image);
        Ok(())
    }

    /// The stored image file name of workspace `id`, or `None` when it has no image.
    fn workspace_icon_file(&self, id: &WorkspaceId) -> AppResult<Option<String>> {
        workspaces::table
            .find(id.as_str())
            .select(workspaces::icon_image)
            .first::<Option<String>>(&mut *self.database.borrow_mut())
            .optional()
            .map_err(|_| AppError::storage())?
            .ok_or_else(|| AppError::not_found("Workspace no longer exists."))
    }

    /// The absolute path of a stored image, when `file` is a plain file name naming a regular
    /// file.
    fn stored_image(&self, file: &str) -> Option<String> {
        let path = self.icon_path(file)?;
        fs::symlink_metadata(&path)
            .is_ok_and(|metadata| metadata.file_type().is_file())
            .then(|| path.to_string_lossy().into_owned())
    }

    /// The path of `file` under the icons directory, or `None` unless it is a plain file name.
    fn icon_path(&self, file: &str) -> Option<PathBuf> {
        let mut components = Path::new(file).components();
        let Some(Component::Normal(_)) = components.next() else {
            return None;
        };
        components
            .next()
            .is_none()
            .then(|| self.icons_dir().join(file))
    }

    fn icons_dir(&self) -> PathBuf {
        self.config.data_dir.join("icons")
    }

    /// Removes a stored image file. A missing file needs no removal; a failure is logged, since
    /// the workspace change it belongs to has already been saved.
    fn discard_icon(&self, file: Option<String>) {
        let Some(path) = file.and_then(|file| self.icon_path(&file)) else {
            return;
        };
        if let Err(error) = fs::remove_file(&path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            self.log.record(format!(
                "Could not remove workspace icon {}: {error}",
                path.display()
            ));
        }
    }
}

fn set_position(
    connection: &mut SqliteConnection,
    id: &WorkspaceId,
    position: usize,
) -> AppResult<()> {
    diesel::update(workspaces::table.find(id.as_str()))
        .set(workspaces::position.eq(position as i64))
        .execute(connection)
        .map(drop)
        .map_err(|_| AppError::storage())
}

/// A stored glyph, `"folder"` for the default, or `None` when this version does not know it.
fn stored_icon(text: &str, workspace: &str, log: &Logger) -> Option<PresetIcon> {
    if text == "folder" {
        return None;
    }
    match from_text(text) {
        Ok(icon) => Some(icon),
        Err(_) => {
            log.record(format!(
                "Workspace {workspace} has an icon this version does not know; showing the folder."
            ));
            None
        }
    }
}

/// A stored color, or `None` when there is none or this version does not know it.
fn stored_color(text: Option<&str>, workspace: &str, log: &Logger) -> Option<PresetColor> {
    let text = text?;
    match from_text(text) {
        Ok(color) => Some(color),
        Err(_) => {
            log.record(format!(
                "Workspace {workspace} has a color this version does not know; showing none."
            ));
            None
        }
    }
}

/// A glyph's stored text, with `"folder"` for the default.
fn icon_text(icon: Option<PresetIcon>) -> AppResult<String> {
    match icon {
        Some(icon) => to_text(&icon),
        None => Ok("folder".to_owned()),
    }
}

/// A new workspace's name: its folder's last path component.
fn name(path: &Path) -> String {
    path.file_name().map_or_else(
        || "Workspace".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}
