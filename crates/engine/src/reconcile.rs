//! Keeps the database's skills in step with the directories under `skills/` and their records
//! under `records/`. Changes made there outside SkillBinder are unsupported, so only skills that
//! appear, disappear, or Git reports as changed are inspected; every other skill keeps its cached
//! facts. An inspected skill whose content differs from its record has drifted.

use crate::{
    database,
    library_store::{self, SavedSkill},
    payload,
    records::Record,
    shutdown::Shutdown,
    worker::State,
};
use skillbinder_proto::{AppError, AppResult, Drift, ErrorCategory};
use std::{collections::BTreeSet, fs};

/// The device setting holding the last commit whose skills the database reflects.
pub(crate) const RECONCILED_HEAD: &str = "reconciled_head";

impl State {
    /// Adds the skill directories the database lacks, drops the skills whose directory is gone,
    /// and records the skills that have no record yet.
    pub(crate) fn reconcile_library(&self) -> AppResult<()> {
        let listed = self.skill_dirs()?;
        let known = library_store::slugs(&mut self.database.borrow_mut())?;
        let unrecorded = library_store::unrecorded(&mut self.database.borrow_mut())?;
        self.remove_orphan_records(&listed)?;
        let changed: BTreeSet<_> = listed.symmetric_difference(&known).cloned().collect();
        self.reconcile_slugs(changed.into_iter().chain(unrecorded))
    }

    /// Rehashes the skills Git changed since the last reconciled commit, then records the current
    /// one, so a Pull interrupted before its reconcile still catches up at the next launch.
    pub(crate) fn reconcile_head(&self) -> AppResult<()> {
        let repo = self.library_dir();
        let Some(git) = self.git.as_ref().filter(|_| repo.join(".git").exists()) else {
            return Ok(());
        };
        let head = git.revision(&repo, "HEAD")?;
        let reconciled: Option<String> = self.setting(RECONCILED_HEAD)?.flatten();
        if head == reconciled {
            return Ok(());
        }
        if let Some(head) = &head {
            let known = match &reconciled {
                Some(commit) => git.revision(&repo, commit)?,
                None => None,
            };
            let mut changed = git.changed_skills(&repo, known.as_deref(), head)?;
            if known.is_none() {
                // Without a known starting commit, every skill is checked, so missing ones drop.
                changed.extend(library_store::slugs(&mut self.database.borrow_mut())?);
            }
            self.reconcile_slugs(changed)?;
        }
        self.set_setting(RECONCILED_HEAD, &head)
    }

    /// The exact names of the directories under `skills/`, which case-insensitive file systems
    /// would otherwise also match under other spellings.
    pub(crate) fn skill_dirs(&self) -> AppResult<BTreeSet<String>> {
        let root = self.library_dir().join("skills");
        fs::create_dir_all(&root).map_err(|_| AppError::storage())?;
        let mut listed = BTreeSet::new();
        for entry in fs::read_dir(&root).map_err(|_| AppError::storage())? {
            let entry = entry.map_err(|_| AppError::storage())?;
            // An unknown entry fails the reconcile rather than reading as a removed skill.
            let directory = entry.file_type().map_err(|_| AppError::storage())?.is_dir();
            if let (true, Some(slug)) = (directory, entry.file_name().to_str()) {
                listed.insert(slug.to_owned());
            }
        }
        Ok(listed)
    }

    /// Reinspects each slug's directory and reads its record, saving those that exist and
    /// dropping the rest. A new skill is unfiled, in no preset, and has no import time. A directory that cannot be
    /// inspected keeps its previous row, and the first such failure is returned after the rest
    /// are saved, so the caller retries it.
    fn reconcile_slugs(&self, slugs: impl IntoIterator<Item = String>) -> AppResult<()> {
        let root = self.library_dir().join("skills");
        let listed = self.skill_dirs()?;
        let mut saved = Vec::new();
        let mut gone = Vec::new();
        let mut failure = None;
        for slug in slugs {
            // A quit stops inspecting the rest; the recorded commit stays behind, so the next
            // launch retries them.
            if self.shutdown.cancelled() {
                return Err(Shutdown::error());
            }
            if !listed.contains(&slug) {
                gone.push(slug);
                continue;
            }
            match self
                .contained_skill_path(&root.join(&slug))
                .and_then(|path| payload::inspect(&path))
            {
                Ok(inspection) => match self.recorded(&slug, &inspection) {
                    Ok((record, drift)) => {
                        saved.push(SavedSkill::new(slug, &inspection, None, record, drift));
                    }
                    Err(error) => {
                        failure.get_or_insert(error);
                    }
                },
                Err(error) => {
                    self.log.record(format!(
                        "Library skill {slug} could not be inspected: {error}"
                    ));
                    failure.get_or_insert(error);
                }
            }
        }
        database::transaction(&mut self.database.borrow_mut(), |connection| {
            for record in &saved {
                library_store::save_skill(connection, record)?;
            }
            for slug in &gone {
                library_store::delete_skill(connection, slug)?;
            }
            Ok(())
        })?;
        failure.map_or(Ok(()), Err)
    }

