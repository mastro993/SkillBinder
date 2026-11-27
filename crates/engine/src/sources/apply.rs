//! Replacing a library skill's content: Update brings its source's current content, and Reinstall
//! restores the content its record holds. Each skill's directory is replaced in its own journaled
//! move, so one failure never holds back the others. Restore and Discard replace through the same
//! move.

use super::{CacheUser, GONE, cache_dir, checkout, refusal};
use crate::{
    git::Git,
    library_store::SavedSkill,
    logging::{Area, Level},
    payload::{self, Inspection},
    persistence::{Journal, Move, RecordChange},
    records::Record,
    storage,
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
            let updated = self.staged(|state, staging| {
                let (source, fetched) = state.fetch_update(&git, skill, staging)?;
                state.land_update(skill, source, &fetched, staging)
            });
            match updated {
                Ok(()) => outcome.updated.push(skill.id.clone()),
                // A failure the journal could not undo stops every later change.
                Err(error) if self.recovery_required => return Err(error),
                Err(error) => {
                    self.update_failed(skill, error);
                    outcome.failed.push(skill.id.clone());
                }
            }
        }
        self.apply_library();
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
        let restored = self.staged(|state, staging| state.reinstall_content(&git, skill, staging));
        if restored.as_ref().is_err_and(|error| error.message == LOST) {
            // The source moved past the recorded content, so Reinstall is no longer offered.
            crate::library_store::set_drift(
                &mut self.database.borrow_mut(),
                id.as_str(),
                Some(Drift::Lost),
            )?;
        }
        if restored.is_ok() {
            self.apply_library();
        }
        restored
    }

    pub(super) fn require_git(&self) -> AppResult<Git> {
        self.git
            .clone()
            .ok_or_else(|| AppError::validation("Git is unavailable."))
    }

    /// Runs `work` with a fresh staging directory, removed afterwards unless recovery needs it.
    pub(super) fn staged<T>(
        &mut self,
        work: impl FnOnce(&mut Self, &Path) -> AppResult<T>,
    ) -> AppResult<T> {
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

    /// Records why updating `skill` failed, which only reaches the user as its message, so the
    /// error is logged here.
    pub(super) fn update_failed(&mut self, skill: &Skill, error: AppError) {
        self.log
            .failure(
                Level::Error,
                Area::Update,
                ("skill update failed", "skill update stopped at quit"),
                &error,
            )
            .field("slug", skill.id.as_str());
        self.updates.failed(skill, error.message);
    }

    /// Fetches `skill`'s source and checks its folder out under `staging`, refusing content an
    /// update cannot put in the library.
    pub(super) fn fetch_update(
        &self,
        git: &Git,
        skill: &Skill,
        staging: &Path,
    ) -> AppResult<(SkillSource, Inspection)> {
        let source = skill
            .source
            .clone()
            .filter(|source| !source.pinned())
            .ok_or_else(|| AppError::validation("This skill has no source to update from."))?;
        let cache = cache_dir(&self.config.data_dir, CacheUser::Update, &source);
        git.fetch_source(&cache, &source)?;
        let fetched = checkout(
            git,
            &cache,
            &source,
            &staging.join("source"),
            skill.id.as_str(),
        )?
        .ok_or_else(|| AppError::not_found(GONE))?;
        if let Some(why) = refusal(&fetched) {
            return Err(AppError::validation(why.update_text()));
        }
        Ok((source, fetched))
    }

    /// Replaces `skill`'s content with `fetched`, from `source`, and records it current.
    pub(super) fn land_update(
        &mut self,
        skill: &Skill,
        source: SkillSource,
        fetched: &Inspection,
        staging: &Path,
    ) -> AppResult<()> {
        self.replace(
            skill.id.as_str(),
            Some(skill),
            fetched,
            Some(Record::of(fetched, Some(source.clone()))),
            Some(Reason::Updated),
            staging,
        )?;
        self.updates
            .current_with(skill.id.clone(), source, fetched.tree.clone());
        Ok(())
    }

    fn reinstall_content(&mut self, git: &Git, skill: &Skill, staging: &Path) -> AppResult<()> {
        let slug = skill.id.as_str();
        let repo = self.library_dir();
        let into = staging.join("restored");
        let restored = if repo.join(".git").exists() && git.has_tree(&repo, &skill.recorded)? {
            let directory = into.join(slug);
            fs::create_dir_all(&directory).map_err(storage::error_at("creating", &directory))?;
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
        self.replace(
            slug,
            Some(skill),
            &restored,
            Some(record),
            Some(Reason::Reinstalled),
            staging,
        )
    }

    /// Replaces `slug`'s content with `content` and its record with `record`, or no record, in
    /// one journaled change; content already in place moves nothing. A slug the library lacks,
    /// `current` being `None`, arrives unfiled and in no preset. The change records `reason` as
    /// why the slug changed, or, with none, forgets why it did, as Discard does.
    pub(super) fn replace(
        &mut self,
        slug: &str,
        current: Option<&Skill>,
        content: &Inspection,
        record: Option<Record>,
        reason: Option<Reason>,
        staging: &Path,
    ) -> AppResult<()> {
        let target = self.library_dir().join("skills").join(slug);
        // The directory itself, not the database's facts, which an edit outside SkillBinder
        // leaves behind until the next reconcile.
        let present = if self.skill_dirs()?.contains(slug) {
            Some(self.contained_skill_path(&target)?)
        } else {
            None
        };
        let unchanged = match &present {
            Some(directory) => {
                payload::inspect(directory)?.manifest.digest == content.manifest.digest
            }
            None => false,
        };
        let mut moves = Vec::new();
        let mut saved = content.clone();
        if !unchanged {
            let new = staging.join("new");
            let replacement = new.join(slug);
            fs::create_dir_all(&new).map_err(storage::error_at("creating", &new))?;
            payload::materialize(content, &replacement)?;
            saved = payload::inspect(&replacement)?;
            if let Some(directory) = present {
                moves.push(Move {
                    from: directory,
                    to: staging.join("old").join(slug),
                });
            }
            moves.push(Move {
                from: replacement,
                to: target,
            });
        }
        // A version from before records keeps the source this device learned for the skill.
        let kept = record
            .clone()
            .unwrap_or_else(|| Record::of(content, current.and_then(|skill| skill.source.clone())));
        let records = vec![RecordChange {
            slug: slug.to_owned(),
            record,
            previous: self.read_record(slug),
        }];
        let (reasons, cleared) = match reason {
            Some(reason) => (vec![(slug.to_owned(), reason)], Vec::new()),
            None => (Vec::new(), vec![slug.to_owned()]),
        };
        self.commit(Journal {
            id: format!("replace-{}", uuid::Uuid::new_v4()),
            moves,
            completed_moves: 0,
            progress: Default::default(),
            skills: vec![SavedSkill::new(slug.to_owned(), &saved, None, kept, None)],
            outcome: None,
            records,
            held: Vec::new(),
            reasons,
            cleared,
            dropped: Vec::new(),
        })
    }
}
