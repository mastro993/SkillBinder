use super::model::*;
use crate::{
    discovery::scan::PayloadSource,
    library::{PayloadModel, ValidationLimits, ValidationStatus, inspect_payload},
};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

pub trait PlanStore: Send + Sync {
    fn create(&self, plan: &ImportPlan) -> Result<(), ImportError>;
    fn load(&self, id: &str) -> Result<Option<ImportPlan>, ImportError>;
    fn consume(&self, id: &str, result: &ImportResult) -> Result<(), ImportError>;
    fn expire(&self, now: u64) -> Result<(), ImportError>;
    fn idempotency_lookup(
        &self,
        operation_id: &str,
    ) -> Result<Option<(String, ImportResult)>, ImportError>;
    fn idempotency_store(
        &self,
        operation_id: &str,
        request_hash: &str,
        result: &ImportResult,
    ) -> Result<(), ImportError>;
    fn consume_and_store(
        &self,
        id: &str,
        request_hash: &str,
        result: &ImportResult,
    ) -> Result<(), ImportError> {
        self.consume(id, result)?;
        self.idempotency_store(id, request_hash, result)
    }
}
pub trait ObservationStore: Send + Sync {
    fn append(&self, observation: &SourceObservation) -> Result<(), ImportError>;
    fn list(&self, skill_id: &str) -> Result<Vec<SourceObservation>, ImportError>;
    fn rollback_observation(&self, _observation: &SourceObservation) -> Result<(), ImportError> {
        Ok(())
    }
    fn index_metadata(
        &self,
        _skill_id: &str,
        _metadata: &IndexedSkillMetadata,
    ) -> Result<(), ImportError> {
        Ok(())
    }
    fn remove_index_metadata(&self, _skill_id: &str) -> Result<(), ImportError> {
        Ok(())
    }
    fn indexed_metadata(
        &self,
        _skill_id: &str,
    ) -> Result<Option<IndexedSkillMetadata>, ImportError> {
        Ok(None)
    }
}
pub trait LibraryRepository: Send + Sync {
    fn catalog(&self) -> Result<Vec<LibraryRecord>, ImportError>;
    fn stage_payload(
        &self,
        plan_id: &str,
        skill_id: &str,
        model: &PayloadModel,
    ) -> Result<(), ImportError>;
    fn verify_staged_manifest(
        &self,
        plan_id: &str,
        skill_id: &str,
        manifest: &crate::library::Manifest,
    ) -> Result<(), ImportError>;
    fn move_staged_payload(
        &self,
        plan_id: &str,
        skill_id: &str,
        slug: &str,
    ) -> Result<(), ImportError>;
    fn write_record_and_manifest(
        &self,
        skill_id: &str,
        slug: &str,
        model: &PayloadModel,
    ) -> Result<(), ImportError>;
    fn remove_record_and_manifest(&self, _skill_id: &str) -> Result<(), ImportError> {
        Ok(())
    }
    fn rollback_import(
        &self,
        _plan_id: &str,
        _moved: &[(String, String)],
    ) -> Result<(), ImportError> {
        self.delete_staged(_plan_id)
    }
    fn delete_staged(&self, plan_id: &str) -> Result<(), ImportError>;
    fn current_revision(&self) -> Result<Option<String>, ImportError>;
    fn has_uncommitted_changes(&self) -> Result<bool, ImportError>;
    fn write_import_journal(&self, _plan: &ImportPlan) -> Result<(), ImportError> {
        Ok(())
    }
    fn remove_import_journal(&self, _plan_id: &str) -> Result<(), ImportError> {
        Ok(())
    }
    fn unresolved_import_journal(&self) -> Result<Option<String>, ImportError> {
        Ok(None)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryRecord {
    pub skill_id: String,
    pub slug: String,
    pub digest: String,
    pub manifest: crate::library::Manifest,
}
pub trait Clock: Send + Sync {
    fn now_seconds(&self) -> u64;
}
pub trait IdSource: Send + Sync {
    fn next_id(&self) -> String;
}
pub struct SystemClock;
impl Clock for SystemClock {
    fn now_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}
pub struct UuidSource;
impl IdSource for UuidSource {
    fn next_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ImportError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("unsupported skill: {0}")]
    UnsupportedSkill(String),
    #[error("source changed")]
    SourceChanged,
    #[error("stale or expired plan")]
    StalePlan,
    #[error("idempotency conflict")]
    IdempotencyConflict,
    #[error("recovery required")]
    RecoveryRequired,
    #[error("database unavailable: {0}")]
    Database(String),
    #[error("library unavailable: {0}")]
    Library(String),
    #[error("internal import error: {0}")]
    Internal(String),
}

pub struct ImportService {
    pub source: std::sync::Arc<dyn PayloadSource>,
    pub plans: std::sync::Arc<dyn PlanStore>,
    pub library: std::sync::Arc<dyn LibraryRepository>,
    pub observations: std::sync::Arc<dyn ObservationStore>,
    pub clock: std::sync::Arc<dyn Clock>,
    pub ids: std::sync::Arc<dyn IdSource>,
    pub limits: ValidationLimits,
}
impl ImportService {
    pub fn prepare(
        &self,
        selections: Vec<ImportSelection>,
        allow_invalid_skills: bool,
        snapshot: ImportSnapshot,
    ) -> Result<ImportPlan, ImportError> {
        let now = self.clock.now_seconds();
        self.plans.expire(now)?;
        let revision = self.library.current_revision()?;
        if revision != snapshot.library_revision {
            return Err(ImportError::StalePlan);
        }
        let catalog = self.library.catalog()?;
        let catalog_fingerprint = fingerprint_catalog(&catalog);
        let id = self.ids.next_id();
        let request_hash = request_hash(&snapshot, &selections);
        let mut items = Vec::new();
        let mut planned = Vec::<(String, crate::library::Manifest, String)>::new();
        for mut selection in selections {
            let model = inspect_payload(&selection.source, self.source.as_ref(), &self.limits)
                .map_err(|e| ImportError::UnsupportedSkill(e.to_string()))?;
            selection.validation = model.validation.clone();
            let exclusions = model
                .validation
                .messages
                .iter()
                .filter(|message| {
                    message.code == crate::library::ValidationCode::VcsMetadataExcluded
                })
                .map(|message| message.message.clone())
                .collect();
            if model.validation.status == ValidationStatus::Blocked {
                return Err(ImportError::UnsupportedSkill(format!(
                    "{}: {}",
                    selection.slug,
                    model
                        .validation
                        .messages
                        .iter()
                        .map(|m| format!("{:?}", m.code))
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
            if model.validation.status == ValidationStatus::Invalid && !allow_invalid_skills {
                return Err(ImportError::Validation(format!(
                    "{} has invalid validation",
                    selection.slug
                )));
            }
            let same: Vec<_> = catalog
                .iter()
                .filter(|r| {
                    r.digest == model.manifest.digest && r.manifest.equivalent(&model.manifest)
                })
                .collect();
            let planned_match = planned
                .iter()
                .find(|(digest, manifest, _)| {
                    digest == &model.manifest.digest && manifest.equivalent(&model.manifest)
                })
                .map(|(_, _, skill_id)| skill_id.clone());
            let decision = if let Some(skill_id) = planned_match {
                ImportDecision::AttachObservation { skill_id }
            } else if same.len() == 1 {
                ImportDecision::AttachObservation {
                    skill_id: same[0].skill_id.clone(),
                }
            } else if same.len() > 1 {
                return Err(ImportError::Validation(
                    "multiple identical library entries require a choice".into(),
                ));
            } else {
                ImportDecision::NewSkill {
                    skill_id: self.ids.next_id(),
                }
            };
            let skill_id = match &decision {
                ImportDecision::NewSkill { skill_id }
                | ImportDecision::AttachObservation { skill_id } => skill_id.clone(),
                ImportDecision::Conflict { .. } => unreachable!("conflicts rejected above"),
            };
            if matches!(decision, ImportDecision::NewSkill { .. }) {
                planned.push((
                    model.manifest.digest.clone(),
                    model.manifest.clone(),
                    skill_id.clone(),
                ));
            }
            items.push(ImportPlanItem {
                selection,
                skill_id,
                decision,
                manifest: model.manifest,
                exclusions,
            });
        }
        let plan = ImportPlan {
            id,
            expires_at: now + 300,
            request_hash,
            library_revision: revision,
            catalog_fingerprint,
            allow_invalid_skills,
            items,
        };
        self.plans.create(&plan)?;
        Ok(plan)
    }
    pub fn apply(&self, plan_id: &str) -> Result<ImportResult, ImportError> {
        if let Some((_request_hash, result)) = self.plans.idempotency_lookup(plan_id)? {
            return Ok(result);
        }
        let plan = self.plans.load(plan_id)?.ok_or(ImportError::StalePlan)?;
        if self.clock.now_seconds() > plan.expires_at {
            return Err(ImportError::StalePlan);
        }
        let catalog = self.library.catalog()?;
        let current_revision = self.library.current_revision()?;
        if current_revision != plan.library_revision
            || fingerprint_catalog(&catalog) != plan.catalog_fingerprint
        {
            return Err(ImportError::StalePlan);
        }
        let mut staged = Vec::new();
        for item in &plan.items {
            if self
                .source
                .entry_metadata(&item.selection.source)
                .is_ok_and(|metadata| metadata.kind != crate::discovery::scan::EntryKind::Directory)
            {
                let _ = self.library.delete_staged(plan_id);
                return Err(ImportError::SourceChanged);
            }
            let model = inspect_payload(&item.selection.source, self.source.as_ref(), &self.limits)
                .map_err(|_| ImportError::SourceChanged)?;
            if !model.manifest.equivalent(&item.manifest) {
                let _ = self.library.delete_staged(plan_id);
                return Err(ImportError::SourceChanged);
            }
            if let Err(error) = self.library.stage_payload(plan_id, &item.skill_id, &model) {
                let _ = self.library.delete_staged(plan_id);
                return Err(error);
            }
            if let Err(error) =
                self.library
                    .verify_staged_manifest(plan_id, &item.skill_id, &item.manifest)
            {
                let _ = self.library.delete_staged(plan_id);
                return Err(error);
            }
            staged.push((item.clone(), model));
        }
        for item in &plan.items {
            let model = inspect_payload(&item.selection.source, self.source.as_ref(), &self.limits)
                .map_err(|_| ImportError::SourceChanged)?;
            if !model.manifest.equivalent(&item.manifest) {
                let _ = self.library.delete_staged(plan_id);
                return Err(ImportError::SourceChanged);
            }
        }
        if let Err(error) = self.library.write_import_journal(&plan) {
            let _ = self.library.delete_staged(plan_id);
            return Err(error);
        }
        let mut moved = Vec::new();
        let mut written = Vec::new();
        let mut observations = Vec::new();
        let mut indexed_new = Vec::new();
        let mut imported = Vec::new();
        for (item, model) in staged {
            let observation = SourceObservation {
                source: item.selection.source.clone(),
                skill_id: item.skill_id.clone(),
                digest: item.manifest.digest.clone(),
                warnings: model.warnings.clone(),
                reader_agent_ids: item.selection.reader_agent_ids.clone(),
            };
            let result = (|| {
                if matches!(item.decision, ImportDecision::NewSkill { .. }) {
                    self.library.move_staged_payload(
                        plan_id,
                        &item.skill_id,
                        &item.selection.slug,
                    )?;
                    moved.push((item.skill_id.clone(), item.selection.slug.clone()));
                    self.library.write_record_and_manifest(
                        &item.skill_id,
                        &item.selection.slug,
                        &model,
                    )?;
                    written.push(item.skill_id.clone());
                    self.observations.index_metadata(
                        &item.skill_id,
                        &IndexedSkillMetadata {
                            description: model.description.clone(),
                            validation: model.validation.clone(),
                            updated_at: self.clock.now_seconds(),
                        },
                    )?;
                    indexed_new.push(item.skill_id.clone());
                }
                self.observations.append(&observation)?;
                observations.push(observation.clone());
                imported.push(ImportedSkill {
                    skill_id: item.skill_id.clone(),
                    slug: item.selection.slug.clone(),
                    source: item.selection.source.clone(),
                    decision: item.decision.clone(),
                    file_count: model
                        .entries
                        .iter()
                        .filter(|entry| entry.kind == crate::library::ManifestKind::File)
                        .count() as u32,
                    total_bytes: model.entries.iter().map(|e| e.bytes).sum(),
                });
                Ok::<(), ImportError>(())
            })();
            if let Err(error) = result {
                let rollback = self
                    .library
                    .rollback_import(plan_id, &moved)
                    .and_then(|_| {
                        written
                            .iter()
                            .try_for_each(|skill| self.library.remove_record_and_manifest(skill))
                    })
                    .and_then(|_| {
                        indexed_new
                            .iter()
                            .try_for_each(|skill| self.observations.remove_index_metadata(skill))
                    })
                    .and_then(|_| {
                        observations.iter().try_for_each(|observation| {
                            self.observations.rollback_observation(observation)
                        })
                    });
                if rollback.is_err() {
                    return Err(ImportError::RecoveryRequired);
                }
                let _ = self.library.remove_import_journal(plan_id);
                let _ = self.library.delete_staged(plan_id);
                return Err(error);
            }
        }
        let result = ImportResult {
            plan_id: plan.id.clone(),
            imported,
            library_revision: self.library.current_revision()?,
        };
        self.plans
            .consume_and_store(&plan.id, &plan.request_hash, &result)?;
        self.library.remove_import_journal(plan_id)?;
        self.library.delete_staged(plan_id)?;
        Ok(result)
    }
}
fn fingerprint_catalog(catalog: &[LibraryRecord]) -> String {
    let mut rows = catalog
        .iter()
        .map(|record| format!("{}|{}|{}", record.skill_id, record.slug, record.digest))
        .collect::<Vec<_>>();
    rows.sort();
    format!("{:x}", md5ish(rows.join("\n").as_bytes()))
}
fn request_hash(snapshot: &ImportSnapshot, selections: &[ImportSelection]) -> String {
    let mut value = format!(
        "{:?}:{:?}:{}",
        snapshot.candidate_ids, snapshot.library_revision, snapshot.allow_invalid_skills
    );
    for item in selections {
        value.push_str(&format!("|{}:{}", item.candidate_id, item.source.display()));
    }
    format!("{:x}", md5ish(value.as_bytes()))
}
fn md5ish(value: &[u8]) -> u64 {
    value.iter().fold(1469598103934665603, |hash, byte| {
        (hash ^ *byte as u64).wrapping_mul(1099511628211)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        discovery::scan::{EntryKind, EntryMetadata, SourceError},
        library::{Manifest, ValidationSummary},
    };
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    const ROOT: &str = "/home/skill";

    #[derive(Default)]
    struct FakeSource {
        bodies: Mutex<HashMap<PathBuf, Vec<u8>>>,
    }

    impl FakeSource {
        fn put(&self, root: &str, body: &[u8]) {
            self.bodies
                .lock()
                .unwrap()
                .insert(PathBuf::from(root), body.to_vec());
        }
    }

    impl PayloadSource for FakeSource {
        fn list_entries(&self, path: &Path) -> Result<Vec<PathBuf>, SourceError> {
            let bodies = self.bodies.lock().unwrap();
            if path == Path::new(ROOT) {
                return Ok(if bodies.contains_key(path) {
                    vec![path.join("SKILL.md")]
                } else {
                    Vec::new()
                });
            }
            if !bodies.contains_key(path) {
                return Err(SourceError::Missing);
            }
            Ok(vec![path.join("SKILL.md")])
        }

        fn entry_metadata(&self, path: &Path) -> Result<EntryMetadata, SourceError> {
            let bodies = self.bodies.lock().unwrap();
            if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md")
                && bodies.contains_key(path.parent().unwrap_or(Path::new("")))
            {
                return Ok(EntryMetadata {
                    kind: EntryKind::File,
                    executable: false,
                    size: bodies[path.parent().unwrap()].len() as u64,
                });
            }
            Err(SourceError::Missing)
        }

        fn read_file(&self, path: &Path, _cap: usize) -> Result<Vec<u8>, SourceError> {
            let root = path.parent().ok_or(SourceError::Missing)?;
            self.bodies
                .lock()
                .unwrap()
                .get(root)
                .cloned()
                .ok_or(SourceError::Missing)
        }

        fn resolve_symlink(&self, _path: &Path, _max_hops: u8) -> Result<PathBuf, SourceError> {
            Err(SourceError::Missing)
        }
    }

    #[derive(Default)]
    struct FakePlans {
        plans: Mutex<HashMap<String, ImportPlan>>,
        results: Mutex<HashMap<String, (String, ImportResult)>>,
    }

    impl PlanStore for FakePlans {
        fn create(&self, plan: &ImportPlan) -> Result<(), ImportError> {
            self.plans
                .lock()
                .unwrap()
                .insert(plan.id.clone(), plan.clone());
            Ok(())
        }
        fn load(&self, id: &str) -> Result<Option<ImportPlan>, ImportError> {
            Ok(self.plans.lock().unwrap().get(id).cloned())
        }
        fn consume(&self, id: &str, result: &ImportResult) -> Result<(), ImportError> {
            self.plans.lock().unwrap().remove(id);
            self.results
                .lock()
                .unwrap()
                .insert(id.to_owned(), ("request".into(), result.clone()));
            Ok(())
        }
        fn expire(&self, now: u64) -> Result<(), ImportError> {
            self.plans
                .lock()
                .unwrap()
                .retain(|_, plan| plan.expires_at >= now);
            Ok(())
        }
        fn idempotency_lookup(
            &self,
            id: &str,
        ) -> Result<Option<(String, ImportResult)>, ImportError> {
            Ok(self.results.lock().unwrap().get(id).cloned())
        }
        fn idempotency_store(
            &self,
            id: &str,
            hash: &str,
            result: &ImportResult,
        ) -> Result<(), ImportError> {
            self.results
                .lock()
                .unwrap()
                .insert(id.into(), (hash.into(), result.clone()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeLibrary {
        catalog: Mutex<Vec<LibraryRecord>>,
        fail_stage: Mutex<bool>,
        moved: Mutex<Vec<String>>,
        events: Mutex<Vec<String>>,
    }

    impl LibraryRepository for FakeLibrary {
        fn catalog(&self) -> Result<Vec<LibraryRecord>, ImportError> {
            Ok(self.catalog.lock().unwrap().clone())
        }
        fn stage_payload(
            &self,
            _plan: &str,
            skill: &str,
            _model: &PayloadModel,
        ) -> Result<(), ImportError> {
            self.events.lock().unwrap().push(format!("stage:{skill}"));
            if *self.fail_stage.lock().unwrap() {
                return Err(ImportError::Library("stage failed".into()));
            }
            Ok(())
        }
        fn verify_staged_manifest(
            &self,
            _plan: &str,
            _skill: &str,
            _manifest: &Manifest,
        ) -> Result<(), ImportError> {
            Ok(())
        }
        fn move_staged_payload(
            &self,
            _plan: &str,
            skill: &str,
            _slug: &str,
        ) -> Result<(), ImportError> {
            self.events.lock().unwrap().push(format!("move:{skill}"));
            self.moved.lock().unwrap().push(skill.into());
            Ok(())
        }
        fn write_record_and_manifest(
            &self,
            _skill: &str,
            _slug: &str,
            _model: &PayloadModel,
        ) -> Result<(), ImportError> {
            Ok(())
        }
        fn delete_staged(&self, _plan: &str) -> Result<(), ImportError> {
            Ok(())
        }
        fn current_revision(&self) -> Result<Option<String>, ImportError> {
            Ok(None)
        }
        fn has_uncommitted_changes(&self) -> Result<bool, ImportError> {
            Ok(false)
        }
        fn write_import_journal(&self, _plan: &ImportPlan) -> Result<(), ImportError> {
            self.events.lock().unwrap().push("journal-write".into());
            Ok(())
        }
        fn remove_import_journal(&self, _plan: &str) -> Result<(), ImportError> {
            self.events.lock().unwrap().push("journal-remove".into());
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeObservations {
        values: Mutex<Vec<SourceObservation>>,
    }

    impl ObservationStore for FakeObservations {
        fn append(&self, observation: &SourceObservation) -> Result<(), ImportError> {
            self.values.lock().unwrap().push(observation.clone());
            Ok(())
        }
        fn list(&self, skill_id: &str) -> Result<Vec<SourceObservation>, ImportError> {
            Ok(self
                .values
                .lock()
                .unwrap()
                .iter()
                .filter(|observation| observation.skill_id == skill_id)
                .cloned()
                .collect())
        }
    }

    struct FixedClock(Mutex<u64>);
    impl Clock for FixedClock {
        fn now_seconds(&self) -> u64 {
            *self.0.lock().unwrap()
        }
    }

    struct FixedIds(Mutex<u32>);
    impl IdSource for FixedIds {
        fn next_id(&self) -> String {
            let mut id = self.0.lock().unwrap();
            *id += 1;
            format!("id-{id}")
        }
    }

    fn source() -> Arc<FakeSource> {
        let source = Arc::new(FakeSource::default());
        source.put(
            ROOT,
            b"---\nname: skill\ndescription: description\n---\nbody\n",
        );
        source
    }

    fn selection(source: &str, slug: &str) -> ImportSelection {
        ImportSelection {
            candidate_id: slug.into(),
            source: source.into(),
            slug: slug.into(),
            validation: ValidationSummary::valid(),
            reader_agent_ids: vec!["agent".into()],
        }
    }

    fn service(
        source: Arc<FakeSource>,
        plans: Arc<FakePlans>,
        library: Arc<FakeLibrary>,
        observations: Arc<FakeObservations>,
        now: u64,
    ) -> ImportService {
        ImportService {
            source,
            plans,
            library,
            observations,
            clock: Arc::new(FixedClock(Mutex::new(now))),
            ids: Arc::new(FixedIds(Mutex::new(0))),
            limits: ValidationLimits::default(),
        }
    }

    fn snapshot() -> ImportSnapshot {
        ImportSnapshot {
            candidate_ids: vec!["skill".into()],
            library_revision: None,
            allow_invalid_skills: false,
        }
    }

    #[test]
    fn identical_payload_attaches_observation() {
        let source = source();
        let model = inspect_payload(
            Path::new(ROOT),
            source.as_ref(),
            &ValidationLimits::default(),
        )
        .unwrap();
        let library = Arc::new(FakeLibrary::default());
        library.catalog.lock().unwrap().push(LibraryRecord {
            skill_id: "existing".into(),
            slug: "skill".into(),
            digest: model.manifest.digest.clone(),
            manifest: model.manifest,
        });
        let observations = Arc::new(FakeObservations::default());
        let service = service(
            source,
            Arc::new(FakePlans::default()),
            library,
            observations.clone(),
            1,
        );
        let plan = service
            .prepare(vec![selection(ROOT, "skill")], false, snapshot())
            .unwrap();
        assert!(matches!(
            plan.items[0].decision,
            ImportDecision::AttachObservation { .. }
        ));
        service.apply(&plan.id).unwrap();
        assert_eq!(observations.values.lock().unwrap().len(), 1);
    }

    #[test]
    fn same_slug_with_different_content_creates_separate_skill() {
        let source = source();
        let library = Arc::new(FakeLibrary::default());
        let service = service(
            source.clone(),
            Arc::new(FakePlans::default()),
            library,
            Arc::new(FakeObservations::default()),
            1,
        );
        let plan = service
            .prepare(vec![selection(ROOT, "skill")], false, snapshot())
            .unwrap();
        assert!(matches!(
            plan.items[0].decision,
            ImportDecision::NewSkill { .. }
        ));
    }

    #[test]
    fn duplicate_existing_payloads_refuse_prepare() {
        let source = source();
        let model = inspect_payload(
            Path::new(ROOT),
            source.as_ref(),
            &ValidationLimits::default(),
        )
        .unwrap();
        let library = Arc::new(FakeLibrary::default());
        for id in ["one", "two"] {
            library.catalog.lock().unwrap().push(LibraryRecord {
                skill_id: id.into(),
                slug: "skill".into(),
                digest: model.manifest.digest.clone(),
                manifest: model.manifest.clone(),
            });
        }
        let service = service(
            source,
            Arc::new(FakePlans::default()),
            library,
            Arc::new(FakeObservations::default()),
            1,
        );
        assert!(matches!(
            service.prepare(vec![selection(ROOT, "skill")], false, snapshot()),
            Err(ImportError::Validation(message)) if message.contains("multiple identical")
        ));
    }

    #[test]
    fn invalid_candidate_requires_explicit_confirmation() {
        let source = Arc::new(FakeSource::default());
        let service = service(
            source,
            Arc::new(FakePlans::default()),
            Arc::new(FakeLibrary::default()),
            Arc::new(FakeObservations::default()),
            1,
        );
        let result = service.prepare(vec![selection(ROOT, "skill")], false, snapshot());
        assert!(
            matches!(result, Err(ImportError::Validation(_))),
            "{result:?}"
        );
        assert!(
            service
                .prepare(
                    vec![selection(ROOT, "skill")],
                    true,
                    ImportSnapshot {
                        allow_invalid_skills: true,
                        ..snapshot()
                    }
                )
                .is_ok()
        );
    }

    #[test]
    fn staging_failure_aborts_before_any_move() {
        let source = source();
        let library = Arc::new(FakeLibrary::default());
        *library.fail_stage.lock().unwrap() = true;
        let service = service(
            source,
            Arc::new(FakePlans::default()),
            library.clone(),
            Arc::new(FakeObservations::default()),
            1,
        );
        let plan = service
            .prepare(vec![selection(ROOT, "skill")], false, snapshot())
            .unwrap();
        assert!(service.apply(&plan.id).is_err());
        assert!(library.moved.lock().unwrap().is_empty());
    }

    #[test]
    fn expired_plan_is_rejected() {
        let source = source();
        let plans = Arc::new(FakePlans::default());
        let service = service(
            source,
            plans.clone(),
            Arc::new(FakeLibrary::default()),
            Arc::new(FakeObservations::default()),
            100,
        );
        let plan = service
            .prepare(vec![selection(ROOT, "skill")], false, snapshot())
            .unwrap();
        plans
            .plans
            .lock()
            .unwrap()
            .get_mut(&plan.id)
            .unwrap()
            .expires_at = 99;
        assert_eq!(service.apply(&plan.id), Err(ImportError::StalePlan));
    }
    #[test]
    fn source_change_between_prepare_and_apply_is_rejected() {
        let source = source();
        let plans = Arc::new(FakePlans::default());
        let library = Arc::new(FakeLibrary::default());
        let service = service(
            source.clone(),
            plans,
            library,
            Arc::new(FakeObservations::default()),
            1,
        );
        let plan = service
            .prepare(vec![selection(ROOT, "skill")], false, snapshot())
            .unwrap();
        source.put(ROOT, b"---\nname: skill\ndescription: changed\n---\nbody\n");
        assert_eq!(service.apply(&plan.id), Err(ImportError::SourceChanged));
    }

    #[test]
    fn consumed_plan_replay_returns_stored_result_and_journal_brackets_move() {
        let source = source();
        let library = Arc::new(FakeLibrary::default());
        let observations = Arc::new(FakeObservations::default());
        let service = service(
            source,
            Arc::new(FakePlans::default()),
            library.clone(),
            observations.clone(),
            1,
        );
        let plan = service
            .prepare(vec![selection(ROOT, "skill")], false, snapshot())
            .unwrap();
        let first = service.apply(&plan.id).unwrap();
        let second = service.apply(&plan.id).unwrap();
        assert_eq!(first, second);
        assert_eq!(observations.values.lock().unwrap().len(), 1);
        assert_eq!(
            library.events.lock().unwrap().as_slice(),
            ["stage:id-2", "journal-write", "move:id-2", "journal-remove"]
        );
    }
}