    /// `slug`'s record, written from `inspection` when it has none, and whether its content
    /// drifted from it.
    fn recorded(
        &self,
        slug: &str,
        inspection: &payload::Inspection,
    ) -> AppResult<(Record, Option<Drift>)> {
        // A source learned on this device waits in the database for the next Push or Sync to
        // write it into the record, so a record without one keeps it.
        let cached = library_store::cached_source(&mut self.database.borrow_mut(), slug)?;
        // An unreadable record fails the skill's reconcile, which keeps its row and retries.
        let Some(mut record) = self.stored_record(slug)? else {
            // Its file is written at the next Push or Sync, which commits it: written now, it
            // would dirty the library and block a Pull of what other devices pushed meanwhile.
            return Ok((Record::of(inspection, cached), None));
        };
        if record.source.is_none() {
            record.source = cached;
        }
        if record.tree == inspection.tree {
            return Ok((record, None));
        }
        self.log
            .record(format!("Library skill {slug} differs from its record"));
        let repo = self.library_dir();
        let in_history = match &self.git {
            Some(git) if repo.join(".git").exists() => git.has_tree(&repo, &record.tree)?,
            _ => false,
        };
        let drift = if in_history || record.source.is_some() {
            Drift::Restorable
        } else {
            Drift::Lost
        };
        Ok((record, Some(drift)))
    }

    /// Reconciles after launch or onboarding: failures are logged and retried at the next launch,
    /// since the recorded commit only advances once every changed skill was inspected. A quit
    /// stops the pass without reporting it as a failure.
    pub(crate) fn catch_up(&self) {
        for result in [self.reconcile_head(), self.reconcile_library()] {
            if let Err(error) = result
                && error.category != ErrorCategory::Cancelled
            {
                self.log
                    .record(format!("Library reconciliation will retry: {error}"));
            }
        }
        self.apply_library();
        if let Err(error) = self.prune_source_caches() {
            self.log.record(format!(
                "Unused source caches will be removed later: {error}"
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn a_slug_spelled_differently_from_its_directory_is_dropped() {
        let directory = tempfile::tempdir().unwrap();
        let state = State::isolated_test_state(fs::canonicalize(directory.path()).unwrap());
        let skills = state.library_dir().join("skills");
        fs::create_dir(skills.join("foo")).unwrap();
        fs::write(
            skills.join("foo/SKILL.md"),
            "---\nname: foo\ndescription: test\n---\n",
        )
        .unwrap();
        state
            .reconcile_slugs(["Foo".to_owned(), "foo".to_owned()])
            .unwrap();
        let slugs = library_store::slugs(&mut state.database.borrow_mut()).unwrap();
        assert_eq!(slugs, BTreeSet::from(["foo".to_owned()]));
    }

    #[test]
    fn a_skill_without_a_record_gets_one_at_the_next_push() {
        let directory = tempfile::tempdir().unwrap();
        let state = State::isolated_test_state(fs::canonicalize(directory.path()).unwrap());
        let skill = state.library_dir().join("skills/foo");
        fs::create_dir(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: foo\ndescription: test\n---\n",
        )
        .unwrap();
        // Saved as a build before records would have: known, but with no recorded tree.
        let inspection = payload::inspect(&skill).unwrap();
        let mut saved = SavedSkill::new(
            "foo".into(),
            &inspection,
            None,
            Record::of(&inspection, None),
            None,
        );
        saved.recorded.clear();
        library_store::save_skill(&mut state.database.borrow_mut(), &saved).unwrap();
        state.reconcile_library().unwrap();
        let unrecorded = library_store::unrecorded(&mut state.database.borrow_mut()).unwrap();
        assert!(unrecorded.is_empty());
        // The file waits for the next Push or Sync, so a Pull is never blocked by it.
        assert!(state.read_record("foo").is_none());
        state.write_pending_records().unwrap();
        assert_eq!(state.read_record("foo").unwrap().tree, inspection.tree);
        // A source learned later waits in the database, survives a reconcile, and is written by
        // the next Push or Sync.
        let source = skillbinder_proto::SkillSource {
            url: "https://example.invalid/o/r.git".into(),
            reference: None,
            path: "foo".into(),
        };
        library_store::set_source(&mut state.database.borrow_mut(), "foo", &source).unwrap();
        state.reconcile_slugs(["foo".to_owned()]).unwrap();
        assert_eq!(state.read_record("foo").unwrap().source, None);
        state.write_pending_records().unwrap();
        assert_eq!(state.read_record("foo").unwrap().source, Some(source));
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_skill_is_retried_without_holding_back_the_others() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let state = State::isolated_test_state(fs::canonicalize(directory.path()).unwrap());
        let skills = state.library_dir().join("skills");
        for slug in ["locked", "open"] {
            fs::create_dir(skills.join(slug)).unwrap();
            fs::write(skills.join(slug).join("SKILL.md"), "---\nname: x\n---\n").unwrap();
        }
        let locked = skills.join("locked/SKILL.md");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read(&locked).is_ok() {
            // Permissions don't bind this user, such as root in a container.
            return;
        }
        let first = state.reconcile_library();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(first.is_err());
        let slugs = || library_store::slugs(&mut state.database.borrow_mut()).unwrap();
        assert_eq!(slugs(), BTreeSet::from(["open".to_owned()]));
        state.reconcile_library().unwrap();
        assert_eq!(
            slugs(),
            BTreeSet::from(["locked".to_owned(), "open".to_owned()])
        );
    }
}
