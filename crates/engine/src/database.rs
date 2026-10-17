//! Machine-local SQLite through Diesel. The engine worker owns the only connection.

use diesel::{connection::SimpleConnection, prelude::*};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use skillbinder_proto::{AppError, AppResult};
use std::path::Path;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

/// Opens the database at the absolute `path`, creating it when absent, and applies pending
/// migrations. The path reaches SQLite as a percent-encoded `file:` URI, so its bytes arrive
/// unchanged even when they are not UTF-8.
pub(crate) fn open(path: &Path) -> AppResult<SqliteConnection> {
    let uri = url::Url::from_file_path(path).map_err(|()| AppError::storage())?;
    let mut connection =
        SqliteConnection::establish(uri.as_str()).map_err(|_| AppError::storage())?;
    connection
        .batch_execute(
            "PRAGMA busy_timeout=5000; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; \
             PRAGMA foreign_keys=ON;",
        )
        .map_err(|_| AppError::storage())?;
    connection
        .run_pending_migrations(MIGRATIONS)
        .map_err(|_| AppError::storage())?;
    Ok(connection)
}

/// Runs `body` in a transaction that commits only when `body` succeeds.
pub(crate) fn transaction<T>(
    connection: &mut SqliteConnection,
    body: impl FnOnce(&mut SqliteConnection) -> AppResult<T>,
) -> AppResult<T> {
    struct Failure(AppError);
    impl From<diesel::result::Error> for Failure {
        fn from(_: diesel::result::Error) -> Self {
            Self(AppError::storage())
        }
    }
    connection
        .transaction(|connection| body(connection).map_err(Failure))
        .map_err(|Failure(error)| error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opens_at(path: &Path) -> AppResult<()> {
        let mut connection = open(path)?;
        connection
            .batch_execute("INSERT INTO device_settings(key, value) VALUES('probe', '1');")
            .map_err(|_| AppError::storage())?;
        if path.is_file() {
            Ok(())
        } else {
            Err(AppError::storage())
        }
    }

    #[test]
    fn opens_paths_with_uri_delimiters() -> AppResult<()> {
        let root = tempfile::tempdir().map_err(|_| AppError::storage())?;
        opens_at(&root.path().join("a %20?b#c.db"))
    }

    /// Databases that already applied the initial migration, as released builds left them,
    /// must upgrade in place, so applied migrations are never edited.
    #[test]
    fn upgrades_a_database_from_the_initial_schema() -> AppResult<()> {
        let root = tempfile::tempdir().map_err(|_| AppError::storage())?;
        let path = root.path().join("state.db");
        let mut initial = SqliteConnection::establish(path.to_str().ok_or_else(AppError::storage)?)
            .map_err(|_| AppError::storage())?;
        initial
            .run_next_migration(MIGRATIONS)
            .map_err(|_| AppError::storage())?;
        initial
            .batch_execute(
                "INSERT INTO device_settings(key, value) VALUES('kept', '1'); \
                 INSERT INTO skill_metadata VALUES('old', '', 'Valid', 0);",
            )
            .map_err(|_| AppError::storage())?;
        drop(initial);
        let mut upgraded = open(&path)?;
        let kept = crate::schema::device_settings::table
            .count()
            .get_result::<i64>(&mut upgraded)
            .map_err(|_| AppError::storage())?;
        let skills = crate::schema::skills::table
            .count()
            .get_result::<i64>(&mut upgraded)
            .map_err(|_| AppError::storage())?;
        if (kept, skills) == (1, 0) {
            Ok(())
        } else {
            Err(AppError::storage())
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn opens_paths_that_are_not_utf8() -> AppResult<()> {
        use std::os::unix::ffi::OsStrExt;
        let root = tempfile::tempdir().map_err(|_| AppError::storage())?;
        opens_at(
            &root
                .path()
                .join(std::ffi::OsStr::from_bytes(b"data-\xff.db")),
        )
    }
}
