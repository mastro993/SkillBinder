//! Reviewing one skill's update before it lands (ADR 0037). A review fetches the source as
//! Update does and compares the skill's current library content with the fetched content. Update
//! from a review fetches again and lands only while both still match what was reviewed;
//! otherwise it builds the newest review from that same checkout and writes nothing.

mod patch;

use crate::{
    logging::{Area, Level},
    payload::{self, Inspection},
    worker::State,
};
use skillbinder_proto::*;

impl State {
    /// What updating skill `id` from its source would change. Nothing but staging is written,
    /// and staging is gone once the review is built.
    pub(crate) fn review_update(&mut self, id: SkillId) -> AppResult<UpdateReview> {
        self.require_recovered()?;
        let git = self.require_git()?;
        let skill = self.library_skill(&id)?;
        self.staged(|state, staging| {
            let (source, fetched) = state.fetch_update(&git, &skill, staging)?;
            state.review(&skill, source, &fetched)
        })
    }

    /// Updates the reviewed skill while its source still holds the reviewed content and its
    /// library content is still what the review started from; otherwise returns the changes
    /// now. A failure is recorded and reported as Update's are.
    pub(crate) fn update_reviewed(
        &mut self,
        reviewed: ReviewedUpdate,
    ) -> AppResult<ReviewedOutcome> {
        self.require_recovered()?;
        let git = self.require_git()?;
        let skill = match self.library_skill(&reviewed.id) {
            Ok(skill) => skill,
            // A skill removed meanwhile, such as by a Pull, has nothing to update.
            Err(error) if error.category == ErrorCategory::NotFound => {
                return Ok(ReviewedOutcome::Updated(UpdateOutcome {
                    updated: Vec::new(),
                    failed: vec![reviewed.id],
                }));
            }
            Err(error) => return Err(error),
        };
        let landed = self.staged(|state, staging| {
            let (source, fetched) = state.fetch_update(&git, &skill, staging)?;
            let moved = if fetched.tree != reviewed.to {
                Some(Moved::Source)
            } else if payload::inspect(&state.skill_dir(&skill.id)?)?
                .manifest
                .digest
                != reviewed.from
            {
                Some(Moved::Library)
            } else {
                None
            };
            let Some(moved) = moved else {
                return state
                    .land_update(&skill, source, &fetched, staging)
                    .map(|()| None);
            };
            state
                .log
                .event(Level::Info, Area::Update, "review moved")
                .field("slug", skill.id.as_str())
                .field("moved", format!("{moved:?}").to_lowercase());
            let review = Box::new(state.review(&skill, source, &fetched)?);
            Ok(Some(ReviewedOutcome::Moved { moved, review }))
        });
        let mut outcome = UpdateOutcome::default();
        match landed {
            Ok(Some(moved)) => return Ok(moved),
            Ok(None) => {
                outcome.updated.push(skill.id.clone());
                self.apply_library();
            }
            // A failure the journal could not undo stops every later change.
            Err(error) if self.recovery_required => return Err(error),
            Err(error) => {
                self.update_failed(&skill, error);
                outcome.failed.push(skill.id.clone());
            }
        }
        Ok(ReviewedOutcome::Updated(outcome))
    }

    /// The library skill `id`.
    fn library_skill(&self, id: &SkillId) -> AppResult<Skill> {
        self.read_library()?
            .skills
            .into_iter()
            .find(|skill| &skill.id == id)
            .ok_or_else(|| AppError::not_found("This skill is no longer in the library."))
    }

    /// The changes from `skill`'s library content to `fetched`, from `source`, noting when that
    /// content has drifted from its record, which reconciling would also note (R8).
    fn review(
        &self,
        skill: &Skill,
        source: SkillSource,
        fetched: &Inspection,
    ) -> AppResult<UpdateReview> {
        let (current, current_bytes) = payload::read(&self.skill_dir(&skill.id)?)?;
        let (_, fetched_bytes) = payload::read(&fetched.canonical_source)?;
        let (files, patch) = patch::build(
            &patch::files(&current.manifest, &current_bytes),
            &patch::files(&fetched.manifest, &fetched_bytes),
        );
        self.log
            .event(Level::Debug, Area::Update, "review built")
            .field("slug", skill.id.as_str())
            .field("files", files.len())
            .field("bytes", patch.len());
        Ok(UpdateReview {
            id: skill.id.clone(),
            source,
            from: current.manifest.digest,
            to: fetched.tree.clone(),
            drifted: current.tree != skill.recorded,
            files,
            patch,
        })
    }
}
