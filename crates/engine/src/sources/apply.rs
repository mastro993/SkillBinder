//! Replacing a library skill's content: Update brings its source's current content, and Reinstall
//! restores the content its record holds. Each skill's directory is replaced in its own journaled
//! move, so one failure never holds back the others.

use super::{CacheUser, GONE, cache_dir, checkout, refusal};
use crate::{
    git::Git,
    library_store::SavedSkill,
    payload::{self, Inspection},
    persistence::{Journal, Move, RecordChange},
    records::Record,
    worker::State,
};
use skillbinder_proto::*;
use std::{fs, path::Path};

/// Why Reinstall cannot happen: nothing holds the recorded content any more.
const LOST: &str =
    "Neither the library's history nor the skill's source holds its recorded content.";

impl State {
    /// Updates each skill in `ids` from its source. A skill that fails keeps its content, and its
    /// update state records why.
    pub(crate) fn update_skills(&mut self, ids: Vec<SkillId>) -> AppResult<UpdateOutcome> {
        self.require_recovered()?;
        let git = self.require_git()?;
        let library = self.read_library()?;
        let mut outcome = UpdateOutcome::default();
        let mut selected = Vec::new();
        for id in ids {
            match library.skills.iter().find(|skill| skill.id == id) {
                Some(skill) if !selected.contains(&skill) => selected.push(skill),
                Some(_) => {}
                // A skill removed meanwhile, such as by a Pull, has nothing to update.
                None if !outcome.failed.contains(&id) => outcome.failed.push(id),
                None => {}
            }
        }
        for skill in selected {
            match self.staged(|state, staging| state.update_skill(&git, skill, staging)) {
                Ok(()) => outcome.updated.push(skill.id.clone()),
                // A failure the journal could not undo stops every later change.
                Err(error) if self.recovery_required => return Err(error),
                Err(error) => {
                    self.updates.failed(skill, error.message);
                    outcome.failed.push(skill.id.clone());
                }
            }
        }
        Ok(outcome)
    }

    /// Restores skill `id`'s recorded content, from the library's Git history when it holds it,
    /// else from the skill's source while that still publishes it.
    pub(crate) fn reinstall_skill(&mut self, id: SkillId) -> AppResult<()> {
        self.require_recovered()?;
        let git = self.require_git()?;
        let library = self.read_library()?;
        let skill = library
            .skills
            .iter()
            .find(|skill| skill.id == id)
            .ok_or_else(|| AppError::not_found("This skill is no longer in the library."))?;
        let restored = self.staged(|state, staging| state.restore_skill(&git, skill, staging));
        if restored.as_ref().is_err_and(|error| error.message == LOST) {
            // The source moved past the recorded content, so Reinstall is no longer offered.
            crate::library_store::set_drift(
                &mut self.database.borrow_mut(),
                id.as_str(),
                Some(Drift::Lost),
            )?;
        }
        restored
    }

    fn require_git(&self) -> AppResult<Git> {
        self.git
            .clone()
            .ok_or_else(|| AppError::validation("Git is unavailable."))
    }

    /// Runs `work` with a fresh staging directory, removed afterwards unless recovery needs it.
    fn staged(&mut self, work: impl FnOnce(&mut Self, &Path) -> AppResult<()>) -> AppResult<()> {
        let staging = self
            .config
            .data_dir
            .join("staging")
            .join(format!("replace-{}", uuid::Uuid::new_v4()));
        let result = work(self, &staging);
        if !self.recovery_required {
            let _ = fs::remove_dir_all(staging);
        }
        result
    }

    fn update_skill(&mut self, git: &Git, skill: &Skill, staging: &Path) -> AppResult<()> {
        let source = skill
            .source
            .clone()
            .filter(|source| !source.pinned())
            .ok_or_else(|| AppError::validation("This skill has no source to update from."))?;
        let slug = skill.id.as_str();
        let cache = cache_dir(&self.config.data_dir, CacheUser::Update, &source);
        git.fetch_source(&cache, &source)?;
        let fetched = checkout(git, &cache, &source, &staging.join("source"), slug)?
            .ok_or_else(|| AppError::not_found(GONE))?;
        if let Some(why) = refusal(&fetched) {
            return Err(AppError::validation(why));
        }
        let tree = fetched.tree.clone();
        self.replace(
            skill,
            &fetched,
            Record::of(&fetched, Some(source.clone())),
            staging,
        )?;
        self.updates.current_with(skill.id.clone(), source, tree);
        Ok(())
    }

    fn restore_skill(&mut self, git: &Git, skill: &Skill, staging: &Path) -> AppResult<()> {
        let slug = skill.id.as_str();
        let repo = self.library_dir();
        let into = staging.join("restored");
        let restored = if repo.join(".git").exists() && git.has_tree(&repo, &skill.recorded)? {
            let directory = into.join(slug);
            fs::create_dir_all(&directory).map_err(|_| AppError::storage())?;
            git.restore_tree(&repo, &skill.recorded, &staging.join("scratch"), &directory)?;
            payload::inspect(&directory)?
        } else {
            let source = skill
                .source
                .as_ref()
                .ok_or_else(|| AppError::not_found(LOST))?;
            let cache = cache_dir(&self.config.data_dir, CacheUser::Update, source);
            git.fetch_source(&cache, source)?;
            checkout(git, &cache, source, &into, slug)?
                .filter(|fetched| fetched.tree == skill.recorded)
                .ok_or_else(|| AppError::not_found(LOST))?
        };
        if restored.tree != skill.recorded {
            return Err(AppError::not_found(LOST));
        }
        let record = Record {
            tree: skill.recorded.clone(),
            source: skill.source.clone(),
        };
        self.replace(skill, &restored, record, staging)
    }

    /// Replaces `skill`'s content with `content` and its record with `record`, in one journaled
    /// change; content already in place moves nothing.
    fn replace(
        &mut self,
        skill: &Skill,
        content: &Inspection,
        record: Record,
        staging: &Path,
    ) -> AppResult<()> {
        let slug = skill.id.as_str();
        let mut moves = Vec::new();
        let mut saved = content.clone();
        if content.manifest.digest != skill.digest {
            let replacement = staging.join("new").join(slug);
            fs::create_dir_all(staging.join("new")).map_err(|_| AppError::storage())?;
            payload::materialize(content, &replacement)?;
            saved = payload::inspect(&replacement)?;
            moves = vec![
                Move {
                    from: self.skill_dir(&skill.id)?,
                    to: staging.join("old").join(slug),
                },
                Move {
                    from: replacement,
                    to: self.library_dir().join("skills").join(slug),
                },
            ];
        }
        let records = vec![RecordChange {
            slug: slug.to_owned(),
            record: Some(record.clone()),
            previous: self.read_record(slug),
        }];
        self.commit(Journal {
            id: format!("replace-{}", uuid::Uuid::new_v4()),
            moves,
            completed_moves: 0,
            progress: Default::default(),
            skills: vec![SavedSkill::new(slug.to_owned(), &saved, None, record, None)],
            outcome: None,
            records,
        })
    }
}
