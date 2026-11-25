//! Preparing an import plan (ADR 0009): Scan for skills plans every found skill that is not in
//! library from a fresh scan, and choosing another version of one skill plans that skill alone
//! again, keeping the first plan's library revision and expiry.

use super::{Prepared, placement, reinspect};
use crate::{
    library::normalized,
    payload::Inspection,
    persistence,
    scan::{Found, ImportSource, Selected},
    schema::operation_plans,
    storage,
    worker::State,
};
use diesel::prelude::*;
use skillbinder_proto::*;
use std::collections::{BTreeMap, BTreeSet};

/// How long a prepared plan can be applied.
const PLAN_LIFETIME: u64 = 300;

/// One planned skill: its row, the inspection it imports, where it was published, and where it
/// becomes active.
struct Planned {
    item: ImportItem,
    source: Inspection,
    origin: Option<SkillSource>,
    placement: placement::Placement,
}

/// What planning reads of the library: the slugs and contents it holds, and where a new skill
/// can become active.
struct Planner {
    revision: String,
    agents: Vec<AgentOption>,
    workspaces: Vec<Workspace>,
    /// Slugs by normalized name, so names that differ only in case collide.
    names: BTreeMap<String, SkillId>,
    digests: BTreeMap<String, SkillId>,
    /// The normalized names this plan imports.
    batch: BTreeSet<String>,
}

impl State {
    /// Plans every found skill of `found` that is not in library: the version `versions` chooses
    /// for it, else the preferred one. A skill whose versions are all blocked or empty is left
    /// out; an invalid one is planned and marked.
    pub(crate) fn prepare_found(&mut self, versions: &[CandidateId]) -> AppResult<FoundImport> {
        self.require_recovered()?;
        if self.shutdown.cancelled() {
            return Err(crate::shutdown::Shutdown::error());
        }
        let held = self.held_names()?;
        let (scan, offered) = self.scans.last(|found| {
            if let Some(error) = &found.snapshot.error {
                return Err(AppError::new(
                    ErrorCategory::Storage,
                    error.clone(),
                    "Try scanning again.",
                ));
            }
            let ids = offered(found, versions, &held)?;
            Ok((found.snapshot.clone(), found.selected(&ids)?))
        })?;
        let plan = self.prepare_plan(offered)?;
        Ok(FoundImport { scan, plan })
    }

    /// Plans `version` in place of the version plan `id` holds of the same skill, carrying every
    /// other item over, under a new plan id with the first plan's revision and expiry.
    pub(crate) fn prepare_version(
        &mut self,
        id: &PlanId,
        version: CandidateId,
    ) -> AppResult<ImportPlan> {
        self.require_recovered()?;
        let mut prepared = self.prepared(id)?;
        let mut selected = self.scans.selected(&[version])?;
        let found = selected.pop().ok_or_else(AppError::storage)?;
        let key = normalized(&found.source.inspection.slug);
        let index = prepared
            .review
            .items
            .iter()
            .position(|item| normalized(&item.slug) == key)
            .ok_or_else(|| AppError::validation("Choose a version of a skill in this import."))?;
        let mut planner = self.planner()?;
        let planned = planner
            .plan(&found)?
            .ok_or_else(|| AppError::validation("This version cannot be imported."))?;
        prepared.review.items[index] = planned.item;
        prepared.sources[index] = planned.source;
        if let Some(origin) = prepared.origins.get_mut(index) {
            *origin = planned.origin;
        }
        if let Some(placement) = prepared.placements.get_mut(index) {
            *placement = planned.placement;
        }
        prepared.review.id = PlanId::new();
        self.store(&prepared)?;
        Ok(prepared.review)
    }

    /// Re-inspects `selected` into an immutable plan, with where each new skill becomes active
    /// and the copies of other versions it replaces there. A source that turned blocked or empty
    /// or changed since the scan is left out.
    pub(super) fn prepare_plan(&mut self, selected: Vec<Selected>) -> AppResult<ImportPlan> {
        let mut planner = self.planner()?;
        let mut prepared = Prepared {
            review: ImportPlan {
                id: PlanId::new(),
                expires_at: persistence::now() + PLAN_LIFETIME,
                revision: planner.revision.clone(),
                items: Vec::new(),
            },
            sources: Vec::new(),
            origins: Vec::new(),
            placements: Vec::new(),
        };
        for found in &selected {
            if let Some(planned) = planner.plan(found)? {
                prepared.review.items.push(planned.item);
                prepared.sources.push(planned.source);
                prepared.origins.push(planned.origin);
                prepared.placements.push(planned.placement);
            }
        }
        self.store(&prepared)?;
        Ok(prepared.review)
    }

    /// The normalized slugs the library holds, including directories not yet reconciled.
    pub(crate) fn held_names(&self) -> AppResult<BTreeSet<String>> {
        let library = self.read_library()?;
        Ok(library
            .skills
            .into_iter()
            .map(|skill| skill.id.0)
            .chain(self.skill_dirs()?)
            .map(|slug| normalized(&slug))
            .collect())
    }

