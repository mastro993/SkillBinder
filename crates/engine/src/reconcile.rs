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

impl State {
    /// Adds the skill directories the database lacks and drops the skills whose directory is
    /// gone.
    pub(crate) fn reconcile_library(&self) -> AppResult<()> {
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
        let known = library_store::slugs(&mut self.database.borrow_mut())?;
        self.reconcile_slugs(listed.symmetric_difference(&known).cloned())
    }

    /// Reinspects each slug's directory, saving those that exist and dropping the rest. A new
    /// skill is unfiled, in no preset, and has no install time.
    pub(crate) fn reconcile_slugs(&self, slugs: impl IntoIterator<Item = String>) -> AppResult<()> {
        let root = self.library_dir().join("skills");
        let mut saved = Vec::new();
        let mut gone = Vec::new();
        for slug in slugs {
            let path = root.join(&slug);
            if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.is_dir()) {
                let inspection = payload::inspect(&self.contained_skill_path(&path)?)?;
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
