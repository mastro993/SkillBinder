//! The machine-local SQLite state behind the core ports: onboarding progress, import plans and
//! their idempotency records, source observations, indexed skill metadata, and the registered
//! project-search roots.
//!
//! This state never enters the portable library repository. The schema is created on connect, so a
//! store opened against a new path is ready to use.

use rusqlite::{Connection, OptionalExtension, params};
use skillbinder_core::{
    bootstrap::{BootstrapError, OnboardingState},
    discovery::roots::ScanRoot,
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
        connection.execute_batch("CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS device_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS operation_plans(id TEXT PRIMARY KEY, payload TEXT NOT NULL, expires_at INTEGER NOT NULL, consumed INTEGER NOT NULL DEFAULT 0); CREATE TABLE IF NOT EXISTS idempotency_records(operation_id TEXT PRIMARY KEY, request_hash TEXT NOT NULL, result TEXT NOT NULL); CREATE TABLE IF NOT EXISTS source_observations(id INTEGER PRIMARY KEY, source TEXT NOT NULL, skill_id TEXT NOT NULL, digest TEXT NOT NULL, warnings TEXT NOT NULL, reader_agents TEXT NOT NULL, observed_at INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS skill_metadata(skill_id TEXT PRIMARY KEY, description TEXT, validation TEXT NOT NULL, updated_at INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS scan_roots(id TEXT PRIMARY KEY, canonical_path TEXT NOT NULL UNIQUE, display_path TEXT NOT NULL, label TEXT NOT NULL, enabled INTEGER NOT NULL DEFAULT 1, created_at INTEGER NOT NULL); INSERT OR IGNORE INTO schema_migrations(version,applied_at) VALUES (1,datetime('now')); INSERT OR IGNORE INTO schema_migrations(version,applied_at) VALUES (2,datetime('now')); INSERT OR IGNORE INTO schema_migrations(version,applied_at) VALUES (3,datetime('now')); ")?;
        Ok(connection)
    }
    pub fn insert_scan_root(&self, root: &ScanRoot) -> Result<(), StateError> {
        self.connect()?
            .execute(
                "INSERT INTO scan_roots(id,canonical_path,display_path,label,enabled,created_at) VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    root.id,
                    root.canonical_path.to_string_lossy(),
                    root.display_path,
                    root.label,
                    root.enabled as i64,
                    root.created_at as i64,
                ],
            )
            .map_err(map_scan_root_error)?;
        Ok(())
    }
    pub fn list_scan_roots(&self) -> Result<Vec<ScanRoot>, StateError> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT id,canonical_path,display_path,label,enabled,created_at FROM scan_roots ORDER BY enabled DESC, created_at, id",
        )?;
        let rows = statement.query_map([], scan_root_from_row)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StateError::from)
    }
    pub fn update_scan_root(
        &self,
        id: &str,
        label: Option<&str>,
        enabled: bool,
    ) -> Result<Option<ScanRoot>, StateError> {
        let connection = self.connect()?;
        let changed = connection.execute(
            "UPDATE scan_roots SET label=COALESCE(?2, label), enabled=?3 WHERE id=?1",
            params![id, label, enabled as i64],
        )?;
        if changed == 0 {
            return Ok(None);
        }
        connection
            .query_row(
                "SELECT id,canonical_path,display_path,label,enabled,created_at FROM scan_roots WHERE id=?1",
                params![id],
                scan_root_from_row,
            )
            .optional()
            .map_err(StateError::from)
    }
    pub fn remove_scan_root(&self, id: &str) -> Result<bool, StateError> {
        let removed = self
            .connect()?
            .execute("DELETE FROM scan_roots WHERE id=?1", params![id])?;
        Ok(removed > 0)
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
        v.map_or(Ok(OnboardingProgress::default()), |value| {
            serde_json::from_str(&value)
                .map_err(|error| StateError::InvalidState(error.to_string()))
        })
    }
    fn save_progress(&self, p: &OnboardingProgress) -> Result<(), StateError> {
        let value =
            serde_json::to_string(p).map_err(|e| StateError::InvalidState(e.to_string()))?;
        self.connect()?.execute("INSERT INTO device_settings(key,value) VALUES('onboarding_progress',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![value])?;
        Ok(())
    }
}
fn scan_root_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScanRoot> {
    Ok(ScanRoot {
        id: row.get(0)?,
        canonical_path: PathBuf::from(row.get::<_, String>(1)?),
        display_path: row.get(2)?,
        label: row.get(3)?,
        enabled: row.get::<_, i64>(4)? != 0,
        created_at: row.get::<_, i64>(5)? as u64,
    })
}
fn map_scan_root_error(error: rusqlite::Error) -> StateError {
    if let rusqlite::Error::SqliteFailure(failure, _) = &error
        && failure.code == rusqlite::ErrorCode::ConstraintViolation
    {
        return StateError::InvalidState("a project-search root already covers that path".into());
    }
    StateError::Database(error)
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
                "DELETE FROM source_observations WHERE id = (
                     SELECT id FROM source_observations
                     WHERE source=?1 AND skill_id=?2 AND digest=?3
                     ORDER BY id DESC LIMIT 1
                 )",
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
#[cfg(test)]
mod tests {

