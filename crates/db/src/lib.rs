//! The machine-local SQLite state behind the core ports: onboarding progress, import plans and
//! their idempotency records, source observations, indexed skill metadata, and the registered
//! project-search roots.
//!
//! This state never enters the portable library repository. The schema lives in `migrations`, which
//! run on connect, so a store opened against a new path is ready to use.

mod repositories;
mod schema;

use diesel::SqliteConnection;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};
use repositories::{
    DUPLICATE_PATH_MESSAGE, ObservationRow, observations, onboarding, plans, roots,
};
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

const MIGRATIONS: EmbeddedMigrations = diesel_migrations::embed_migrations!();

#[derive(Debug, Error)]
pub enum StateError {
    #[error("local state database is unavailable: {0}")]
    Database(#[from] diesel::result::Error),
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
    /// Opens a connection with the crate's pragmas applied and its migrations up to date.
    fn connect(&self) -> Result<SqliteConnection, StateError> {
        let mut connection = SqliteConnection::establish(&self.database.to_string_lossy())
            .map_err(database_error)?;
        connection.batch_execute(
            "PRAGMA foreign_keys = ON; \
             PRAGMA busy_timeout = 5000; \
             PRAGMA synchronous = FULL;",
        )?;
        connection
            .run_pending_migrations(MIGRATIONS)
            .map_err(database_error)?;
        Ok(connection)
    }
    pub fn insert_scan_root(&self, root: &ScanRoot) -> Result<(), StateError> {
        let mut connection = self.connect()?;
        roots::insert(&mut connection, root).map_err(map_scan_root_error)
    }
    pub fn list_scan_roots(&self) -> Result<Vec<ScanRoot>, StateError> {
        let mut connection = self.connect()?;
        roots::list(&mut connection).map_err(StateError::from)
    }
    pub fn update_scan_root(
        &self,
        id: &str,
        label: Option<&str>,
        enabled: bool,
    ) -> Result<Option<ScanRoot>, StateError> {
        let mut connection = self.connect()?;
        roots::update(&mut connection, id, label, enabled).map_err(StateError::from)
    }
    pub fn remove_scan_root(&self, id: &str) -> Result<bool, StateError> {
        let mut connection = self.connect()?;
        roots::remove(&mut connection, id).map_err(StateError::from)
    }
    pub fn onboarding_progress(&self) -> Result<OnboardingProgress, StateError> {
        let stored = onboarding::read(&mut self.connect()?)?;
        stored.map_or(Ok(OnboardingProgress::default()), |value| {
            serde_json::from_str(&value)
                .map_err(|error| StateError::InvalidState(error.to_string()))
        })
    }
    fn save_progress(&self, progress: &OnboardingProgress) -> Result<(), StateError> {
        let value =
            serde_json::to_string(progress).map_err(|e| StateError::InvalidState(e.to_string()))?;
        onboarding::write(&mut self.connect()?, &value)?;
        Ok(())
    }
}
fn map_scan_root_error(error: diesel::result::Error) -> StateError {
    match error {
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        ) => StateError::InvalidState(DUPLICATE_PATH_MESSAGE.to_owned()),
        error => StateError::Database(error),
    }
}
/// Folds a failure Diesel has no dedicated `Error` variant for (opening a connection, running
/// migrations) into `StateError::Database`, so the variant stays the single database failure.
fn database_error(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> StateError {
    StateError::Database(diesel::result::Error::QueryBuilderError(error.into()))
}
/// Folds a connect or query failure into a port error, keeping the message the ports already had.
fn import_error(error: impl std::fmt::Display) -> ImportError {
    ImportError::Database(error.to_string())
}
impl OnboardingState for StateStore {
    fn load(&self) -> Result<OnboardingProgress, BootstrapError> {
        self.onboarding_progress()
            .map_err(|e| BootstrapError::State(e.to_string()))
    }
    fn save(&self, progress: &OnboardingProgress) -> Result<(), BootstrapError> {
        self.save_progress(progress)
            .map_err(|e| BootstrapError::State(e.to_string()))
    }
}
impl PlanStore for StateStore {
    fn create(&self, plan: &ImportPlan) -> Result<(), ImportError> {
        let payload = serde_json::to_string(plan).map_err(import_error)?;
        let mut connection = self.connect().map_err(import_error)?;
        plans::create(&mut connection, &plan.id, &payload, plan.expires_at as i64)
            .map_err(import_error)
    }
    fn load(&self, id: &str) -> Result<Option<ImportPlan>, ImportError> {
        let mut connection = self.connect().map_err(import_error)?;
        plans::load(&mut connection, id)
            .map_err(import_error)?
            .map(|payload| serde_json::from_str(&payload).map_err(import_error))
            .transpose()
    }
    fn consume(&self, id: &str, _result: &ImportResult) -> Result<(), ImportError> {
        let mut connection = self.connect().map_err(import_error)?;
        plans::consume(&mut connection, id).map_err(import_error)
    }
    fn expire(&self, now: u64) -> Result<(), ImportError> {
        let mut connection = self.connect().map_err(import_error)?;
        plans::expire(&mut connection, now as i64).map_err(import_error)
    }
    fn idempotency_lookup(
        &self,
        operation_id: &str,
    ) -> Result<Option<(String, ImportResult)>, ImportError> {
        let mut connection = self.connect().map_err(import_error)?;
        match plans::idempotency_lookup(&mut connection, operation_id).map_err(import_error)? {
            Some((hash, result)) => Ok(Some((
                hash,
                serde_json::from_str(&result).map_err(import_error)?,
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
        let value = serde_json::to_string(result).map_err(import_error)?;
        let mut connection = self.connect().map_err(import_error)?;
        plans::idempotency_store(&mut connection, id, hash, &value).map_err(import_error)
    }
    fn consume_and_store(
        &self,
        id: &str,
        hash: &str,
        result: &ImportResult,
    ) -> Result<(), ImportError> {
        let value = serde_json::to_string(result).map_err(import_error)?;
        let mut connection = self.connect().map_err(import_error)?;
        plans::consume_and_store(&mut connection, id, hash, &value).map_err(import_error)
    }
}
impl ObservationStore for StateStore {
    fn append(&self, observation: &SourceObservation) -> Result<(), ImportError> {
        let mut connection = self.connect().map_err(import_error)?;
        observations::append(
            &mut connection,
            &observation.source.to_string_lossy(),
            &observation.skill_id,
            &observation.digest,
            &serde_json::to_string(&observation.warnings).unwrap_or_default(),
            &serde_json::to_string(&observation.reader_agent_ids).unwrap_or_default(),
            chrono::Utc::now().timestamp(),
        )
        .map_err(import_error)
    }
    fn rollback_observation(&self, observation: &SourceObservation) -> Result<(), ImportError> {
        let mut connection = self.connect().map_err(import_error)?;
        observations::rollback(
            &mut connection,
            &observation.source.to_string_lossy(),
            &observation.skill_id,
            &observation.digest,
        )
        .map_err(import_error)
    }
    fn list(&self, skill_id: &str) -> Result<Vec<SourceObservation>, ImportError> {
        let mut connection = self.connect().map_err(import_error)?;
        Ok(observations::list(&mut connection, skill_id)
            .map_err(import_error)?
            .into_iter()
            .map(observation_from_row)
            .collect())
    }
    fn index_metadata(
        &self,
        skill_id: &str,
        metadata: &IndexedSkillMetadata,
    ) -> Result<(), ImportError> {
        let validation = serde_json::to_string(&metadata.validation).map_err(import_error)?;
        let mut connection = self.connect().map_err(import_error)?;
        observations::index_metadata(
            &mut connection,
            skill_id,
            metadata.description.as_deref(),
            &validation,
            metadata.updated_at as i64,
        )
        .map_err(import_error)
    }
    fn indexed_metadata(
        &self,
        skill_id: &str,
    ) -> Result<Option<IndexedSkillMetadata>, ImportError> {
        let mut connection = self.connect().map_err(import_error)?;
        let stored =
            observations::indexed_metadata(&mut connection, skill_id).map_err(import_error)?;
        Ok(stored.map(
            |(description, validation, updated_at)| IndexedSkillMetadata {
                description,
                validation: match serde_json::from_str(&validation) {
                    Ok(validation) => validation,
                    Err(_) => {
                        warn_index_fallback(skill_id, "validation_summary");
                        ValidationSummary::valid()
                    }
                },
                updated_at: updated_at as u64,
            },
        ))
    }
    fn remove_index_metadata(&self, skill_id: &str) -> Result<(), ImportError> {
        let mut connection = self.connect().map_err(import_error)?;
        observations::remove_index_metadata(&mut connection, skill_id).map_err(import_error)
    }
}
fn observation_from_row(row: ObservationRow) -> SourceObservation {
    let warnings = match serde_json::from_str(&row.warnings) {
        Ok(warnings) => warnings,
        Err(_) => {
            warn_index_fallback(&row.skill_id, "observation");
            Vec::new()
        }
    };
    let reader_agent_ids = match serde_json::from_str(&row.reader_agents) {
        Ok(reader_agent_ids) => reader_agent_ids,
        Err(_) => {
            warn_index_fallback(&row.skill_id, "observation");
            Vec::new()
        }
    };
    SourceObservation {
        source: PathBuf::from(row.source),
        skill_id: row.skill_id,
        digest: row.digest,
        warnings,
        reader_agent_ids,
    }
}
/// A stored index row that cannot be read is not a reason to fail the read, but it is a reason to
/// say so: the caller sees a permissive default and the skill id is the only handle on it.
fn warn_index_fallback(skill_id: &str, fallback: &'static str) {
    tracing::warn!(
        event = "indexed_metadata_fallback",
        skill_id,
        fallback,
        "stored indexed metadata could not be read and a default was used"
    );
}

pub use repositories::bindings::{BindingActionRow, BindingReceiptRow};

impl StateStore {
    pub fn list_bindings(&self) -> Result<Vec<BindingActionRow>, StateError> {
        let mut connection = self.connect()?;
        repositories::bindings::list(&mut connection).map_err(StateError::from)
    }

    pub fn insert_binding(&self, row: &BindingActionRow) -> Result<(), StateError> {
        let mut connection = self.connect()?;
        repositories::bindings::insert(&mut connection, row).map_err(StateError::from)
    }

    pub fn binding_receipt(&self, path: &str) -> Result<Option<BindingReceiptRow>, StateError> {
        let mut connection = self.connect()?;
        repositories::bindings::receipt(&mut connection, path).map_err(StateError::from)
    }

    pub fn put_binding_receipt(&self, row: &BindingReceiptRow) -> Result<(), StateError> {
        let mut connection = self.connect()?;
        repositories::bindings::put_receipt(&mut connection, row).map_err(StateError::from)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use diesel::connection::SimpleConnection;
    use skillbinder_core::onboarding::OnboardingStep;
    use std::fs;

    /// The DDL the pre-Diesel build ran on every connect, verbatim. It is history: it stays here
    /// because it is the only way to build the kind of database an existing install already has.
    const OLD_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS device_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS operation_plans(id TEXT PRIMARY KEY, payload TEXT NOT NULL, expires_at INTEGER NOT NULL, consumed INTEGER NOT NULL DEFAULT 0); CREATE TABLE IF NOT EXISTS idempotency_records(operation_id TEXT PRIMARY KEY, request_hash TEXT NOT NULL, result TEXT NOT NULL); CREATE TABLE IF NOT EXISTS source_observations(id INTEGER PRIMARY KEY, source TEXT NOT NULL, skill_id TEXT NOT NULL, digest TEXT NOT NULL, warnings TEXT NOT NULL, reader_agents TEXT NOT NULL, observed_at INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS skill_metadata(skill_id TEXT PRIMARY KEY, description TEXT, validation TEXT NOT NULL, updated_at INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS scan_roots(id TEXT PRIMARY KEY, canonical_path TEXT NOT NULL UNIQUE, display_path TEXT NOT NULL, label TEXT NOT NULL, enabled INTEGER NOT NULL DEFAULT 1, created_at INTEGER NOT NULL); INSERT OR IGNORE INTO schema_migrations(version,applied_at) VALUES (1,datetime('now')); INSERT OR IGNORE INTO schema_migrations(version,applied_at) VALUES (2,datetime('now')); INSERT OR IGNORE INTO schema_migrations(version,applied_at) VALUES (3,datetime('now')); ";

    fn temporary_store() -> (PathBuf, StateStore) {
        // The counter keeps two tests running in the same process off each other's file: the clock
        // alone is not fine-grained enough for that.
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "skillbinder-scan-roots-{}-{unique}-{counter}.sqlite",
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
    fn overlapping_bindings_remain_independent_actions_with_one_receipt() {
        let (database, store) = temporary_store();
        for id in ["first", "second"] {
            store
                .insert_binding(&BindingActionRow {
                    id: id.into(),
                    skill_ids: "[\"review\"]".into(),
                    scope: "project".into(),
                    project_root_id: Some("project".into()),
                    agent_ids: "[\"codex\"]".into(),
                    target_paths: "[]".into(),
                    created_at: 1,
                })
                .unwrap();
        }
        store
            .put_binding_receipt(&BindingReceiptRow {
                target_path: "/project/.agents/skills/review".into(),
                skill_id: "review".into(),
                digest: "sha256:test".into(),
            })
            .unwrap();
        assert_eq!(store.list_bindings().unwrap().len(), 2);
        assert_eq!(
            store
                .binding_receipt("/project/.agents/skills/review")
                .unwrap()
                .unwrap()
                .skill_id,
            "review"
        );
        fs::remove_file(database).unwrap();
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

    #[test]
    fn a_database_written_before_the_migrations_still_opens_and_keeps_its_rows() {
        let (database, _store) = temporary_store();
        let mut legacy = SqliteConnection::establish(&database.to_string_lossy())
            .expect("open a database the old way");
        legacy.batch_execute(OLD_SCHEMA).expect("apply old DDL");
        legacy
            .batch_execute(
                "INSERT INTO scan_roots(id,canonical_path,display_path,label,enabled,created_at) \
                 VALUES('legacy','/work/legacy','/work/legacy','Legacy',1,7)",
            )
            .expect("seed old rows");
        drop(legacy);

        let store = StateStore::new(database.clone());
        let listed = store.list_scan_roots().expect("read an old database");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "legacy");
        assert_eq!(listed[0].created_at, 7);
        assert_eq!(
            store
                .onboarding_progress()
                .expect("read unwritten progress"),
            OnboardingProgress::default()
        );

        store
            .insert_scan_root(&scan_root("root-a", "/work/a", true, 2))
            .expect("write to an old database");
        store
            .save(&OnboardingProgress {
                step: OnboardingStep::Ready,
                completed: true,
            })
            .expect("save progress");

        drop(store);
        let reopened = StateStore::new(database.clone());
        assert_eq!(reopened.list_scan_roots().expect("reopen").len(), 2);
        assert_eq!(
            OnboardingState::load(&reopened).expect("load progress"),
            OnboardingProgress {
                step: OnboardingStep::Ready,
                completed: true,
            }
        );

        let _ = fs::remove_file(&database);
    }
}
