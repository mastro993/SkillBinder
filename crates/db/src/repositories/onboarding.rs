//! Onboarding progress, the only `device_settings` entry today.

use crate::schema::device_settings;
use diesel::SqliteConnection;
use diesel::prelude::*;
use diesel::upsert::excluded;

/// The `device_settings` key holding the serialized onboarding progress.
pub(crate) const ONBOARDING_PROGRESS: &str = "onboarding_progress";

/// The stored JSON text, or `None` when onboarding has never been saved.
pub(crate) fn read(connection: &mut SqliteConnection) -> QueryResult<Option<String>> {
    device_settings::table
        .select(device_settings::value)
        .filter(device_settings::key.eq(ONBOARDING_PROGRESS))
        .first(connection)
        .optional()
}

pub(crate) fn write(connection: &mut SqliteConnection, value: &str) -> QueryResult<()> {
    diesel::insert_into(device_settings::table)
        .values((
            device_settings::key.eq(ONBOARDING_PROGRESS),
            device_settings::value.eq(value),
        ))
        .on_conflict(device_settings::key)
        .do_update()
        .set(device_settings::value.eq(excluded(device_settings::value)))
        .execute(connection)?;
    Ok(())
}