    fn planner(&self) -> AppResult<Planner> {
        let (library, revision) = self.revised_library()?;
        let agents = self.agents()?;
        let workspaces = self.workspaces_for(&library, &agents)?;
        let mut names = BTreeMap::new();
        let mut digests = BTreeMap::new();
        for skill in library.skills {
            names.insert(normalized(skill.id.as_str()), skill.id.clone());
            digests.insert(skill.digest, skill.id);
        }
        // A directory not yet reconciled still holds its name.
        for slug in self.skill_dirs()? {
            names.entry(normalized(&slug)).or_insert(SkillId(slug));
        }
        Ok(Planner {
            revision,
            agents,
            workspaces,
            names,
            digests,
            batch: BTreeSet::new(),
        })
    }

    /// Stores `prepared`, first removing expired plans: nothing applies them, and replaying a
    /// completed import reads its idempotency record, not its plan.
    fn store(&self, prepared: &Prepared) -> AppResult<()> {
        let review = &prepared.review;
        let now = i64::try_from(persistence::now()).map_err(storage::error("reading the clock"))?;
        diesel::delete(operation_plans::table.filter(operation_plans::expires_at.lt(now)))
            .execute(&mut *self.database.borrow_mut())
            .map_err(storage::error("removing expired import plans"))?;
        diesel::insert_into(operation_plans::table)
            .values((
                operation_plans::id.eq(review.id.as_str()),
                operation_plans::payload.eq(serde_json::to_string(prepared)
                    .map_err(storage::error("serializing an import plan"))?),
                operation_plans::expires_at.eq(review.expires_at as i64),
                operation_plans::consumed.eq(0),
            ))
            .execute(&mut *self.database.borrow_mut())
            .map_err(storage::error("saving an import plan"))?;
        Ok(())
    }
}

impl Planner {
    /// Re-inspects `found` into a planned row, or `None` when [`plannable`] leaves it out.
    fn plan(&mut self, found: &Selected) -> AppResult<Option<Planned>> {
        let source = &found.source;
        let Some(inspection) = plannable(source)? else {
            return Ok(None);
        };
        let digest = &inspection.manifest.digest;
        let name = normalized(&inspection.slug);
        let (skill_id, decision) = match self.digests.get(digest) {
            Some(id) => (id.clone(), ImportDecision::Existing),
            None => {
                if let Some(held) = self.names.get(&name) {
                    return Err(AppError::validation(if self.batch.contains(&name) {
                        format!("Choose one version of {held} to import.")
                    } else {
                        format!("The library already holds a different skill named {held}.")
                    }));
                }
                let id = SkillId(inspection.slug.clone());
                self.batch.insert(name.clone());
                self.names.insert(name, id.clone());
                self.digests.insert(digest.clone(), id.clone());
                (id, ImportDecision::NewSkill)
            }
        };
        // A skill already in library stays where it is active; only a new one is placed.
        let placement = match decision {
            ImportDecision::NewSkill => placement::place(
                skill_id.as_str(),
                digest,
                found,
                &self.agents,
                &self.workspaces,
            ),
            ImportDecision::Existing => placement::Placement::default(),
        };
        let item = ImportItem {
            candidate_id: found.id.clone(),
            destination: format!("skills/{skill_id}"),
            skill_id,
            slug: inspection.slug.clone(),
            source: source.display_path.clone(),
            decision,
            validation: inspection.status,
            messages: inspection.messages.clone(),
            total_bytes: inspection.manifest.total_bytes(),
            replaces: placement.replaced.keys().cloned().collect(),
        };
        Ok(Some(Planned {
            item,
            origin: crate::sources::lock::source(&source.locks, &inspection.slug),
            source: inspection,
            placement,
        }))
    }
}

/// The version of each found skill not in library that the plan takes: the one `versions`
/// chooses while it is importable, else the preferred one. Two chosen versions of one skill are
/// refused.
fn offered(
    found: &Found,
    versions: &[CandidateId],
    held: &BTreeSet<String>,
) -> AppResult<Vec<CandidateId>> {
    let candidates = &found.snapshot.candidates;
    let importable = |candidate: &&Candidate| {
        candidate.validation != ValidationStatus::Blocked && candidate.file_count > 0
    };
    let mut chosen: BTreeMap<&str, &Candidate> = BTreeMap::new();
    for version in versions {
        let Some(candidate) = candidates
            .iter()
            .filter(importable)
            .find(|c| &c.id == version)
        else {
            continue;
        };
        if chosen.insert(&candidate.key, candidate).is_some() {
            return Err(AppError::validation(format!(
                "Choose one version of {} to import.",
                candidate.slug
            )));
        }
    }
    let mut ids = Vec::new();
    for candidate in candidates {
        let offered = candidate.in_library().is_none() && !held.contains(&candidate.key);
        let picked = match chosen.get(candidate.key.as_str()) {
            Some(choice) => choice.id == candidate.id,
            None => candidate.preferred,
        };
        if offered && picked {
            ids.push(candidate.id.clone());
        }
    }
    Ok(ids)
}

/// Re-inspects `source` for a plan, or `None` when it turned blocked or empty since the scan
/// (Git tracks files, not directories, so a skill without files would never sync) or no longer
/// is the directory the scan found. Leaving such a skill out keeps it from failing the others.
fn plannable(source: &ImportSource) -> AppResult<Option<Inspection>> {
    let inspection = match reinspect(source) {
        Ok(inspection) => inspection,
        Err(error) if error.category == ErrorCategory::Stale => return Ok(None),
        Err(error) => return Err(error),
    };
    let importable =
        inspection.status != ValidationStatus::Blocked && inspection.manifest.file_count() > 0;
    Ok(importable.then_some(inspection))
}

#[cfg(test)]
mod tests;
