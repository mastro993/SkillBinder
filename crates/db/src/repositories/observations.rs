//! Ingest observations and the indexed skill metadata derived from them.
//!
//! `warnings` and `reader_agents` are JSON text produced by the caller, parsed back by it too.

use crate::schema::{skill_metadata, source_observations};
use diesel::SqliteConnection;
use diesel::prelude::*;
use diesel::upsert::excluded;

/// One `source_observations` row, without the id: source, skill id, digest, warnings, agents.
pub(crate) struct ObservationRow {
    pub(crate) source: String,
    pub(crate) skill_id: String,
    pub(crate) digest: String,
    pub(crate) warnings: String,
    pub(crate) reader_agents: String,
}

pub(crate) fn append(
    connection: &mut SqliteConnection,
    source: &str,
    skill_id: &str,
    digest: &str,
    warnings: &str,
    reader_agents: &str,
    observed_at: i64,
) -> QueryResult<()> {
    diesel::insert_into(source_observations::table)
        .values((
            source_observations::source.eq(source),
            source_observations::skill_id.eq(skill_id),
            source_observations::digest.eq(digest),
            source_observations::warnings.eq(warnings),
            source_observations::reader_agents.eq(reader_agents),
            source_observations::observed_at.eq(observed_at),
        ))
        .execute(connection)?;
    Ok(())
}

/// Drops every observation recorded for a skill.
pub(crate) fn delete_for_skill(
    connection: &mut SqliteConnection,
    skill_id: &str,
) -> QueryResult<()> {
    diesel::delete(source_observations::table.filter(source_observations::skill_id.eq(skill_id)))
        .execute(connection)?;
    Ok(())
}

/// Drops the newest observation matching the source, skill id and digest.
pub(crate) fn rollback(
    connection: &mut SqliteConnection,
    source: &str,
    skill_id: &str,
    digest: &str,
) -> QueryResult<()> {
    connection.transaction(|connection| {
        let newest: Option<i32> = source_observations::table
            .select(source_observations::id)
            .filter(source_observations::source.eq(source))
            .filter(source_observations::skill_id.eq(skill_id))
            .filter(source_observations::digest.eq(digest))
            .order(source_observations::id.desc())
            .first(connection)
            .optional()?;
        if let Some(newest) = newest {
            diesel::delete(source_observations::table.filter(source_observations::id.eq(newest)))
                .execute(connection)?;
        }
        Ok(())
    })
}

pub(crate) fn list(
    connection: &mut SqliteConnection,
    skill_id: &str,
) -> QueryResult<Vec<ObservationRow>> {
    let rows: Vec<(String, String, String, String, String)> = source_observations::table
        .select((
            source_observations::source,
            source_observations::skill_id,
            source_observations::digest,
            source_observations::warnings,
            source_observations::reader_agents,
        ))
        .filter(source_observations::skill_id.eq(skill_id))
        .load(connection)?;
    Ok(rows
        .into_iter()
        .map(
            |(source, skill_id, digest, warnings, reader_agents)| ObservationRow {
                source,
                skill_id,
                digest,
                warnings,
                reader_agents,
            },
        )
        .collect())
}

pub(crate) fn index_metadata(
    connection: &mut SqliteConnection,
    skill_id: &str,
    description: Option<&str>,
    validation: &str,
    updated_at: i64,
) -> QueryResult<()> {
    diesel::insert_into(skill_metadata::table)
        .values((
            skill_metadata::skill_id.eq(skill_id),
            skill_metadata::description.eq(description),
            skill_metadata::validation.eq(validation),
            skill_metadata::updated_at.eq(updated_at),
        ))
        .on_conflict(skill_metadata::skill_id)
        .do_update()
        .set((
            skill_metadata::description.eq(excluded(skill_metadata::description)),
            skill_metadata::validation.eq(excluded(skill_metadata::validation)),
            skill_metadata::updated_at.eq(excluded(skill_metadata::updated_at)),
        ))
        .execute(connection)?;
    Ok(())
}

/// `(description, validation JSON, updated_at)` for an indexed skill.
pub(crate) fn indexed_metadata(
    connection: &mut SqliteConnection,
    skill_id: &str,
) -> QueryResult<Option<(Option<String>, String, i64)>> {
    skill_metadata::table
        .select((
            skill_metadata::description,
            skill_metadata::validation,
            skill_metadata::updated_at,
        ))
        .filter(skill_metadata::skill_id.eq(skill_id))
        .first(connection)
        .optional()
}

pub(crate) fn remove_index_metadata(
    connection: &mut SqliteConnection,
    skill_id: &str,
) -> QueryResult<()> {
    diesel::delete(skill_metadata::table.filter(skill_metadata::skill_id.eq(skill_id)))
        .execute(connection)?;
    Ok(())
}
