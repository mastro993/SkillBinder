use crate::{
    database,
    library_store::{SkillRecord, save_skill},
    schema::{device_settings, idempotency_records, journals, operation_plans},
    worker::State,
};
use diesel::{prelude::*, upsert::excluded};
use serde::{Serialize, de::DeserializeOwned};
use skillbinder_proto::{AppError, AppResult, ErrorCategory, ImportOutcome};
#[cfg(unix)]
use std::fs::File;
use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

pub(crate) fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
pub(crate) fn writable(path: &Path) -> bool {
    let probe = path.join(format!(".probe-{}", uuid::Uuid::new_v4()));
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe)
        .is_ok_and(|file| {
            let success = file.sync_all().is_ok();
            drop(file);
            let _ = fs::remove_file(probe);
            success
        })
}
pub(crate) fn sync_directory(path: &Path) -> AppResult<()> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| AppError::storage())?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
impl State {
    pub(crate) fn setting<T: DeserializeOwned>(&self, key: &str) -> AppResult<Option<T>> {
        device_settings::table
            .find(key)
            .select(device_settings::value)
            .first::<String>(&mut *self.database.borrow_mut())
            .optional()
            .map_err(|_| AppError::storage())?
            .map(|json| serde_json::from_str(&json).map_err(|_| AppError::storage()))
            .transpose()
    }
    pub(crate) fn set_setting(&self, key: &str, value: &impl Serialize) -> AppResult<()> {
        let value = serde_json::to_string(value).map_err(|_| AppError::storage())?;
        put_setting(&mut self.database.borrow_mut(), key, &value)
    }
    pub(crate) fn library_dir(&self) -> PathBuf {
        self.config.data_dir.join("library")
    }
    pub(crate) fn require_recovered(&self) -> AppResult<()> {
        if self.recovery_required {
            Err(AppError::new(
                ErrorCategory::Recovery,
                "An interrupted library operation needs recovery.",
                "Restart SkillBinder before changing the library.",
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Move {
    pub from: PathBuf,
    pub to: PathBuf,
}
/// A payload operation recorded in the database until its database change commits. Deleting
/// the row is the commit marker, in the same transaction as the change it describes.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Journal {
    pub id: String,
    pub moves: Vec<Move>,
    pub completed_moves: usize,
    pub progress: JournalProgress,
    /// Skills the commit saves.
    pub skills: Vec<SkillRecord>,
    pub outcome: Option<ImportOutcome>,
}
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct JournalProgress {
    phase: Phase,
    active_move: Option<usize>,
    expected: Vec<ExpectedPayload>,
}
#[derive(Default, PartialEq, serde::Serialize, serde::Deserialize)]
enum Phase {
    #[default]
    Applying,
    RollingBack,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct ExpectedPayload {
    identity: crate::payload::SourceIdentity,
    digest: String,
}
impl ExpectedPayload {
    fn capture(path: &Path) -> AppResult<Self> {
        let inspection = crate::payload::inspect(path)?;
        #[cfg(not(unix))]
        if inspection.identity.created_nanos.is_none() {
            return Err(uncertain_payload());
        }
        Ok(Self {
            identity: inspection.identity,
            digest: inspection.manifest.digest,
        })
    }
    fn matches(&self, path: &Path) -> AppResult<bool> {
        let mut actual = Self::capture(path)?;
        actual
            .identity
            .canonical_path
            .clone_from(&self.identity.canonical_path);
        Ok(actual.identity == self.identity && actual.digest == self.digest)
    }
}
fn uncertain_payload() -> AppError {
    AppError::new(
        ErrorCategory::Recovery,
        "A journaled payload has an unexpected identity or location.",
        "Keep the journal and payloads for recovery before changing the library.",
    )
}
impl State {
    fn save_journal(&self, journal: &Journal) -> AppResult<()> {
        let body = serde_json::to_string(journal).map_err(|_| AppError::storage())?;
        diesel::insert_into(journals::table)
            .values((journals::id.eq(&journal.id), journals::body.eq(&body)))
            .on_conflict(journals::id)
            .do_update()
            .set(journals::body.eq(excluded(journals::body)))
            .execute(&mut *self.database.borrow_mut())
            .map(drop)
            .map_err(|_| AppError::storage())
    }
    fn remove_journal(&self, journal: &Journal) -> AppResult<()> {
        diesel::delete(journals::table.find(&journal.id))
            .execute(&mut *self.database.borrow_mut())
            .map(drop)
            .map_err(|_| AppError::storage())
    }
    /// Whether the journal's database change has committed, which removed its row.
    fn journal_committed(&self, journal: &Journal) -> AppResult<bool> {
        journals::table
            .find(&journal.id)
            .count()
            .get_result::<i64>(&mut *self.database.borrow_mut())
            .map(|rows| rows == 0)
            .map_err(|_| AppError::storage())
    }
    pub(crate) fn recover(&mut self) -> AppResult<()> {
        self.recovery_required = true;
        let bodies = journals::table
            .order(journals::id)
            .select(journals::body)
            .load::<String>(&mut *self.database.borrow_mut())
            .map_err(|_| AppError::storage())?;
        for body in bodies {
            let mut journal: Journal =
                serde_json::from_str(&body).map_err(|_| AppError::storage())?;
            self.validate_progress(&journal)?;
            if journal.progress.phase == Phase::RollingBack {
                self.rollback_journal(&mut journal)?;
            } else {
                self.finish_journal(&mut journal)?;
            }
        }
        self.recovery_required = false;
        Ok(())
    }
    pub(crate) fn commit(&mut self, mut journal: Journal) -> AppResult<()> {
        self.require_recovered()?;
        if journal.completed_moves != 0
            || journal.progress.active_move.is_some()
            || journal.progress.phase != Phase::Applying
        {
            return Err(uncertain_payload());
        }
        journal.progress.expected = journal
            .moves
            .iter()
            .map(|movement| {
                self.validate_move(movement)?;
                ExpectedPayload::capture(&movement.from)
            })
            .collect::<AppResult<_>>()?;
        // A write may succeed before its sync reports an error.
        self.recovery_required = true;
        self.save_journal(&journal)?;
        if let Err(error) = self.finish_journal(&mut journal) {
            self.log.record(format!(
                "{}: mutation failed; recovery required",
                error.diagnostic_id
            ));
            if !self.journal_committed(&journal)? {
                journal.progress.phase = Phase::RollingBack;
                if self.save_journal(&journal).is_ok()
                    && self.rollback_journal(&mut journal).is_ok()
                {
                    self.recovery_required = false;
                }
            }
            return Err(error);
        }
        self.recovery_required = false;
        Ok(())
    }
    fn validate_progress(&self, journal: &Journal) -> AppResult<()> {
        if journal.progress.expected.len() != journal.moves.len()
            || journal.completed_moves > journal.moves.len()
        {
            return Err(uncertain_payload());
        }
        if let Some(active) = journal.progress.active_move {
            let valid = active < journal.moves.len()
                && match journal.progress.phase {
                    Phase::Applying => active == journal.completed_moves,
                    Phase::RollingBack => {
                        active == journal.completed_moves
                            || Some(active) == journal.completed_moves.checked_sub(1)
                    }
                };
            if !valid {
                return Err(uncertain_payload());
            }
        }
        Ok(())
    }
    fn move_payload(&self, journal: &Journal, index: usize, reverse: bool) -> AppResult<()> {
        let movement = &journal.moves[index];
        self.validate_move(movement)?;
        let (from, to) = if reverse {
            (&movement.to, &movement.from)
        } else {
            (&movement.from, &movement.to)
        };
        let exists = |path: &Path| -> AppResult<bool> {
            match fs::symlink_metadata(path) {
                Ok(_) => Ok(true),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(_) => Err(AppError::storage()),
            }
        };
        let expected = &journal.progress.expected[index];
        match (exists(from)?, exists(to)?) {
            (true, false) if expected.matches(from)? => {
                let parent = to.parent().ok_or_else(AppError::storage)?;
                fs::create_dir_all(parent).map_err(|_| AppError::storage())?;
                let mut ancestor = Some(parent);
                while let Some(directory) =
                    ancestor.filter(|directory| directory.starts_with(&self.config.data_dir))
                {
                    sync_directory(directory)?;
                    ancestor = directory.parent();
                }
                checkpoint("before_rename")?;
                fs::rename(from, to).map_err(|_| AppError::storage())?;
                checkpoint("after_rename")?;
            }
            (false, true) if expected.matches(to)? => {}
            _ => return Err(uncertain_payload()),
        }
        sync_directory(from.parent().ok_or_else(AppError::storage)?)?;
        sync_directory(to.parent().ok_or_else(AppError::storage)?)?;
        Ok(())
    }
    fn finish_journal(&mut self, journal: &mut Journal) -> AppResult<()> {
        self.validate_progress(journal)?;
        while journal.completed_moves < journal.moves.len() {
            let index = journal.completed_moves;
            journal.progress.active_move = Some(index);
            self.save_journal(journal)?;
            self.move_payload(journal, index, false)?;
            checkpoint("before_progress")?;
            journal.completed_moves += 1;
            journal.progress.active_move = None;
            self.save_journal(journal)?;
            checkpoint("after_progress")?;
        }
        for (movement, expected) in journal.moves.iter().zip(&journal.progress.expected) {
            if !expected.matches(&movement.to)? {
                return Err(uncertain_payload());
            }
        }
        database::transaction(&mut self.database.borrow_mut(), |connection| {
            for record in &journal.skills {
                save_skill(connection, record)?;
            }
            if let Some(outcome) = &journal.outcome {
                record_outcome(connection, outcome)?;
            }
            diesel::delete(journals::table.find(&journal.id))
                .execute(connection)
                .map_err(|_| AppError::storage())?;
            checkpoint("before_database")
        })?;
        checkpoint("after_database")
    }
    fn rollback_journal(&self, journal: &mut Journal) -> AppResult<()> {
        if self.journal_committed(journal)? {
            return Err(uncertain_payload());
        }
        self.validate_progress(journal)?;
        self.rollback_unrecorded_move(journal)?;
        while journal.completed_moves > 0 {
            let index = journal.completed_moves - 1;
            journal.progress.active_move = Some(index);
            self.save_journal(journal)?;
            checkpoint("before_rollback")?;
            self.move_payload(journal, index, true)?;
            checkpoint("after_rollback")?;
            journal.completed_moves -= 1;
            journal.progress.active_move = None;
            self.save_journal(journal)?;
        }
        self.remove_journal(journal)
    }
    fn rollback_unrecorded_move(&self, journal: &mut Journal) -> AppResult<()> {
        if journal.progress.active_move == Some(journal.completed_moves) {
            self.move_payload(journal, journal.completed_moves, true)?;
            journal.progress.active_move = None;
            self.save_journal(journal)?;
        }
        Ok(())
    }
    fn validate_move(&self, movement: &Move) -> AppResult<()> {
        for path in [&movement.from, &movement.to] {
            let owned = [
                self.config.data_dir.join("staging"),
                self.library_dir().join("skills"),
            ];
            if !owned
                .iter()
                .any(|root| path.starts_with(root) && path != root)
                || path
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
            {
                return Err(AppError::validation(
                    "Journal path leaves application storage.",
                ));
            }
            let mut parent = path.as_path();
            while !parent.exists() {
                parent = parent.parent().ok_or_else(AppError::storage)?;
            }
            let canonical = fs::canonicalize(parent).map_err(|_| AppError::storage())?;
            if !canonical.starts_with(&self.config.data_dir) || canonical != parent {
                return Err(AppError::validation(
                    "Journal path crosses a symbolic link outside application storage.",
                ));
            }
        }
        Ok(())
    }
}

/// Inserts or replaces a device setting holding serialized JSON.
fn put_setting(connection: &mut SqliteConnection, key: &str, value: &str) -> AppResult<()> {
    diesel::insert_into(device_settings::table)
        .values((
            device_settings::key.eq(key),
            device_settings::value.eq(value),
        ))
        .on_conflict(device_settings::key)
        .do_update()
        .set(device_settings::value.eq(excluded(device_settings::value)))
        .execute(connection)
        .map(drop)
        .map_err(|_| AppError::storage())
}

/// Stores a completed import's outcome for replay and consumes its plan.
fn record_outcome(connection: &mut SqliteConnection, outcome: &ImportOutcome) -> AppResult<()> {
    let serialized = serde_json::to_string(outcome).map_err(|_| AppError::storage())?;
    let plan = outcome.plan_id.as_str();
    diesel::insert_or_ignore_into(idempotency_records::table)
        .values((
            idempotency_records::operation_id.eq(plan),
            idempotency_records::request_hash.eq(plan),
            idempotency_records::result.eq(&serialized),
        ))
        .execute(connection)
        .map_err(|_| AppError::storage())?;
    diesel::update(operation_plans::table.find(plan))
        .set(operation_plans::consumed.eq(1))
        .execute(connection)
        .map_err(|_| AppError::storage())?;
    put_setting(connection, "import_outcome", &serialized)
}

#[cfg(not(test))]
pub(crate) fn checkpoint(_: &str) -> AppResult<()> {
    Ok(())
}
#[cfg(test)]
thread_local! { pub(crate) static FAULTS: std::cell::RefCell<std::collections::VecDeque<(&'static str, bool)>> = const { std::cell::RefCell::new(std::collections::VecDeque::new()) }; }
#[cfg(test)]
pub(crate) fn checkpoint(name: &str) -> AppResult<()> {
    let fault = FAULTS.with(|faults| {
        let mut faults = faults.borrow_mut();
        if faults.front().is_some_and(|(point, _)| *point == name) {
            faults.pop_front()
        } else {
            None
        }
    });
    match fault {
        Some((_, true)) => panic!("simulated crash at {name}"),
        Some((_, false)) => Err(AppError::storage()),
        None => Ok(()),
    }
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod tests;
