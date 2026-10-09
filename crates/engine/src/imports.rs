use crate::{
    library::{payload_relative, validate_library},
    payload::{self, Inspection, Manifest},
    persistence::{self, Journal, Move},
    worker::State,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skillbinder_proto::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

#[derive(Serialize, Deserialize)]
struct Prepared {
    review: ImportPlan,
    after: Library,
    sources: Vec<Inspection>,
    readers: Vec<Vec<String>>,
    catalog: String,
}
impl State {
    pub(crate) fn catalog(
        &self,
        library: &Library,
    ) -> AppResult<(BTreeMap<SkillId, Manifest>, String)> {
        let mut manifests = BTreeMap::new();
        for skill in &library.skills {
            let path = self
                .contained_skill_path(&self.library_dir().join(payload_relative(library, skill)))?;
            let inspection = payload::inspect(&path)?;
            if inspection.manifest.digest != skill.digest {
                return Err(AppError::stale(
                    "A managed payload changed outside SkillBinder. Review it before importing.",
                ));
            }
            manifests.insert(skill.id.clone(), inspection.manifest);
        }
        let fingerprint = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&manifests).map_err(|_| AppError::storage())?)
        );
        Ok((manifests, fingerprint))
    }
    pub(crate) fn prepare_import(
        &mut self,
        scan: ScanId,
        ids: Vec<CandidateId>,
        acknowledge_invalid: bool,
    ) -> AppResult<ImportPlan> {
        self.require_recovered()?;
        let selected = self.scans.selected(&scan, &ids)?;
        let snapshot = self.library_snapshot()?;
        let (mut catalog, fingerprint) = self.catalog(&snapshot.library)?;
        let mut after = snapshot.library.clone();
        let mut items = Vec::new();
        let mut sources = Vec::new();
        let mut readers = Vec::new();
        let mut selected_slugs = BTreeMap::new();
        for (candidate, previous) in selected {
            let resolved = fs::canonicalize(&candidate.display_path)
                .map_err(|_| AppError::stale("Selected source path no longer exists."))?;
            if resolved != previous.canonical_source {
                return Err(AppError::stale("Selected source path changed."));
            }
            let inspection = payload::inspect(&previous.canonical_source)?;
            if inspection.identity != previous.identity {
                return Err(AppError::stale("A selected source directory was replaced."));
            }
            if inspection.status == ValidationStatus::Blocked {
                return Err(AppError::validation("Blocked payloads cannot be imported."));
            }
            if inspection.status == ValidationStatus::Invalid && !acknowledge_invalid {
                return Err(AppError::validation(
                    "Acknowledge invalid skill metadata before preparing an import.",
                ));
            }
            if selected_slugs
                .get(&inspection.slug)
                .is_some_and(|digest| digest != &inspection.manifest.digest)
            {
                return Err(AppError::validation(
                    "Select only one distinct payload per shared slug in an import batch.",
                ));
            }
            selected_slugs.insert(inspection.slug.clone(), inspection.manifest.digest.clone());
            let existing = catalog
                .iter()
                .find(|(_, manifest)| manifest.equivalent(&inspection.manifest))
                .map(|(id, _)| id.clone());
            let (skill_id, decision) = if let Some(id) = existing {
                (id, ImportDecision::AttachObservation)
            } else {
                let id = SkillId::new();
                let conflict = after
                    .skills
                    .iter()
                    .any(|skill| skill.slug == inspection.slug);
                after.skills.push(Skill {
                    id: id.clone(),
                    slug: inspection.slug.clone(),
                    display_name: None,
                    folder_id: None,
                    tag_ids: Vec::new(),
                    upstream_bindings: Vec::new(),
                    digest: inspection.manifest.digest.clone(),
                    file_count: inspection.manifest.file_count(),
                    total_bytes: inspection.manifest.total_bytes(),
                });
                catalog.insert(id.clone(), inspection.manifest.clone());
                (
                    id,
                    if conflict {
                        ImportDecision::Conflict
                    } else {
                        ImportDecision::NewSkill
                    },
                )
            };
            items.push(ImportItem {
                candidate_id: candidate.id,
                skill_id,
                slug: inspection.slug.clone(),
                source: candidate.display_path,
                destination: String::new(),
                decision,
                validation: inspection.status,
                messages: inspection.messages.clone(),
                total_bytes: inspection.manifest.total_bytes(),
            });
            readers.push(candidate.readers);
            sources.push(inspection);
        }
        validate_library(&after)?;
        for item in &mut items {
            let skill = after
                .skills
                .iter()
                .find(|skill| skill.id == item.skill_id)
                .ok_or_else(AppError::storage)?;
            item.destination = payload_relative(&after, skill);
        }
        let review = ImportPlan {
            id: PlanId::new(),
            expires_at: persistence::now() + 300,
            revision: snapshot.revision,
            items,
        };
        let prepared = Prepared {
            review: review.clone(),
            after,
            sources,
            readers,
            catalog: fingerprint,
        };
        self.database
            .execute(
                "INSERT INTO operation_plans(id,payload,expires_at,consumed) VALUES(?1,?2,?3,0)",
                params![
                    review.id.as_str(),
                    serde_json::to_string(&prepared).map_err(|_| AppError::storage())?,
                    review.expires_at
                ],
            )
            .map_err(|_| AppError::storage())?;
        Ok(review)
    }
    pub(crate) fn apply_import(&mut self, id: PlanId) -> AppResult<ImportOutcome> {
        self.require_recovered()?;
        if let Some(result) = self
            .database
            .query_row(
                "SELECT result FROM idempotency_records WHERE operation_id=?1",
                [id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| AppError::storage())?
        {
            return serde_json::from_str(&result).map_err(|_| AppError::storage());
        }
        let serialized = self
            .database
            .query_row(
                "SELECT payload FROM operation_plans WHERE id=?1",
                [id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| AppError::storage())?
            .ok_or_else(|| AppError::stale("Import plan is unavailable. Prepare it again."))?;
        let prepared: Prepared =
            serde_json::from_str(&serialized).map_err(|_| AppError::storage())?;
        if persistence::now() >= prepared.review.expires_at {
            return Err(AppError::stale("Import plan expired. Review it again."));
        }
        let snapshot = self.library_snapshot()?;
        if snapshot.revision != prepared.review.revision
            || self.catalog(&snapshot.library)?.1 != prepared.catalog
        {
            return Err(AppError::stale("The library changed after import review."));
        }
        for (item, inspection) in prepared.review.items.iter().zip(&prepared.sources) {
            if fs::canonicalize(&item.source).map_err(|_| AppError::stale("SourceChanged"))?
                != inspection.canonical_source
                || payload::inspect(&inspection.canonical_source)? != *inspection
            {
                return Err(AppError::stale("SourceChanged"));
            }
        }
        let staging = self
            .config
            .data_dir
            .join("staging")
            .join(format!("import-{id}-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&staging).map_err(|_| AppError::storage())?;
        let staged = (|| {
            for (item, source) in prepared.review.items.iter().zip(&prepared.sources) {
                if item.decision != ImportDecision::AttachObservation {
                    payload::materialize(source, &staging.join(item.skill_id.as_str()))?;
                }
            }
            for inspection in &prepared.sources {
                if payload::inspect(&inspection.canonical_source)? != *inspection {
                    return Err(AppError::stale("SourceChanged"));
                }
            }
            Ok(())
        })();
        if let Err(error) = staged {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
        if self.library_snapshot()?.revision != prepared.review.revision
            || self.catalog(&snapshot.library)?.1 != prepared.catalog
        {
            let _ = fs::remove_dir_all(&staging);
            return Err(AppError::stale(
                "The library changed while the import was staged.",
            ));
        }
        let mut moves = Vec::new();
        for before in &snapshot.library.skills {
            let after = prepared
                .after
                .skills
                .iter()
                .find(|skill| skill.id == before.id)
                .ok_or_else(AppError::storage)?;
            let from = self
                .library_dir()
                .join(payload_relative(&snapshot.library, before));
            let to = self
                .library_dir()
                .join(payload_relative(&prepared.after, after));
            if from != to {
                moves.push(Move { from, to });
            }
        }
        let mut details = snapshot.details;
        let mut outcome = ImportOutcome {
            plan_id: id.clone(),
            imported: Vec::new(),
            attached: Vec::new(),
            conflicts: Vec::new(),
        };
        let mut touched = BTreeSet::new();
        for ((item, source), readers) in prepared
            .review
            .items
            .iter()
            .zip(&prepared.sources)
            .zip(&prepared.readers)
        {
            if item.decision == ImportDecision::AttachObservation {
                outcome.attached.push(item.skill_id.clone());
            } else {
                moves.push(Move {
                    from: staging.join(item.skill_id.as_str()),
                    to: self.library_dir().join(&item.destination),
                });
                outcome.imported.push(item.skill_id.clone());
                if item.decision == ImportDecision::Conflict {
                    outcome.conflicts.push(item.slug.clone());
                }
            }
            let local = details.entry(item.skill_id.clone()).or_default();
            local.description = source.description.clone();
            local.validation = source.status;
            local.messages = source.messages.clone();
            local.readers.extend(readers.clone());
            local.readers.sort();
            local.readers.dedup();
            local.sources.push(item.source.clone());
            local.sources.sort();
            local.sources.dedup();
            touched.insert(item.skill_id.clone());
        }
        let result = self.commit(Journal {
            id: id.to_string(),
            before: snapshot.library,
            after: prepared.after,
            moves,
            completed_moves: 0,
            progress: Default::default(),
            details: details
                .into_iter()
                .filter(|(id, _)| touched.contains(id))
                .collect(),
            outcome: Some(outcome.clone()),
        });
        if !self.recovery_required {
            let _ = fs::remove_dir_all(staging);
        }
        result?;
        Ok(outcome)
    }
}
