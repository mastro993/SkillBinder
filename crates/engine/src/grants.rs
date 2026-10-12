use crate::worker::State;
use skillbinder_proto::*;
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

const GRANT_LIFETIME: Duration = Duration::from_secs(300);

pub(crate) struct GrantedDirectory {
    path: PathBuf,
    created: Instant,
}

impl State {
    pub(crate) fn grant_directory(&mut self, path: PathBuf) -> AppResult<DirectoryGrant> {
        self.grants
            .retain(|_, grant| grant.created.elapsed() < GRANT_LIFETIME);
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

    /// Consumes a grant and returns its directory if it is unexpired and unchanged.
    pub(crate) fn take_grant(&mut self, grant: &GrantId) -> AppResult<PathBuf> {
        let grant = self
            .grants
            .remove(grant)
            .ok_or_else(|| AppError::stale("Directory selection expired or was already used."))?;
        if grant.created.elapsed() >= GRANT_LIFETIME {
            return Err(AppError::stale("Directory selection expired."));
        }
        let canonical = fs::canonicalize(&grant.path)
            .map_err(|_| AppError::stale("Selected directory no longer exists."))?;
        if canonical != grant.path {
            return Err(AppError::stale("Selected directory changed."));
        }
        Ok(canonical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_grant_is_refused_and_consumed() -> AppResult<()> {
        let root = tempfile::tempdir().map_err(|_| AppError::storage())?;
        let mut state = State::isolated_test_state(root.path().join("data"));
        let grant = state.grant_directory(root.path().to_owned())?;
        state
            .grants
            .get_mut(&grant.id)
            .ok_or_else(AppError::storage)?
            .created = Instant::now() - GRANT_LIFETIME;
        for _ in 0..2 {
            let result = state.take_grant(&grant.id);
            assert!(result.is_err_and(|error| error.category == ErrorCategory::Stale));
        }
        Ok(())
    }
}
