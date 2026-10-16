//! Machine-local SQLite through Diesel. The engine worker owns the only connection.

use crate::{persistence::now, schema::skill_metadata};
use diesel::{connection::SimpleConnection, prelude::*, upsert::excluded};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use skillbinder_proto::{AppError, AppResult, SkillDetails, SkillId};
use std::path::Path;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

/// Opens the database at `path`, creating it when absent, and applies pending migrations.
pub(crate) fn open(path: &Path) -> AppResult<SqliteConnection> {
    let path = path.to_str().ok_or_else(AppError::storage)?;
    let mut connection = SqliteConnection::establish(path).map_err(|_| AppError::storage())?;
    connection
        .batch_execute(
            "PRAGMA busy_timeout=5000; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )
        .map_err(|_| AppError::storage())?;
    connection
        .run_pending_migrations(MIGRATIONS)
        .map_err(|_| AppError::storage())?;
    Ok(connection)
}

/// The cached details of a library skill, if any.
pub(crate) fn details(
    connection: &mut SqliteConnection,
    id: &SkillId,
) -> AppResult<Option<SkillDetails>> {
    skill_metadata::table
        .find(id.as_str())
        .select(skill_metadata::validation)
        .first::<String>(connection)
        .optional()
        .map_err(|_| AppError::storage())?
        .map(|json| serde_json::from_str(&json).map_err(|_| AppError::storage()))
        .transpose()
}

/// Inserts or replaces the cached details of a library skill.
pub(crate) fn save_details(
    connection: &mut SqliteConnection,
    id: &SkillId,
    details: &SkillDetails,
) -> AppResult<()> {
    let validation = serde_json::to_string(details).map_err(|_| AppError::storage())?;
    diesel::insert_into(skill_metadata::table)
        .values((
            skill_metadata::skill_id.eq(id.as_str()),
            skill_metadata::description.eq(&details.description),
            skill_metadata::validation.eq(validation),
            skill_metadata::updated_at.eq(now() as i64),
        ))
        .on_conflict(skill_metadata::skill_id)
        .do_update()
        .set((
            skill_metadata::description.eq(excluded(skill_metadata::description)),
            skill_metadata::validation.eq(excluded(skill_metadata::validation)),
            skill_metadata::updated_at.eq(excluded(skill_metadata::updated_at)),
        ))
        .execute(connection)
        .map(drop)
        .map_err(|_| AppError::storage())
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
