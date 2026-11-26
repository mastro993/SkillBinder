pub(crate) mod ignored;
pub(crate) mod placement;
mod plan;

use crate::{
    library::normalized,
    library_store::SavedSkill,
    payload::{self, Inspection},
    persistence::{self, Journal, Move, RecordChange},
    records::Record,
    scan::ImportSource,
    schema::{idempotency_records, operation_plans},
    storage,
    worker::State,
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use skillbinder_proto::*;
use std::{collections::BTreeSet, fs};

#[derive(Serialize, Deserialize)]
pub(crate) struct Prepared {
    review: ImportPlan,
    sources: Vec<Inspection>,
    /// Where each row's skill was published, as its lock file recorded at review.
    #[serde(default)]
    origins: Vec<Option<SkillSource>>,
    /// Where each row's skill becomes active; a plan stored before imports activated has none.
    #[serde(default)]
    placements: Vec<placement::Placement>,
}
impl Prepared {
    /// The plan narrowed to the ticked skills `selection`, in plan order: between 1 and
    /// [`MAX_IMPORT`] distinct skills of the plan, and an invalid one only when acknowledged.
    fn select(self, selection: &[CandidateId], acknowledge_invalid: bool) -> AppResult<Self> {
        if selection.is_empty() {
            return Err(AppError::validation("Choose skills to import."));
        }
        if selection.len() > MAX_IMPORT {
            return Err(AppError::validation("Import at most 500 at a time."));
        }
        let ticked: BTreeSet<&CandidateId> = selection.iter().collect();
        if ticked.len() != selection.len() {
            return Err(AppError::validation("Choose each skill once."));
        }
        let items = &self.review.items;
        if ticked
            .iter()
            .any(|id| !items.iter().any(|item| &item.candidate_id == *id))
        {
            return Err(AppError::validation(
                "A chosen skill is not in this import. Scan again.",
            ));
        }
        let keep: Vec<bool> = items
            .iter()
            .map(|item| ticked.contains(&item.candidate_id))
            .collect();
        if items
            .iter()
            .zip(&keep)
            .any(|(item, kept)| *kept && item.validation == ValidationStatus::Invalid)
            && !acknowledge_invalid
        {
            return Err(AppError::validation(
                "Acknowledge invalid skill metadata before importing.",
            ));
        }
        let Self {
            mut review,
            sources,
            origins,
            placements,
        } = self;
        let kept = |index: usize| keep.get(index).copied().unwrap_or(false);
        review.items = narrow(review.items, kept);
        Ok(Self {
            review,
            sources: narrow(sources, kept),
            origins: narrow(origins, kept),
            placements: narrow(placements, kept),
        })
    }
}

/// The entries of `list` whose index `kept` keeps.
fn narrow<T>(list: Vec<T>, kept: impl Fn(usize) -> bool) -> Vec<T> {
    list.into_iter()
        .enumerate()
        .filter_map(|(index, entry)| kept(index).then_some(entry))
        .collect()
}

impl State {
    /// The stored plan `id`.
    pub(crate) fn prepared(&self, id: &PlanId) -> AppResult<Prepared> {
        let serialized = operation_plans::table
            .find(id.as_str())
            .select(operation_plans::payload)
            .first::<String>(&mut *self.database.borrow_mut())
            .optional()
            .map_err(storage::error("reading an import plan"))?
            .ok_or_else(|| AppError::stale("Import plan is unavailable. Prepare it again."))?;
        serde_json::from_str(&serialized).map_err(storage::error("parsing an import plan"))
    }
    /// Applies plan `id` to the ticked skills `selection`, re-inspecting only those, so a change
    /// to a skill left out never refuses it. A plan already applied returns its stored result.
    pub(crate) fn apply_import(
        &mut self,
        id: PlanId,
        selection: Vec<CandidateId>,
        acknowledge_invalid: bool,
    ) -> AppResult<ImportOutcome> {
        self.require_recovered()?;
        if let Some(result) = idempotency_records::table
            .find(id.as_str())
            .select(idempotency_records::result)
            .first::<String>(&mut *self.database.borrow_mut())
            .optional()
            .map_err(storage::error("reading an import result"))?
        {
            return serde_json::from_str(&result)
                .map_err(storage::error("parsing an import result"));
        }
        let prepared = self
            .prepared(&id)?
            .select(&selection, acknowledge_invalid)?;
        if persistence::now() >= prepared.review.expires_at {
            return Err(AppError::stale("Import plan expired. Review it again."));
        }
        // The revision covers the active preset, so the one at review is still active.
        let (library, revision) = self.revised_library()?;
        if revision != prepared.review.revision {
            return Err(AppError::stale("The library changed after import review."));
        }
        for (item, inspection) in prepared.review.items.iter().zip(&prepared.sources) {
            if dunce::canonicalize(&item.source).map_err(|error| {
                AppError::stale("SourceChanged")
                    .because(format_args!("resolving {}: {error}", item.source))
            })? != inspection.canonical_source
                || payload::inspect(&inspection.canonical_source)? != *inspection
            {
                return Err(AppError::stale("SourceChanged"));
            }
        }
        // A directory that a failed reconcile left out of the database may have appeared since
        // review; names that differ only in case collide on some file systems.
        let taken: BTreeSet<_> = self.skill_dirs()?.iter().map(|s| normalized(s)).collect();
        if prepared.review.items.iter().any(|item| {
            item.decision == ImportDecision::NewSkill
                && taken.contains(&normalized(item.skill_id.as_str()))
        }) {
            return Err(AppError::stale("The library changed after import review."));
        }
        let staging = self
            .config
            .data_dir
            .join("staging")
            .join(format!("import-{id}-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&staging).map_err(storage::error_at("creating", &staging))?;
        let staged = (|| {
            for (item, source) in prepared.review.items.iter().zip(&prepared.sources) {
                if item.decision == ImportDecision::NewSkill {
                    payload::materialize(source, &staging.join(item.skill_id.as_str()))?;
                }
            }
            // A source staged early may have changed while later ones were copied.
            for source in &prepared.sources {
                if payload::inspect(&source.canonical_source)? != *source {
                    return Err(AppError::stale("SourceChanged"));
                }
            }
            Ok(())
        })();
        if let Err(error) = staged {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
        let imported_at = crate::lifecycle::format_timestamp();
        let mut moves = Vec::new();
        let mut skills = Vec::new();
        let mut outcome = ImportOutcome {
            plan_id: id.clone(),
            imported: Vec::new(),
            existing: Vec::new(),
        };
        let mut origins = prepared.origins.iter();
        let mut placements = prepared.placements.iter();
        let mut records = Vec::new();
        let mut held = Vec::new();
        for (item, source) in prepared.review.items.iter().zip(&prepared.sources) {
            let origin = origins.next().cloned().flatten();
            let placement = placements.next().cloned().unwrap_or_default();
            if item.decision == ImportDecision::Existing {
                outcome.existing.push(item.skill_id.clone());
                continue;
            }
            let slug = item.skill_id.as_str();
            held.extend(placement::held(&library.active_preset, slug, placement));
            let record = Record::of(source, origin);
            records.push(RecordChange {
                slug: item.skill_id.to_string(),
                record: Some(record.clone()),
                previous: None,
            });
            moves.push(Move {
                from: staging.join(item.skill_id.as_str()),
                to: self.library_dir().join(&item.destination),
            });
            skills.push(SavedSkill::new(
                item.skill_id.to_string(),
                source,
                Some(imported_at.clone()),
                record,
                None,
            ));
            outcome.imported.push(item.skill_id.clone());
        }
        let result = self.commit(Journal {
            id: id.to_string(),
            moves,
            completed_moves: 0,
            progress: Default::default(),
            skills,
            outcome: Some(outcome.clone()),
            records,
            held,
        });
        if !self.recovery_required {
            let _ = fs::remove_dir_all(staging);
        }
        result?;
        // The skills are saved where they are active; their copies follow.
        self.apply_imported(outcome.imported.iter().map(|id| id.0.clone()).collect());
        // The import that first fills the library uses up the one-time adoption.
        self.adopt_agent_copies();
        Ok(outcome)
    }
}

/// Re-inspects a reviewed source, refusing one whose path or directory changed since the scan.
pub(super) fn reinspect(source: &ImportSource) -> AppResult<Inspection> {
    let previous = &source.inspection;
    let resolved = dunce::canonicalize(&source.display_path).map_err(|error| {
        AppError::stale("Selected source path no longer exists.")
            .because(format_args!("resolving {}: {error}", source.display_path))
    })?;
    if resolved != previous.canonical_source {
        return Err(AppError::stale("Selected source path changed."));
    }
    let inspection = payload::inspect(&previous.canonical_source)?;
    if inspection.identity != previous.identity {
        return Err(AppError::stale("A selected source directory was replaced."));
    }
    Ok(inspection)
}