    use super::*;
    use std::fs;

    fn temporary_store() -> (PathBuf, StateStore) {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "skillbinder-scan-roots-{}-{unique}.sqlite",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        (path.clone(), StateStore::new(path))
    }

    fn scan_root(id: &str, path: &str, enabled: bool, created_at: u64) -> ScanRoot {
        ScanRoot {
            id: id.to_owned(),
            canonical_path: PathBuf::from(path),
            display_path: path.to_owned(),
            label: id.to_owned(),
            enabled,
            created_at,
        }
    }

    #[test]
    fn scan_roots_round_trip_on_a_real_database() {
        let (database, store) = temporary_store();

        store
            .insert_scan_root(&scan_root("root-a", "/work/a", true, 2))
            .expect("insert enabled root");
        store
            .insert_scan_root(&scan_root("root-b", "/work/b", false, 1))
            .expect("insert disabled root");

        let listed = store.list_scan_roots().expect("list roots");
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, "root-a", "enabled roots sort first");
        assert_eq!(listed[1].id, "root-b");
        assert_eq!(listed[0].canonical_path, PathBuf::from("/work/a"));
        assert_eq!(listed[0].created_at, 2);

        let updated = store
            .update_scan_root("root-a", Some("Renamed"), false)
            .expect("update root")
            .expect("updated root exists");
        assert_eq!(updated.label, "Renamed");
        assert!(!updated.enabled);
        let kept = store
            .update_scan_root("root-a", None, true)
            .expect("update root without a label")
            .expect("updated root exists");
        assert_eq!(kept.label, "Renamed", "a blank label keeps the stored one");
        assert!(kept.enabled);
        assert!(
            store
                .update_scan_root("missing", Some("Nope"), true)
                .expect("update missing root")
                .is_none()
        );

        let duplicate = store.insert_scan_root(&scan_root("root-c", "/work/a", true, 3));
        assert!(
            matches!(duplicate, Err(StateError::InvalidState(_))),
            "{duplicate:?}"
        );

        assert!(store.remove_scan_root("root-b").expect("remove root"));
        assert!(!store.remove_scan_root("root-b").expect("remove root again"));
        assert_eq!(store.list_scan_roots().expect("list roots").len(), 1);

        drop(store);
        let reopened = StateStore::new(database.clone());
        let persisted = reopened.list_scan_roots().expect("reopen store");
        assert_eq!(persisted.len(), 1);
        assert_eq!(persisted[0].label, "Renamed");

        let _ = fs::remove_file(&database);
    }
}
