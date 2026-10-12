//! Journaled writes into deployment targets. A journal is saved before the first write and
//! removed after the database records the result; recovery finishes or undoes what a crash
//! interrupted. Every write goes to a recorded path or to a scratch sibling named after the
//! journal, so recovery never touches anything else.

use super::key;
use crate::{
    payload::{self, Inspection},
    persistence::{checkpoint, now, sync_directory, write_json},
    worker::State,
};
use rusqlite::{Transaction, params};
use serde::{Deserialize, Serialize};
use skillbinder_proto::*;
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
pub(crate) struct DeploymentJournal {
    pub id: String,
    pub target: DeploymentTarget,
    pub change: Change,
}

#[derive(Serialize, Deserialize)]
pub(crate) enum Change {
    /// New deployments; undone unless the database recorded them.
    Add(Vec<Copies>),
    /// One deployment being removed; always finished.
    Remove {
        copies: Copies,
        /// Changed copies the user confirmed discarding.
        discard: Vec<PathBuf>,
        /// Index of the copy verified for deletion while it is being deleted.
        deleting: Option<usize>,
    },
}

/// One deployment's copies.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Copies {
    pub skill_id: SkillId,
    pub digest: String,
    /// Every recorded copy.
    pub paths: Vec<PathBuf>,
    /// Copies this operation creates; the others already held identical bytes.
    pub written: Vec<PathBuf>,
}

/// The sibling of `path` that a journaled step stages into or deletes from.
fn scratch(path: &Path, journal: &str, index: usize) -> AppResult<PathBuf> {
    let parent = path.parent().ok_or_else(AppError::storage)?;
    Ok(parent.join(format!(".skillbinder-{journal}-{index}")))
}

fn exists(path: &Path) -> AppResult<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(_) => Err(AppError::storage()),
    }
}

/// Whether the skill at `path` still has the recorded digest.
pub(crate) fn matches(path: &Path, digest: &str) -> bool {
    payload::inspect(path).is_ok_and(|inspection| inspection.manifest.digest == digest)
}

/// Deletes a directory, or only the link or file at `path`.
fn delete(path: &Path) -> AppResult<()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| AppError::storage())?;
    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .map_err(|_| AppError::storage())?;
    sync_directory(path.parent().ok_or_else(AppError::storage)?)
}

fn sync_parent(path: &Path) -> AppResult<()> {
    sync_directory(path.parent().ok_or_else(AppError::storage)?)
}

impl State {
    fn deployment_journals(&self) -> PathBuf {
        self.config.data_dir.join("journals/deployments")
    }

