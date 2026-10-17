use crate::{
    discovery::ImportSource,
    library::normalized,
    library_store::SkillRecord,
    payload::{self, Inspection},
    persistence::{self, Journal, Move},
    schema::{idempotency_records, operation_plans},
    worker::State,
};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use skillbinder_proto::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

#[derive(Serialize, Deserialize)]
struct Prepared {
    review: ImportPlan,
    sources: Vec<Inspection>,
}
impl State {
    pub(crate) fn prepare_import(
        &mut self,
        ids: Vec<CandidateId>,
        acknowledge_invalid: bool,
    ) -> AppResult<ImportPlan> {
        self.require_recovered()?;
        let selected = self.scans.selected(&ids)?;
        self.prepare_reviewed(selected, acknowledge_invalid)
    }
    /// Re-inspects selected sources into an immutable review plan.
    fn prepare_reviewed(
        &mut self,
        selected: Vec<(CandidateId, ImportSource)>,
        acknowledge_invalid: bool,
    ) -> AppResult<ImportPlan> {
        let (library, revision) = self.revised_library()?;
        // Slugs by normalized name, so names that differ only in case collide, and by digest.
        let mut names: BTreeMap<String, SkillId> = BTreeMap::new();
        let mut digests: BTreeMap<String, SkillId> = BTreeMap::new();
        for skill in library.skills {
            names.insert(normalized(skill.id.as_str()), skill.id.clone());
            digests.insert(skill.digest, skill.id);
        }
        // A directory not yet reconciled still holds its name.
        for slug in self.skill_dirs()? {
            names.entry(normalized(&slug)).or_insert(SkillId(slug));
        }
        let mut batch = BTreeSet::new();
        let mut items = Vec::new();
        let mut sources = Vec::new();
        for (candidate_id, source) in selected {
            let inspection = reinspect(&source)?;
            if inspection.status == ValidationStatus::Blocked {
                return Err(AppError::validation(
                    "Blocked payloads cannot be installed.",
                ));
            }
            if inspection.status == ValidationStatus::Invalid && !acknowledge_invalid {
                return Err(AppError::validation(
                    "Acknowledge invalid skill metadata before preparing an install.",
                ));
            }
            let digest = &inspection.manifest.digest;
            let name = normalized(&inspection.slug);
            let (skill_id, decision) = match digests.get(digest) {
                Some(id) => (id.clone(), ImportDecision::Existing),
                None => {
                    if let Some(held) = names.get(&name) {
                        return Err(AppError::validation(if batch.contains(&name) {
                            "Select only one distinct payload per shared slug in an install batch."
                                .to_owned()
                        } else {
                            format!("The library already holds a different skill named {held}.")
                        }));
                    }
                    let id = SkillId(inspection.slug.clone());
                    batch.insert(name.clone());
                    names.insert(name, id.clone());
                    digests.insert(digest.clone(), id.clone());
                    (id, ImportDecision::NewSkill)
                }
            };
            items.push(ImportItem {
                candidate_id,
                destination: format!("skills/{skill_id}"),
                skill_id,
                slug: inspection.slug.clone(),
                source: source.display_path,
                decision,
                validation: inspection.status,
                messages: inspection.messages.clone(),
                total_bytes: inspection.manifest.total_bytes(),
            });
            sources.push(inspection);
        }
        let review = ImportPlan {
            id: PlanId::new(),
            expires_at: persistence::now() + 300,
            revision,
            items,
        };
        let prepared = Prepared {
            review: review.clone(),
            sources,
        };
        diesel::insert_into(operation_plans::table)
            .values((
                operation_plans::id.eq(review.id.as_str()),
                operation_plans::payload
                    .eq(serde_json::to_string(&prepared).map_err(|_| AppError::storage())?),
                operation_plans::expires_at.eq(review.expires_at as i64),
                operation_plans::consumed.eq(0),
            ))
            .execute(&mut *self.database.borrow_mut())
            .map_err(|_| AppError::storage())?;
        Ok(review)
    }
    pub(crate) fn apply_import(&mut self, id: PlanId) -> AppResult<ImportOutcome> {
        self.require_recovered()?;
        if let Some(result) = idempotency_records::table
            .find(id.as_str())
            .select(idempotency_records::result)
            .first::<String>(&mut *self.database.borrow_mut())
            .optional()
            .map_err(|_| AppError::storage())?
        {
            return serde_json::from_str(&result).map_err(|_| AppError::storage());
        }
        let serialized = operation_plans::table
            .find(id.as_str())
            .select(operation_plans::payload)
            .first::<String>(&mut *self.database.borrow_mut())
            .optional()
            .map_err(|_| AppError::storage())?
            .ok_or_else(|| AppError::stale("Install plan is unavailable. Prepare it again."))?;
        let prepared: Prepared =
            serde_json::from_str(&serialized).map_err(|_| AppError::storage())?;
        if persistence::now() >= prepared.review.expires_at {
            return Err(AppError::stale("Install plan expired. Review it again."));
        }
        if self.revised_library()?.1 != prepared.review.revision {
            return Err(AppError::stale("The library changed after install review."));
        }
        for (item, inspection) in prepared.review.items.iter().zip(&prepared.sources) {
            if fs::canonicalize(&item.source).map_err(|_| AppError::stale("SourceChanged"))?
                != inspection.canonical_source
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
            return Err(AppError::stale("The library changed after install review."));
        }
        let staging = self
            .config
            .data_dir
            .join("staging")
            .join(format!("import-{id}-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&staging).map_err(|_| AppError::storage())?;
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
        for (item, source) in prepared.review.items.iter().zip(&prepared.sources) {
            if item.decision == ImportDecision::Existing {
                outcome.existing.push(item.skill_id.clone());
                continue;
            }
            moves.push(Move {
                from: staging.join(item.skill_id.as_str()),
                to: self.library_dir().join(&item.destination),
            });
            skills.push(SkillRecord::new(
                item.skill_id.to_string(),
                source,
                Some(imported_at.clone()),
            ));
            outcome.imported.push(item.skill_id.clone());
        }
        let result = self.commit(Journal {
            id: id.to_string(),
            moves,
            completed_moves: 0,
            progress: Default::default(),
            skills,
            outcome: outcome.clone(),
        });
        if !self.recovery_required {
            let _ = fs::remove_dir_all(staging);
        }
        result?;
        Ok(outcome)
    }
}

/// Re-inspects a reviewed source, refusing one whose path or directory changed since the scan.
fn reinspect(source: &ImportSource) -> AppResult<Inspection> {
    let previous = &source.inspection;
    let resolved = fs::canonicalize(&source.display_path)
        .map_err(|_| AppError::stale("Selected source path no longer exists."))?;
    if resolved != previous.canonical_source {
        return Err(AppError::stale("Selected source path changed."));
    }
    let inspection = payload::inspect(&previous.canonical_source)?;
    if inspection.identity != previous.identity {
        return Err(AppError::stale("A selected source directory was replaced."));
    }
    Ok(inspection)
}
