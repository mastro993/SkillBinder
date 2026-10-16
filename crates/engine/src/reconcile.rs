//! Keeps the database's skills in step with the directories under `skills/`. Changes made there
//! outside SkillBinder are unsupported, so only directories that appear, disappear, or Git reports
//! as changed are inspected; every other skill keeps its cached facts.

use crate::{
    database,
    library_store::{self, SkillRecord},
    payload,
    worker::State,
};
use skillbinder_proto::{AppError, AppResult};
use std::{collections::BTreeSet, fs};

/// The device setting holding the last commit whose skills the database reflects.
pub(crate) const RECONCILED_HEAD: &str = "reconciled_head";

impl State {
    /// Adds the skill directories the database lacks and drops the skills whose directory is
    /// gone.
    pub(crate) fn reconcile_library(&self) -> AppResult<()> {
        let listed = self.skill_dirs()?;
        let known = library_store::slugs(&mut self.database.borrow_mut())?;
        self.reconcile_slugs(listed.symmetric_difference(&known).cloned())
    }

    /// Rehashes the skills Git changed since the last reconciled commit, then records the current
    /// one, so a Pull interrupted before its reconcile still catches up at the next launch.
    pub(crate) fn reconcile_head(&self) -> AppResult<()> {
        let repo = self.library_dir();
        let Some(git) = self.git.as_ref().filter(|_| repo.join(".git").exists()) else {
            return Ok(());
        };
        let head = git.head(&repo)?;
        let reconciled: Option<String> = self.setting(RECONCILED_HEAD)?.flatten();
        if head == reconciled {
            return Ok(());
        }
        if let Some(head) = &head {
            // A recorded commit Git no longer has falls back to every skill in the tree.
            let changed = git
                .changed_skills(&repo, reconciled.as_deref(), head)
                .or_else(|_| git.changed_skills(&repo, None, head))?;
            self.reconcile_slugs(changed)?;
        }
        self.set_setting(RECONCILED_HEAD, &head)
    }

    /// The exact names of the directories under `skills/`, which case-insensitive file systems
    /// would otherwise also match under other spellings.
    fn skill_dirs(&self) -> AppResult<BTreeSet<String>> {
        let root = self.library_dir().join("skills");
        fs::create_dir_all(&root).map_err(|_| AppError::storage())?;
        let mut listed = BTreeSet::new();
        for entry in fs::read_dir(&root).map_err(|_| AppError::storage())? {
            let entry = entry.map_err(|_| AppError::storage())?;
            let directory = entry.file_type().is_ok_and(|kind| kind.is_dir());
            if let (true, Some(slug)) = (directory, entry.file_name().to_str()) {
                listed.insert(slug.to_owned());
            }
        }
        Ok(listed)
    }

    /// Reinspects each slug's directory, saving those that exist and dropping the rest. A new
    /// skill is unfiled, in no preset, and has no install time.
    pub(crate) fn reconcile_slugs(&self, slugs: impl IntoIterator<Item = String>) -> AppResult<()> {
        let root = self.library_dir().join("skills");
        let listed = self.skill_dirs()?;
        let mut saved = Vec::new();
        let mut gone = Vec::new();
        for slug in slugs {
            if listed.contains(&slug) {
                let inspection = payload::inspect(&self.contained_skill_path(&root.join(&slug))?)?;
                saved.push(SkillRecord::new(slug, &inspection, None));
            } else {
                gone.push(slug);
            }
        }
        if saved.is_empty() && gone.is_empty() {
            return Ok(());
        }
        database::transaction(&mut self.database.borrow_mut(), |connection| {
            for record in &saved {
                library_store::save_skill(connection, record)?;
            }
            for slug in &gone {
                library_store::delete_skill(connection, slug)?;
            }
            Ok(())
        })
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
}
