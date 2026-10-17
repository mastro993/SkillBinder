//! Updating library skills: each one's directory is replaced by its source's current content in
//! its own journaled move, so one failure never holds back the others.

use super::{CacheUser, GONE, cache_dir, checkout, refusal};
use crate::{
    git::Git,
    library_store::SkillRecord,
    payload,
    persistence::{Journal, Move},
    worker::State,
};
use skillbinder_proto::*;
use std::fs;

impl State {
    /// Updates each skill in `ids` from its source. A skill that fails keeps its content, and its
    /// update state records why.
    pub(crate) fn update_skills(&mut self, ids: Vec<SkillId>) -> AppResult<UpdateOutcome> {
        self.require_recovered()?;
        let git = self
            .git
            .clone()
            .ok_or_else(|| AppError::validation("Git is unavailable."))?;
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
            match self.update_skill(&git, skill) {
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

    fn update_skill(&mut self, git: &Git, skill: &Skill) -> AppResult<()> {
        let source = skill
            .source
            .clone()
            .filter(|source| !source.pinned())
            .ok_or_else(|| AppError::validation("This skill has no source to update from."))?;
        let staging = self
            .config
            .data_dir
            .join("staging")
            .join(format!("update-{}", uuid::Uuid::new_v4()));
        let result = self.replace_from_source(git, skill, source, &staging);
        if !self.recovery_required {
            let _ = fs::remove_dir_all(staging);
        }
        result
    }

    fn replace_from_source(
        &mut self,
        git: &Git,
        skill: &Skill,
        source: SkillSource,
        staging: &std::path::Path,
    ) -> AppResult<()> {
        let slug = skill.id.as_str();
        let cache = cache_dir(&self.config.data_dir, CacheUser::Update, &source);
        git.fetch_source(&cache, &source)?;
        let fetched = checkout(git, &cache, &source, &staging.join("source"), slug)?
            .ok_or_else(|| AppError::not_found(GONE))?;
        if let Some(why) = refusal(&fetched) {
            return Err(AppError::validation(why));
        }
        if fetched.manifest.digest != skill.digest {
            let replacement = staging.join("new").join(slug);
            fs::create_dir_all(staging.join("new")).map_err(|_| AppError::storage())?;
            payload::materialize(&fetched, &replacement)?;
            let installed = payload::inspect(&replacement)?;
            self.commit(Journal {
                id: format!("update-{}", uuid::Uuid::new_v4()),
                moves: vec![
                    Move {
                        from: self.skill_dir(&skill.id)?,
                        to: staging.join("old").join(slug),
                    },
                    Move {
                        from: replacement,
                        to: self.library_dir().join("skills").join(slug),
                    },
                ],
                completed_moves: 0,
                progress: Default::default(),
                skills: vec![SkillRecord::new(
                    slug.to_owned(),
                    &installed,
                    None,
                    Some(source.clone()),
                )],
                outcome: None,
            })?;
        }
        self.updates
            .current_with(skill.id.clone(), source, fetched.manifest.digest);
        Ok(())
    }
}
