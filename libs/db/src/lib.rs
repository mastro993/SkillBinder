use rusqlite::{Connection, OptionalExtension, params};
use skillbinder_core::{
    bootstrap::{BootstrapError, OnboardingState},
    onboarding::OnboardingProgress,
};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("local state database is unavailable: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("saved onboarding state is invalid: {0}")]
    InvalidState(String),
}

#[derive(Debug, Clone)]
pub struct StateStore {
    database: PathBuf,
}

impl StateStore {
    pub fn new(database: PathBuf) -> Self {
        Self { database }
    }

    fn connect(&self) -> Result<Connection, StateError> {
        let connection = Connection::open(&self.database)?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "busy_timeout", 5_000)?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                 version INTEGER PRIMARY KEY,
                 applied_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS device_settings (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             INSERT OR IGNORE INTO schema_migrations(version, applied_at)
             VALUES (1, datetime('now'));",
        )?;
        Ok(connection)
    }

    pub fn onboarding_progress(&self) -> Result<OnboardingProgress, StateError> {
        let connection = self.connect()?;
        let value: Option<String> = connection
            .query_row(
                "SELECT value FROM device_settings WHERE key = 'onboarding_progress'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        value
            .map(|json| {
                serde_json::from_str(&json)
                    .map_err(|error| StateError::InvalidState(error.to_string()))
            })
            .transpose()
            .map(|progress| progress.unwrap_or_default())
    }

    fn save_progress(&self, progress: &OnboardingProgress) -> Result<(), StateError> {
        let value = serde_json::to_string(progress)
            .map_err(|error| StateError::InvalidState(error.to_string()))?;
        self.connect()?.execute(
            "INSERT INTO device_settings(key, value) VALUES ('onboarding_progress', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![value],
        )?;
        Ok(())
    }
}

impl OnboardingState for StateStore {
    fn load(&self) -> Result<OnboardingProgress, BootstrapError> {
        self.onboarding_progress()
            .map_err(|error| BootstrapError::State(error.to_string()))
    }

    fn save(&self, progress: &OnboardingProgress) -> Result<(), BootstrapError> {
        self.save_progress(progress)
            .map_err(|error| BootstrapError::State(error.to_string()))
    }
}
