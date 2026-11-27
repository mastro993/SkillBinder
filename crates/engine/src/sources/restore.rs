//! Restore and Discard (ADR 0040): one skill put back to a past version from History, or to the
//! base, which undoes a change not pushed yet. Both add a change and never rewrite history. They
//! write through Reinstall's journaled replace, so recovery at launch finishes or rolls back one
//! that was interrupted.

use crate::{
    git::Git,
    payload,
    persistence::{Journal, Move, RecordChange},
    records::{self, MAX_RECORD_BYTES, Record},
    storage,
    worker::State,
};
use skillbinder_proto::{AppError, AppResult, Reason};
use std::{fs, path::Path};

/// Why Restore cannot happen: the change named holds no version of the skill.
const NO_VERSION: &str = "This change holds no version of the skill.";
/// Why Discard cannot happen: the library holds no change to the skill.
const NOTHING_TO_DISCARD: &str = "This skill has no change to discard.";

impl State {
    /// Puts `slug` back to its version in `commit`, or in the commit before it when `commit`
    /// removed it, as a change on this device that says when that version was made. A skill the
    /// library lacks comes back unfiled and in no preset.
    pub(crate) fn restore_version(&mut self, slug: &str, commit: &str) -> AppResult<()> {
        self.require_recovered()?;
        let git = self.require_git()?;
        let repo = self.library_dir();
        if !commit_name(commit)
            || !payload::portable(slug)
            || !repo.join(".git").exists()
            || git.revision(&repo, commit)?.is_none()
        {
            return Err(AppError::not_found(NO_VERSION));
        }
        let version = if git.tree_at(&repo, commit, slug)?.is_some() {
            Some(commit.to_owned())
        } else {
            git.revision(&repo, &format!("{commit}^"))?
        };
        let version = match version {
            Some(version) if git.tree_at(&repo, &version, slug)?.is_some() => version,
            _ => return Err(AppError::not_found(NO_VERSION)),
        };
        let at = git.commit_time(&repo, &version)?;
        self.put_version(&git, slug, &version, Some(Reason::Restored { at }))?;
        self.apply_skill(slug);
        Ok(())
    }

    /// Puts `slug` back as it was at the base and forgets why it changed: a changed or removed
    /// skill returns to its base version, and one added since the base leaves the library, with
    /// its presets and folder, while its managed copies stay in place, unmanaged.
    pub(crate) fn discard_change(&mut self, slug: &str) -> AppResult<()> {
        self.require_recovered()?;
        let git = self.require_git()?;
        let repo = self.library_dir();
        if !payload::portable(slug) {
            return Err(AppError::not_found(NOTHING_TO_DISCARD));
        }
        let base = if repo.join(".git").exists() {
            git.base(&repo)?
        } else {
            None
        };
        let base = match base {
            Some(base) if git.tree_at(&repo, &base, slug)?.is_some() => Some(base),
            _ => None,
        };
        match base {
            Some(base) => self.put_version(&git, slug, &base, None)?,
            None => self.staged(|state, staging| state.drop_added(slug, staging))?,
        }
        self.apply_skill(slug);
        Ok(())
    }

    /// Writes `slug`'s directory and record from `commit`, which holds the directory, recording
    /// `reason`, or forgetting why it changed with none.
    fn put_version(
        &mut self,
        git: &Git,
        slug: &str,
        commit: &str,
        reason: Option<Reason>,
    ) -> AppResult<()> {
        let repo = self.library_dir();
        let tree = git
            .tree_at(&repo, commit, slug)?
            .ok_or_else(|| AppError::not_found(NO_VERSION))?;
        // A version from before records, or with one that does not parse, has none.
        let record: Option<Record> = git
            .record_at(&repo, commit, slug, MAX_RECORD_BYTES)?
            .and_then(|text| records::parse(&text).ok());
        // A skill whose directory was removed outside SkillBinder keeps its row until the next
        // reconcile; dropping it now, as that reconcile would, brings the skill back unfiled and
        // in no preset.
        if !self.skill_dirs()?.contains(slug) {
            crate::library_store::delete_skill(&mut self.database.borrow_mut(), slug)?;
        }
        let library = self.read_library()?;
        let current = library
            .skills
            .iter()
            .find(|skill| skill.id.as_str() == slug);
        self.staged(|state, staging| {
            let directory = staging.join("restored").join(slug);
            fs::create_dir_all(&directory).map_err(storage::error_at("creating", &directory))?;
            git.restore_tree(&repo, &tree, &staging.join("scratch"), &directory)?;
            let content = payload::inspect(&directory)?;
            state.replace(slug, current, &content, record, reason, staging)
        })
    }

    /// Drops `slug`, added on this device since the base, from the library in one journaled
    /// change, leaving its managed copies where they are.
    fn drop_added(&mut self, slug: &str, staging: &Path) -> AppResult<()> {
        let present = self.skill_dirs()?.contains(slug);
        if !present && !crate::library_store::holds(&mut self.database.borrow_mut(), slug)? {
            return Err(AppError::not_found(NOTHING_TO_DISCARD));
        }
        let moves = if present {
            vec![Move {
                from: self.contained_skill_path(&self.library_dir().join("skills").join(slug))?,
                to: staging.join("old").join(slug),
            }]
        } else {
            Vec::new()
        };
        self.commit(Journal {
            id: format!("discard-{}", uuid::Uuid::new_v4()),
            moves,
            completed_moves: 0,
            progress: Default::default(),
            skills: Vec::new(),
            outcome: None,
            records: vec![RecordChange {
                slug: slug.to_owned(),
                record: None,
                previous: self.read_record(slug),
            }],
            held: Vec::new(),
            reasons: Vec::new(),
            cleared: vec![slug.to_owned()],
            dropped: vec![slug.to_owned()],
        })
    }
}

/// Whether `name` is a commit id, as History names its changes.
fn commit_name(name: &str) -> bool {
    (4..=64).contains(&name.len()) && name.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests;
