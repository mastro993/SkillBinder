use rusqlite::{Connection, OptionalExtension, params};
use skillbinder_core::{
    bootstrap::{BootstrapError, OnboardingState},
    import::{
        ImportError, ImportPlan, ImportResult, IndexedSkillMetadata, ObservationStore, PlanStore,
        SourceObservation,
    },
    library::ValidationSummary,
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
        connection.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS device_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS operation_plans(id TEXT PRIMARY KEY, payload TEXT NOT NULL, expires_at INTEGER NOT NULL, consumed INTEGER NOT NULL DEFAULT 0); CREATE TABLE IF NOT EXISTS idempotency_records(operation_id TEXT PRIMARY KEY, request_hash TEXT NOT NULL, result TEXT NOT NULL); CREATE TABLE IF NOT EXISTS source_observations(id INTEGER PRIMARY KEY, source TEXT NOT NULL, skill_id TEXT NOT NULL, digest TEXT NOT NULL, warnings TEXT NOT NULL, reader_agents TEXT NOT NULL, observed_at INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS skill_metadata(skill_id TEXT PRIMARY KEY, description TEXT, validation TEXT NOT NULL, updated_at INTEGER NOT NULL); INSERT OR IGNORE INTO schema_migrations(version,applied_at) VALUES (1,datetime('now')); INSERT OR IGNORE INTO schema_migrations(version,applied_at) VALUES (2,datetime('now')); ")?;
        Ok(connection)
    }
    pub fn onboarding_progress(&self) -> Result<OnboardingProgress, StateError> {
        let c = self.connect()?;
        let v: Option<String> = c
            .query_row(
                "SELECT value FROM device_settings WHERE key='onboarding_progress'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        v.map(|x| serde_json::from_str(&x).map_err(|e| StateError::InvalidState(e.to_string())))
            .transpose()
            .map(|x| x.unwrap_or_default())
    }
    fn save_progress(&self, p: &OnboardingProgress) -> Result<(), StateError> {
        let value =
            serde_json::to_string(p).map_err(|e| StateError::InvalidState(e.to_string()))?;
        self.connect()?.execute("INSERT INTO device_settings(key,value) VALUES('onboarding_progress',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![value])?;
        Ok(())
    }
    pub fn indexed_metadata(
        &self,
        skill_id: &str,
    ) -> Result<Option<IndexedSkillMetadata>, ImportError> {
        let c = self
            .connect()
            .map_err(|error| ImportError::Database(error.to_string()))?;
        c.query_row(
            "SELECT description,validation,updated_at FROM skill_metadata WHERE skill_id=?1",
            params![skill_id],
            |row| {
                Ok(IndexedSkillMetadata {
                    description: row.get(0)?,
                    validation: serde_json::from_str(&row.get::<_, String>(1)?).map_err(
                        |error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        },
                    )?,
                    updated_at: row.get::<_, i64>(2)? as u64,
                })
            },
        )
        .optional()
        .map_err(|error| ImportError::Database(error.to_string()))
    }
}
impl OnboardingState for StateStore {
    fn load(&self) -> Result<OnboardingProgress, BootstrapError> {
        self.onboarding_progress()
            .map_err(|e| BootstrapError::State(e.to_string()))
    }
    fn save(&self, p: &OnboardingProgress) -> Result<(), BootstrapError> {
        self.save_progress(p)
            .map_err(|e| BootstrapError::State(e.to_string()))
    }
}
impl PlanStore for StateStore {
    fn create(&self, plan: &ImportPlan) -> Result<(), ImportError> {
        self.connect()
            .map_err(|e| ImportError::Database(e.to_string()))?
            .execute(
                "INSERT INTO operation_plans(id,payload,expires_at) VALUES(?1,?2,?3)",
                params![
                    plan.id,
                    serde_json::to_string(plan)
                        .map_err(|e| ImportError::Database(e.to_string()))?,
                    plan.expires_at as i64
                ],
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(())
    }
    fn load(&self, id: &str) -> Result<Option<ImportPlan>, ImportError> {
        let c = self
            .connect()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        c.query_row(
            "SELECT payload FROM operation_plans WHERE id=?1 AND consumed=0",
            params![id],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| ImportError::Database(e.to_string()))?
        .map(|v| serde_json::from_str(&v).map_err(|e| ImportError::Database(e.to_string())))
        .transpose()
    }
    fn consume(&self, id: &str, _result: &ImportResult) -> Result<(), ImportError> {
        self.connect()
            .map_err(|e| ImportError::Database(e.to_string()))?
            .execute(
                "UPDATE operation_plans SET consumed=1 WHERE id=?1 AND consumed=0",
                params![id],
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(())
    }
    fn expire(&self, now: u64) -> Result<(), ImportError> {
        self.connect()
            .map_err(|e| ImportError::Database(e.to_string()))?
            .execute(
                "DELETE FROM operation_plans WHERE expires_at < ?1 AND consumed=0",
                params![now as i64],
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(())
    }
    fn idempotency_lookup(
        &self,
        operation_id: &str,
    ) -> Result<Option<(String, ImportResult)>, ImportError> {
        let c = self
            .connect()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let row: Option<(String, String)> = c
            .query_row(
                "SELECT request_hash,result FROM idempotency_records WHERE operation_id=?1",
                params![operation_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        match row {
            Some((hash, value)) => Ok(Some((
                hash,
                serde_json::from_str(&value).map_err(|e| ImportError::Database(e.to_string()))?,
            ))),
            None => Ok(None),
        }
    }
    fn idempotency_store(
        &self,
        id: &str,
        hash: &str,
        result: &ImportResult,
    ) -> Result<(), ImportError> {
        self.connect().map_err(|e|ImportError::Database(e.to_string()))?.execute("INSERT INTO idempotency_records(operation_id,request_hash,result) VALUES(?1,?2,?3) ON CONFLICT(operation_id) DO NOTHING",params![id,hash,serde_json::to_string(result).map_err(|e|ImportError::Database(e.to_string()))?]).map_err(|e|ImportError::Database(e.to_string()))?;
        Ok(())
    }
    fn consume_and_store(
        &self,
        id: &str,
        hash: &str,
        result: &ImportResult,
    ) -> Result<(), ImportError> {
        let mut connection = self
            .connect()
            .map_err(|error| ImportError::Database(error.to_string()))?;
        let transaction = connection
            .transaction()
            .map_err(|error| ImportError::Database(error.to_string()))?;
        transaction
            .execute(
                "UPDATE operation_plans SET consumed=1 WHERE id=?1 AND consumed=0",
                params![id],
            )
            .map_err(|error| ImportError::Database(error.to_string()))?;
        transaction
            .execute(
                "INSERT INTO idempotency_records(operation_id,request_hash,result) VALUES(?1,?2,?3) ON CONFLICT(operation_id) DO NOTHING",
                params![
                    id,
                    hash,
                    serde_json::to_string(result)
                        .map_err(|error| ImportError::Database(error.to_string()))?
                ],
            )
            .map_err(|error| ImportError::Database(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| ImportError::Database(error.to_string()))?;
        Ok(())
    }
}
impl ObservationStore for StateStore {
    fn append(&self, o: &SourceObservation) -> Result<(), ImportError> {
        self.connect().map_err(|e|ImportError::Database(e.to_string()))?.execute("INSERT INTO source_observations(source,skill_id,digest,warnings,reader_agents,observed_at) VALUES(?1,?2,?3,?4,?5,?6)",params![o.source.to_string_lossy(),o.skill_id,o.digest,serde_json::to_string(&o.warnings).unwrap_or_default(),serde_json::to_string(&o.reader_agent_ids).unwrap_or_default(),chrono::Utc::now().timestamp()]).map_err(|e|ImportError::Database(e.to_string()))?;
        Ok(())
    }
    fn rollback_observation(&self, o: &SourceObservation) -> Result<(), ImportError> {
        self.connect()
            .map_err(|e| ImportError::Database(e.to_string()))?
            .execute(
                "DELETE FROM source_observations WHERE source=?1 AND skill_id=?2 AND digest=?3",
                params![o.source.to_string_lossy(), o.skill_id, o.digest],
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(())
    }
    fn list(&self, skill_id: &str) -> Result<Vec<SourceObservation>, ImportError> {
        let c = self
            .connect()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let mut statement = c
            .prepare(
                "SELECT source,skill_id,digest,warnings,reader_agents FROM source_observations WHERE skill_id=?1",
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let rows = statement
            .query_map(params![skill_id], |r| {
                Ok(SourceObservation {
                    source: PathBuf::from(r.get::<_, String>(0)?),
                    skill_id: r.get(1)?,
                    digest: r.get(2)?,
                    warnings: serde_json::from_str(&r.get::<_, String>(3)?).unwrap_or_default(),
                    reader_agent_ids: serde_json::from_str(&r.get::<_, String>(4)?)
                        .unwrap_or_default(),
                })
            })
            .map_err(|e| ImportError::Database(e.to_string()))?;
        rows.map(|r| r.map_err(|e| ImportError::Database(e.to_string())))
            .collect()
    }
    fn index_metadata(
        &self,
        skill_id: &str,
        metadata: &IndexedSkillMetadata,
    ) -> Result<(), ImportError> {
        self.connect()
            .map_err(|error| ImportError::Database(error.to_string()))?
            .execute(
                "INSERT INTO skill_metadata(skill_id,description,validation,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(skill_id) DO UPDATE SET description=excluded.description,validation=excluded.validation,updated_at=excluded.updated_at",
                params![
                    skill_id,
                    metadata.description,
                    serde_json::to_string(&metadata.validation)
                        .map_err(|error| ImportError::Database(error.to_string()))?,
                    metadata.updated_at as i64,
                ],
            )
            .map_err(|error| ImportError::Database(error.to_string()))?;
        Ok(())
    }

    fn indexed_metadata(
        &self,
        skill_id: &str,
    ) -> Result<Option<IndexedSkillMetadata>, ImportError> {
        self.connect()
            .map_err(|error| ImportError::Database(error.to_string()))?
            .query_row(
                "SELECT description,validation,updated_at FROM skill_metadata WHERE skill_id=?1",
                params![skill_id],
                |row| {
                    Ok(IndexedSkillMetadata {
                        description: row.get(0)?,
                        validation: serde_json::from_str(&row.get::<_, String>(1)?)
                            .unwrap_or_else(|_| ValidationSummary::valid()),
                        updated_at: row.get::<_, i64>(2)? as u64,
                    })
                },
            )
            .optional()
            .map_err(|error| ImportError::Database(error.to_string()))
    }
    fn remove_index_metadata(&self, skill_id: &str) -> Result<(), ImportError> {
        self.connect()
            .map_err(|error| ImportError::Database(error.to_string()))?
            .execute(
                "DELETE FROM skill_metadata WHERE skill_id=?1",
                params![skill_id],
            )
            .map_err(|error| ImportError::Database(error.to_string()))?;
        Ok(())
    }
}
