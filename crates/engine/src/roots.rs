use crate::{library::validate_name, persistence::now, worker::State};
use rusqlite::{OptionalExtension, params};
use skillbinder_proto::*;
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

pub(crate) struct GrantedDirectory {
    path: PathBuf,
    created: Instant,
}

impl State {
    pub(crate) fn grant_directory(&mut self, path: PathBuf) -> AppResult<DirectoryGrant> {
        self.grants
            .retain(|_, grant| grant.created.elapsed() < Duration::from_secs(300));
        let canonical = fs::canonicalize(&path)
            .map_err(|_| AppError::validation("Choose an accessible directory."))?;
        if !canonical.is_dir() {
            return Err(AppError::validation("Choose a directory."));
        }
        let id = GrantId::new();
        let display_path = canonical.display().to_string();
        self.grants.insert(
            id.clone(),
            GrantedDirectory {
                path: canonical,
                created: Instant::now(),
            },
        );
        Ok(DirectoryGrant { id, display_path })
    }
    pub(crate) fn register_root(
        &mut self,
        grant: GrantId,
        label: String,
    ) -> AppResult<ProjectRoot> {
        let grant = self
            .grants
            .remove(&grant)
            .ok_or_else(|| AppError::stale("Directory selection expired or was already used."))?;
        if grant.created.elapsed() >= Duration::from_secs(300) {
            return Err(AppError::stale("Directory selection expired."));
        }
        let canonical = fs::canonicalize(&grant.path)
            .map_err(|_| AppError::stale("Selected directory no longer exists."))?;
        if canonical != grant.path {
            return Err(AppError::stale("Selected directory changed."));
        }
        let label = if label.trim().is_empty() {
            canonical.file_name().map_or_else(
                || "Project".to_owned(),
                |name| name.to_string_lossy().into_owned(),
            )
        } else {
            validate_name(&label)?
        };
        let path = canonical
            .to_str()
            .ok_or_else(|| AppError::validation("Directory path must be valid Unicode."))?
            .to_owned();
        let existing = self
            .database
            .query_row(
                "SELECT label FROM scan_roots WHERE canonical_path=?1",
                [&path],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| AppError::storage())?;
        if let Some(label) = existing {
            return Err(AppError::validation(format!(
                "This directory is already registered as {label}."
            )));
        }
        let root = ProjectRoot {
            id: RootId::new(),
            path,
            label,
            enabled: true,
        };
        self.database.execute("INSERT INTO scan_roots(id,canonical_path,display_path,label,enabled,created_at) VALUES(?1,?2,?2,?3,1,?4)", params![root.id.as_str(), root.path, root.label, now()]).map_err(|_| AppError::storage())?;
        Ok(root)
    }
    pub(crate) fn roots(&self) -> AppResult<Vec<ProjectRoot>> {
        let mut query = self.database.prepare("SELECT id,display_path,label,enabled FROM scan_roots ORDER BY label COLLATE NOCASE,id").map_err(|_| AppError::storage())?;
        query
            .query_map([], |row| {
                Ok(ProjectRoot {
                    id: RootId(row.get(0)?),
                    path: row.get(1)?,
                    label: row.get(2)?,
                    enabled: row.get(3)?,
                })
            })
            .map_err(|_| AppError::storage())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AppError::storage())
    }
    pub(crate) fn update_root(&self, id: RootId, label: String, enabled: bool) -> AppResult<()> {
        let label = validate_name(&label)?;
        if self
            .database
            .execute(
                "UPDATE scan_roots SET label=?1,enabled=?2 WHERE id=?3",
                params![label, enabled, id.as_str()],
            )
            .map_err(|_| AppError::storage())?
            == 0
        {
            return Err(AppError::not_found("Project root no longer exists."));
        }
        Ok(())
    }
    pub(crate) fn remove_root(&self, id: RootId) -> AppResult<()> {
        self.database
            .execute("DELETE FROM scan_roots WHERE id=?1", [id.as_str()])
            .map_err(|_| AppError::storage())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_grant_is_refused_and_consumed_without_registering_root() -> AppResult<()> {
        let root = tempfile::tempdir().map_err(|_| AppError::storage())?;
        let mut state = State::isolated_test_state(root.path().join("data"));
        let grant = state.grant_directory(root.path().to_owned())?;
        state
            .grants
            .get_mut(&grant.id)
            .ok_or_else(AppError::storage)?
            .created = Instant::now() - Duration::from_secs(300);
        for _ in 0..2 {
            let result = state.register_root(grant.id.clone(), "Expired".into());
            assert!(result.is_err_and(|error| error.category == ErrorCategory::Stale));
        }
        assert!(state.roots()?.is_empty());
        Ok(())
    }
}