    fn journal_file(&self, id: &str) -> AppResult<PathBuf> {
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err(AppError::storage());
        }
        Ok(self.deployment_journals().join(format!("{id}.json")))
    }

    fn save_deployment_journal(&self, journal: &DeploymentJournal) -> AppResult<()> {
        write_json(&self.journal_file(&journal.id)?, journal)
    }

    fn remove_deployment_journal(&self, journal: &DeploymentJournal) -> AppResult<()> {
        fs::remove_file(self.journal_file(&journal.id)?).map_err(|_| AppError::storage())?;
        sync_directory(&self.deployment_journals())
    }

    fn deployment_committed(&self, journal: &DeploymentJournal) -> AppResult<bool> {
        Ok(self
            .setting::<bool>(&format!("journal:{}", journal.id))?
            .unwrap_or(false))
    }

    /// Runs a journaled change. A failure undoes an addition or finishes a removal at once;
    /// if that fails too, the journal stays and the library waits for recovery.
    pub(crate) fn run_deployment(
        &mut self,
        journal: DeploymentJournal,
        sources: &[Inspection],
    ) -> AppResult<Vec<PathBuf>> {
        self.save_deployment_journal(&journal)?;
        self.recovery_required = true;
        let mut journal = journal;
        let result = if matches!(journal.change, Change::Add(_)) {
            self.write_copies(&journal, sources).map(|()| Vec::new())
        } else {
            self.settle_removal(&mut journal)
        };
        let settled = match &result {
            Ok(_) => Ok(()),
            Err(_) => self.settle(&mut journal).inspect_err(|error| {
                self.log.record(format!(
                    "{}: deployment write failed; recovery required",
                    error.diagnostic_id
                ));
            }),
        };
        if settled.is_ok() {
            self.remove_deployment_journal(&journal)?;
            self.recovery_required = false;
        }
        result
    }

    fn write_copies(
        &mut self,
        journal: &DeploymentJournal,
        sources: &[Inspection],
    ) -> AppResult<()> {
        let Change::Add(deployments) = &journal.change else {
            return Err(AppError::storage());
        };
        let mut index = 0;
        for (copies, source) in deployments.iter().zip(sources) {
            for path in &copies.written {
                let parent = path.parent().ok_or_else(AppError::storage)?;
                fs::create_dir_all(parent).map_err(|_| AppError::storage())?;
                let staging = scratch(path, &journal.id, index)?;
                index += 1;
                payload::stage(source, &staging)?;
                checkpoint("deploy_staged")?;
                fs::rename(&staging, path).map_err(|_| AppError::storage())?;
                sync_directory(parent)?;
                checkpoint("deploy_renamed")?;
            }
        }
        let transaction = self
            .database
            .transaction()
            .map_err(|_| AppError::storage())?;
        for copies in deployments {
            record(&transaction, &journal.target, copies)?;
        }
        mark(&transaction, journal)?;
        checkpoint("deploy_before_database")?;
        transaction.commit().map_err(|_| AppError::storage())?;
        checkpoint("deploy_after_database")
    }

    /// Deletes each copy that still matches the digest or that the user confirmed discarding,
    /// and records what is left. Returns the copies kept because they changed.
    fn settle_removal(&mut self, journal: &mut DeploymentJournal) -> AppResult<Vec<PathBuf>> {
        let Change::Remove { copies, .. } = &journal.change else {
            return Err(AppError::storage());
        };
        let copies = copies.clone();
        let mut kept = Vec::new();
        for (index, path) in copies.paths.iter().enumerate() {
            if !self.remove_copy(journal, index, path, &copies.digest)? {
                kept.push(path.clone());
            }
        }
        if !self.deployment_committed(journal)? {
            let transaction = self
                .database
                .transaction()
                .map_err(|_| AppError::storage())?;
            transaction
                .execute(
                    "DELETE FROM deployments WHERE target=?1 AND skill_id=?2",
                    params![key(&journal.target), copies.skill_id.as_str()],
                )
                .map_err(|_| AppError::storage())?;
            if !kept.is_empty() {
                let remaining = Copies {
                    paths: kept.clone(),
                    ..copies
                };
                record(&transaction, &journal.target, &remaining)?;
            }
            mark(&transaction, journal)?;
            checkpoint("remove_before_database")?;
            transaction.commit().map_err(|_| AppError::storage())?;
        }
        Ok(kept)
    }

    /// Removes one copy, or returns `false` when it changed without confirmation and stays. A
    /// copy moves to its scratch name before it is checked, and the journal marks it before it
    /// is deleted, so a crash never deletes a changed copy the user did not confirm.
    fn remove_copy(
        &mut self,
        journal: &mut DeploymentJournal,
        index: usize,
        path: &Path,
        digest: &str,
    ) -> AppResult<bool> {
        let Change::Remove {
            discard, deleting, ..
        } = &journal.change
        else {
            return Err(AppError::storage());
        };
        let (confirmed, marked) = (discard.iter().any(|p| p == path), *deleting == Some(index));
        let trash = scratch(path, &journal.id, index)?;
        if exists(&trash)? {
            // Left by a crash: half deleted once marked, otherwise moved here before its check.
            if marked || confirmed || matches(&trash, digest) {
                self.delete_marked(journal, index, &trash)?;
            } else if exists(path)? {
                self.set_aside(&trash, path, &journal.id)?;
            } else {
                fs::rename(&trash, path).map_err(|_| AppError::storage())?;
                sync_parent(path)?;
                return Ok(false);
            }
        }
        if !exists(path)? {
            return Ok(true);
        }
        if marked {
            // The marked copy is gone; this one at its path has not been checked.
            set_deleting(journal, None);
            self.save_deployment_journal(journal)?;
        }
        fs::rename(path, &trash).map_err(|_| AppError::storage())?;
        sync_parent(path)?;
        checkpoint("remove_renamed")?;
        if confirmed || matches(&trash, digest) {
            self.delete_marked(journal, index, &trash)?;
            Ok(true)
        } else {
            fs::rename(&trash, path).map_err(|_| AppError::storage())?;
            sync_parent(path)?;
            Ok(false)
        }
    }

    fn delete_marked(
        &mut self,
        journal: &mut DeploymentJournal,
        index: usize,
        trash: &Path,
    ) -> AppResult<()> {
        set_deleting(journal, Some(index));
        self.save_deployment_journal(journal)?;
        delete(trash)?;
        checkpoint("remove_deleted")?;
        set_deleting(journal, None);
        self.save_deployment_journal(journal)
    }

    /// Keeps an unconfirmed changed copy whose path was meanwhile filled again, under a
    /// visible name beside it.
    fn set_aside(&self, trash: &Path, path: &Path, journal: &str) -> AppResult<()> {
        let name = path.file_name().ok_or_else(AppError::storage)?;
        let kept = path.with_file_name(format!(
            "{}.skillbinder-kept-{journal}",
            name.to_string_lossy()
        ));
        fs::rename(trash, &kept).map_err(|_| AppError::storage())?;
        sync_parent(path)?;
        self.log
            .record(format!("Kept a changed copy at {}", kept.display()));
        Ok(())
    }

    /// Undoes an unrecorded addition: deletes its scratch copies and the copies it wrote that
    /// still hold its bytes. A recorded addition only loses leftover scratch copies.
    fn undo_addition(&self, journal: &DeploymentJournal) -> AppResult<()> {
        let Change::Add(deployments) = &journal.change else {
            return Err(AppError::storage());
        };
        let committed = self.deployment_committed(journal)?;
        let written = deployments
            .iter()
            .flat_map(|copies| copies.written.iter().map(|path| (path, &copies.digest)));
        for (index, (path, digest)) in written.enumerate() {
            let staging = scratch(path, &journal.id, index)?;
            if exists(&staging)? {
                delete(&staging)?;
            }
            if !committed && exists(path)? && matches(path, digest) {
                fs::rename(path, &staging).map_err(|_| AppError::storage())?;
                sync_parent(path)?;
                delete(&staging)?;
            }
        }
        Ok(())
    }

    fn settle(&mut self, journal: &mut DeploymentJournal) -> AppResult<()> {
        match journal.change {
            Change::Add(_) => self.undo_addition(journal),
            Change::Remove { .. } => self.settle_removal(journal).map(|_| ()),
        }
    }

    /// Settles every deployment journal left by a crash.
    pub(crate) fn recover_deployments(&mut self) -> AppResult<()> {
        let directory = self.deployment_journals();
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(_) => return Err(AppError::storage()),
        };
        for entry in entries {
            let path = entry.map_err(|_| AppError::storage())?.path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let mut journal: DeploymentJournal =
                serde_json::from_slice(&fs::read(&path).map_err(|_| AppError::storage())?)
                    .map_err(|_| AppError::storage())?;
            if self.journal_file(&journal.id)? != path {
                return Err(AppError::storage());
            }
            self.settle(&mut journal)?;
            self.remove_deployment_journal(&journal)?;
        }
        Ok(())
    }
}

fn set_deleting(journal: &mut DeploymentJournal, index: Option<usize>) {
    if let Change::Remove { deleting, .. } = &mut journal.change {
        *deleting = index;
    }
}

fn record(transaction: &Transaction, target: &DeploymentTarget, copies: &Copies) -> AppResult<()> {
    let mut paths: Vec<_> = copies
        .paths
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    paths.sort();
    transaction
        .execute(
            "INSERT INTO deployments(target,skill_id,digest,paths,recorded_at) VALUES(?1,?2,?3,?4,?5)",
            params![
                key(target),
                copies.skill_id.as_str(),
                copies.digest,
                serde_json::to_string(&paths).map_err(|_| AppError::storage())?,
                now()
            ],
        )
        .map_err(|_| AppError::storage())?;
    Ok(())
}

fn mark(transaction: &Transaction, journal: &DeploymentJournal) -> AppResult<()> {
    transaction
        .execute(
            "INSERT INTO device_settings(key,value) VALUES(?1,'true') \
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [format!("journal:{}", journal.id)],
        )
        .map_err(|_| AppError::storage())?;
    Ok(())
}
