use crate::worker::State;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};
use skillbinder_proto::{
    AppError, AppResult, ErrorCategory, ImportOutcome, Library, SkillDetails, SkillId,
};
#[cfg(unix)]
use std::fs::File;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) fn open(path: &Path) -> AppResult<Connection> {
    let connection = Connection::open(path).map_err(|_| AppError::storage())?;
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
        CREATE TABLE IF NOT EXISTS device_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS operation_plans(id TEXT PRIMARY KEY,payload TEXT NOT NULL,expires_at INTEGER NOT NULL,consumed INTEGER NOT NULL DEFAULT 0);
        CREATE TABLE IF NOT EXISTS idempotency_records(operation_id TEXT PRIMARY KEY,request_hash TEXT NOT NULL,result TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS source_observations(id TEXT PRIMARY KEY,source TEXT NOT NULL,skill_id TEXT NOT NULL,digest TEXT NOT NULL,warnings TEXT NOT NULL,reader_agents TEXT NOT NULL,observed_at INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS skill_metadata(skill_id TEXT PRIMARY KEY,description TEXT NOT NULL,validation TEXT NOT NULL,updated_at INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS scan_roots(id TEXT PRIMARY KEY,canonical_path TEXT NOT NULL UNIQUE,display_path TEXT NOT NULL,label TEXT NOT NULL,enabled INTEGER NOT NULL,created_at INTEGER NOT NULL);") .map_err(|_| AppError::storage())?;
    Ok(connection)
}
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
pub(crate) fn write_json(path: &Path, value: &impl Serialize) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| AppError::storage())?;
    atomic_write(path, &bytes)
}
pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let parent = path.parent().ok_or_else(AppError::storage)?;
    fs::create_dir_all(parent).map_err(|_| AppError::storage())?;
    let temp = parent.join(format!(".write-{}", uuid::Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp)
        .map_err(|_| AppError::storage())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| AppError::storage())?;
    drop(file);
    fs::rename(&temp, path).map_err(|_| AppError::storage())?;
    sync_directory(parent)
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
        self.database
            .query_row(
                "SELECT value FROM device_settings WHERE key=?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| AppError::storage())?
            .map(|json| serde_json::from_str(&json).map_err(|_| AppError::storage()))
            .transpose()
    }
    pub(crate) fn set_setting(&self, key: &str, value: &impl Serialize) -> AppResult<()> {
        self.database.execute("INSERT INTO device_settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, serde_json::to_string(value).map_err(|_| AppError::storage())?]).map_err(|_| AppError::storage())?;
        Ok(())
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
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Journal {
    pub id: String,
    pub before: Library,
    pub after: Library,
    pub moves: Vec<Move>,
    #[serde(default)]
    pub completed_moves: usize,
    #[serde(default)]
    pub progress: JournalProgress,
    pub details: Vec<(SkillId, SkillDetails)>,
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
    fn journal_path(&self, journal: &Journal) -> AppResult<PathBuf> {
        if journal.id.is_empty()
            || !journal
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(uncertain_payload());
        }
        Ok(self
            .config
            .data_dir
            .join("journals")
            .join(format!("{}.json", journal.id)))
    }
    fn save_journal(&self, journal: &Journal) -> AppResult<()> {
        write_json(&self.journal_path(journal)?, journal)
    }
    fn remove_journal(&self, journal: &Journal) -> AppResult<()> {
        fs::remove_file(self.journal_path(journal)?).map_err(|_| AppError::storage())?;
        sync_directory(&self.config.data_dir.join("journals"))
    }
    fn journal_committed(&self, journal: &Journal) -> AppResult<bool> {
        Ok(self
            .setting::<bool>(&format!("journal:{}", journal.id))?
            .unwrap_or(false))
    }
    pub(crate) fn recover(&mut self) -> AppResult<()> {
        self.recovery_required = true;
        let directory = self.config.data_dir.join("journals");
        for entry in fs::read_dir(directory).map_err(|_| AppError::storage())? {
            let path = entry.map_err(|_| AppError::storage())?.path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let mut journal: Journal =
                serde_json::from_slice(&fs::read(&path).map_err(|_| AppError::storage())?)
                    .map_err(|_| AppError::storage())?;
            if self.journal_path(&journal)? != path {
                return Err(uncertain_payload());
            }
            self.validate_progress(&journal)?;
            if journal.progress.phase == Phase::RollingBack {
                self.rollback_journal(&mut journal)?;
            } else {
                self.finish_journal(&mut journal)?;
            }
            self.remove_journal(&journal)?;
        }
        self.recovery_required = false;
        Ok(())
    }
    pub(crate) fn commit(&mut self, mut journal: Journal) -> AppResult<()> {
        self.require_recovered()?;
        self.journal_path(&journal)?;
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
        if self.read_library()? != journal.before {
            return Err(AppError::stale(
                "The library changed before the mutation could commit.",
            ));
        }
        // Atomic replacement may succeed before its directory sync reports an error.
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
                    self.remove_journal(&journal)?;
                    self.recovery_required = false;
                }
            }
            return Err(error);
        }
        self.remove_journal(&journal)?;
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
        self.require_journal_metadata(journal)?;
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
        checkpoint("before_metadata")?;
        self.require_journal_metadata(journal)?;
        write_json(
            &self.library_dir().join(".skillbinder.json"),
            &journal.after,
        )?;
        checkpoint("after_metadata")?;
        let transaction = self
            .database
            .transaction()
            .map_err(|_| AppError::storage())?;
        for (id, details) in &journal.details {
            transaction.execute("INSERT INTO skill_metadata(skill_id,description,validation,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(skill_id) DO UPDATE SET description=excluded.description,validation=excluded.validation,updated_at=excluded.updated_at", params![id.as_str(), details.description, serde_json::to_string(details).map_err(|_| AppError::storage())?, now()]).map_err(|_| AppError::storage())?;
            for source in &details.sources {
                let observation = format!("{}:{}", id, source);
                let digest = journal
                    .after
                    .skills
                    .iter()
                    .find(|skill| &skill.id == id)
                    .map_or("", |skill| skill.digest.as_str());
                transaction.execute("INSERT OR IGNORE INTO source_observations(id,source,skill_id,digest,warnings,reader_agents,observed_at) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![observation, source, id.as_str(), digest, serde_json::to_string(&details.messages).map_err(|_| AppError::storage())?, serde_json::to_string(&details.readers).map_err(|_| AppError::storage())?, now()]).map_err(|_| AppError::storage())?;
            }
        }
        for removed in journal
            .before
            .skills
            .iter()
            .filter(|old| !journal.after.skills.iter().any(|new| new.id == old.id))
        {
            transaction
                .execute(
                    "DELETE FROM skill_metadata WHERE skill_id=?1",
                    [removed.id.as_str()],
                )
                .map_err(|_| AppError::storage())?;
            transaction
                .execute(
                    "DELETE FROM source_observations WHERE skill_id=?1",
                    [removed.id.as_str()],
                )
                .map_err(|_| AppError::storage())?;
        }
        if let Some(outcome) = &journal.outcome {
            let serialized = serde_json::to_string(outcome).map_err(|_| AppError::storage())?;
            transaction.execute("INSERT OR IGNORE INTO idempotency_records(operation_id,request_hash,result) VALUES(?1,?1,?2)", params![outcome.plan_id.as_str(), serialized]).map_err(|_| AppError::storage())?;
            transaction
                .execute(
                    "UPDATE operation_plans SET consumed=1 WHERE id=?1",
                    [outcome.plan_id.as_str()],
                )
                .map_err(|_| AppError::storage())?;
            transaction.execute("INSERT INTO device_settings(key,value) VALUES('import_outcome',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [serialized]).map_err(|_| AppError::storage())?;
        }
        transaction.execute("INSERT INTO device_settings(key,value) VALUES(?1,'true') ON CONFLICT(key) DO UPDATE SET value=excluded.value", [format!("journal:{}", journal.id)]).map_err(|_| AppError::storage())?;
        checkpoint("before_database")?;
        transaction.commit().map_err(|_| AppError::storage())?;
        checkpoint("after_database")
    }
    fn rollback_journal(&self, journal: &mut Journal) -> AppResult<()> {
        if self.journal_committed(journal)? {
            return Err(uncertain_payload());
        }
        self.validate_progress(journal)?;
        self.require_journal_metadata(journal)?;
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
        self.require_journal_metadata(journal)?;
        write_json(
            &self.library_dir().join(".skillbinder.json"),
            &journal.before,
        )
    }
    fn rollback_unrecorded_move(&self, journal: &mut Journal) -> AppResult<()> {
        if journal.progress.active_move == Some(journal.completed_moves) {
            self.move_payload(journal, journal.completed_moves, true)?;
            journal.progress.active_move = None;
            self.save_journal(journal)?;
        }
        Ok(())
    }
    fn require_journal_metadata(&self, journal: &Journal) -> AppResult<()> {
        let unexpected = || {
            AppError::new(
                ErrorCategory::Recovery,
                "Library metadata changed outside the interrupted operation.",
                "Preserve the library and operation journal for recovery before making changes.",
            )
        };
        let encode = |library: &Library| serde_json::to_value(library).map_err(|_| unexpected());
        let current = encode(&self.read_library().map_err(|_| unexpected())?)?;
        if current != encode(&journal.after)?
            && (self.journal_committed(journal)? || current != encode(&journal.before)?)
        {
            return Err(unexpected());
        }
        Ok(())
    }
    fn validate_move(&self, movement: &Move) -> AppResult<()> {
        for path in [&movement.from, &movement.to] {
            let owned = [
                self.config.data_dir.join("staging"),
                self.library_dir().join("skills"),
                self.config.data_dir.join("backups"),
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

#[cfg(not(test))]
fn checkpoint(_: &str) -> AppResult<()> {
    Ok(())
}
#[cfg(test)]
thread_local! { static FAULTS: std::cell::RefCell<std::collections::VecDeque<(&'static str, bool)>> = const { std::cell::RefCell::new(std::collections::VecDeque::new()) }; }
#[cfg(test)]
fn checkpoint(name: &str) -> AppResult<()> {
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
