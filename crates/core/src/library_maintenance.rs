//! Resolving a slug that names more than one library skill.
//!
//! Payloads live at `skills/<slug>/`, and two skills that share a slug cannot both own that name.
//! The user picks the copy to keep, and the others move out of the library into the app's backup
//! directory. The operation is journaled so an interrupted run converges on the same end state.

use crate::import::{IdSource, ImportError, LibraryRepository, ObservationStore};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

/// A slug that names more than one library skill, with its skill ids in ascending order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlugConflict {
    pub slug: String,
    pub skill_ids: Vec<String>,
}

/// The one definition of "this slug names two skills". Pure, so the library write path and any
/// read surface agree on membership.
pub fn slug_conflicts<'a>(
    skills: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<SlugConflict> {
    let mut groups: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (id, slug) in skills {
        groups.entry(slug).or_default().push(id);
    }
    groups
        .into_iter()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(slug, mut ids)| {
            ids.sort_unstable();
            SlugConflict {
                slug: slug.to_owned(),
                skill_ids: ids.into_iter().map(str::to_owned).collect(),
            }
        })
        .collect()
}

/// What the user decided, persisted verbatim in the resolution journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictResolution {
    pub operation_id: String,
    pub slug: String,
    pub keep_skill_id: String,
    pub drop_skill_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveConflictRequest {
    pub slug: String,
    pub keep_skill_id: String,
    /// Exactly the member set the user saw, sorted. A mismatch means the library moved.
    pub expected_skill_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictResolutionOutcome {
    pub slug: String,
    pub kept_skill_id: String,
    pub removed_skill_ids: Vec<String>,
    /// The kept skill's directory under `skills/` after resolution, which is always the bare slug.
    pub payload_directory: String,
}

pub struct LibraryMaintenance {
    pub library: Arc<dyn LibraryRepository>,
    pub observations: Arc<dyn ObservationStore>,
    pub ids: Arc<dyn IdSource>,
}

impl LibraryMaintenance {
    pub fn resolve_conflict(
        &self,
        request: ResolveConflictRequest,
    ) -> Result<ConflictResolutionOutcome, ImportError> {
        if let Some(pending) = self.library.pending_resolution()? {
            return self.finish(&pending);
        }
        let catalog = self.library.catalog()?;
        let group = slug_conflicts(
            catalog
                .iter()
                .map(|record| (record.skill_id.as_str(), record.slug.as_str())),
        )
        .into_iter()
        .find(|group| group.slug == request.slug)
        .ok_or_else(|| {
            ImportError::Validation(format!(
                "{} no longer names more than one skill",
                request.slug
            ))
        })?;
        let mut expected = request.expected_skill_ids.clone();
        expected.sort();
        if group.skill_ids != expected {
            return Err(ImportError::StalePlan);
        }
        if !group
            .skill_ids
            .iter()
            .any(|id| id == &request.keep_skill_id)
        {
            return Err(ImportError::Validation(format!(
                "{} is not one of the skills named {}",
                request.keep_skill_id, request.slug
            )));
        }
        let resolution = ConflictResolution {
            operation_id: self.ids.next_id(),
            slug: group.slug,
            keep_skill_id: request.keep_skill_id.clone(),
            drop_skill_ids: group
                .skill_ids
                .into_iter()
                .filter(|id| id != &request.keep_skill_id)
                .collect(),
        };
        self.library.write_resolution_journal(&resolution)?;
        self.finish(&resolution)
    }

    /// Converges on the same end state when re-entered after a crash at any point.
    fn finish(
        &self,
        resolution: &ConflictResolution,
    ) -> Result<ConflictResolutionOutcome, ImportError> {
        let payload_directory = self.library.apply_resolution(resolution)?;
        for skill in &resolution.drop_skill_ids {
            self.observations.remove_observations(skill)?;
            self.observations.remove_index_metadata(skill)?;
        }
        self.library
            .remove_resolution_journal(&resolution.operation_id)?;
        Ok(ConflictResolutionOutcome {
            slug: resolution.slug.clone(),
            kept_skill_id: resolution.keep_skill_id.clone(),
            removed_skill_ids: resolution.drop_skill_ids.clone(),
            payload_directory,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::{IndexedSkillMetadata, LibraryRecord, ObservationStore, SourceObservation};
    use std::sync::Mutex;

    fn record(id: &str, slug: &str) -> LibraryRecord {
        LibraryRecord {
            skill_id: id.to_owned(),
            slug: slug.to_owned(),
            digest: format!("sha256:{id}"),
            file_count: 1,
            total_bytes: 1,
            payload_directory: slug.to_owned(),
        }
    }

    #[derive(Default)]
    struct FakeLibrary {
        records: Mutex<Vec<LibraryRecord>>,
        pending: Mutex<Option<ConflictResolution>>,
        applied: Mutex<Vec<ConflictResolution>>,
        journal_writes: Mutex<usize>,
    }

    impl LibraryRepository for FakeLibrary {
        fn catalog(&self) -> Result<Vec<LibraryRecord>, ImportError> {
            Ok(self.records.lock().unwrap().clone())
        }
        fn stage_payload(
            &self,
            _plan: &str,
            _skill: &str,
            _model: &crate::library::PayloadModel,
        ) -> Result<(), ImportError> {
            Ok(())
        }
        fn verify_staged_manifest(
            &self,
            _plan: &str,
            _skill: &str,
            _manifest: &crate::library::Manifest,
        ) -> Result<(), ImportError> {
            Ok(())
        }
        fn move_staged_payload(
            &self,
            _plan: &str,
            _skill: &str,
            _slug: &str,
        ) -> Result<(), ImportError> {
            Ok(())
        }
        fn write_record_and_manifest(
            &self,
            _skill: &str,
            _slug: &str,
            _model: &crate::library::PayloadModel,
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
        fn pending_resolution(&self) -> Result<Option<ConflictResolution>, ImportError> {
            Ok(self.pending.lock().unwrap().clone())
        }
        fn write_resolution_journal(
            &self,
            resolution: &ConflictResolution,
        ) -> Result<(), ImportError> {
            *self.journal_writes.lock().unwrap() += 1;
            *self.pending.lock().unwrap() = Some(resolution.clone());
            Ok(())
        }
        fn remove_resolution_journal(&self, _operation_id: &str) -> Result<(), ImportError> {
            *self.pending.lock().unwrap() = None;
            Ok(())
        }
        fn apply_resolution(&self, resolution: &ConflictResolution) -> Result<String, ImportError> {
            self.applied.lock().unwrap().push(resolution.clone());
            self.records
                .lock()
                .unwrap()
                .retain(|record| !resolution.drop_skill_ids.contains(&record.skill_id));
            Ok(resolution.slug.clone())
        }
    }

    #[derive(Default)]
    struct FakeObservations {
        removed: Mutex<Vec<String>>,
    }

    impl ObservationStore for FakeObservations {
        fn append(&self, _observation: &SourceObservation) -> Result<(), ImportError> {
            Ok(())
        }
        fn list(&self, _skill_id: &str) -> Result<Vec<SourceObservation>, ImportError> {
            Ok(Vec::new())
        }
        fn remove_index_metadata(&self, _skill_id: &str) -> Result<(), ImportError> {
            Ok(())
        }
        fn remove_observations(&self, skill_id: &str) -> Result<(), ImportError> {
            self.removed.lock().unwrap().push(skill_id.to_owned());
            Ok(())
        }
    }

    struct Ids;

    impl IdSource for Ids {
        fn next_id(&self) -> String {
            "operation".into()
        }
    }

    fn maintenance(
        library: Arc<FakeLibrary>,
        observations: Arc<FakeObservations>,
    ) -> LibraryMaintenance {
        LibraryMaintenance {
            library,
            observations,
            ids: Arc::new(Ids),
        }
    }

    fn conflicting_library() -> Arc<FakeLibrary> {
        Arc::new(FakeLibrary {
            records: Mutex::new(vec![
                record("aaaa1111", "caveman"),
                record("bbbb2222", "caveman"),
                record("cccc3333", "outline"),
            ]),
            ..Default::default()
        })
    }

    #[test]
    fn slug_conflicts_groups_sorted_members_only() {
        let conflicts = slug_conflicts([
            ("bbbb", "caveman"),
            ("aaaa", "caveman"),
            ("cccc", "outline"),
        ]);
        assert_eq!(
            conflicts,
            vec![SlugConflict {
                slug: "caveman".into(),
                skill_ids: vec!["aaaa".into(), "bbbb".into()],
            }]
        );
    }

    #[test]
    fn keeping_one_copy_drops_the_others_with_their_rows() {
        let library = conflicting_library();
        let observations = Arc::new(FakeObservations::default());
        let outcome = maintenance(library.clone(), observations.clone())
            .resolve_conflict(ResolveConflictRequest {
                slug: "caveman".into(),
                keep_skill_id: "bbbb2222".into(),
                expected_skill_ids: vec!["aaaa1111".into(), "bbbb2222".into()],
            })
            .expect("resolution");
        assert_eq!(outcome.kept_skill_id, "bbbb2222");
        assert_eq!(outcome.removed_skill_ids, vec!["aaaa1111".to_owned()]);
        assert_eq!(outcome.payload_directory, "caveman");
        let remaining: Vec<String> = library
            .records
            .lock()
            .unwrap()
            .iter()
            .map(|record| record.skill_id.clone())
            .collect();
        assert_eq!(
            remaining,
            vec!["bbbb2222".to_owned(), "cccc3333".to_owned()]
        );
        assert_eq!(*observations.removed.lock().unwrap(), vec!["aaaa1111"]);
        assert!(library.pending.lock().unwrap().is_none());
    }

    #[test]
    fn a_moved_library_membership_is_stale() {
        let error = maintenance(conflicting_library(), Arc::new(FakeObservations::default()))
            .resolve_conflict(ResolveConflictRequest {
                slug: "caveman".into(),
                keep_skill_id: "aaaa1111".into(),
                expected_skill_ids: vec!["aaaa1111".into(), "cccc3333".into()],
            })
            .expect_err("stale membership");
        assert_eq!(error, ImportError::StalePlan);
    }

    #[test]
    fn a_chosen_skill_outside_the_slug_is_refused() {
        let error = maintenance(conflicting_library(), Arc::new(FakeObservations::default()))
            .resolve_conflict(ResolveConflictRequest {
                slug: "caveman".into(),
                keep_skill_id: "cccc3333".into(),
                expected_skill_ids: vec!["aaaa1111".into(), "bbbb2222".into()],
            })
            .expect_err("keep id outside the group");
        assert!(matches!(error, ImportError::Validation(_)));
    }

    #[test]
    fn a_settled_slug_is_refused() {
        let library = Arc::new(FakeLibrary {
            records: Mutex::new(vec![record("aaaa1111", "caveman")]),
            ..Default::default()
        });
        let error = maintenance(library, Arc::new(FakeObservations::default()))
            .resolve_conflict(ResolveConflictRequest {
                slug: "caveman".into(),
                keep_skill_id: "aaaa1111".into(),
                expected_skill_ids: vec!["aaaa1111".into()],
            })
            .expect_err("no conflict left");
        assert!(matches!(error, ImportError::Validation(_)));
    }

    #[test]
    fn an_interrupted_resolution_is_finished_instead_of_starting_a_second_one() {
        let library = conflicting_library();
        let pending = ConflictResolution {
            operation_id: "operation".into(),
            slug: "caveman".into(),
            keep_skill_id: "aaaa1111".into(),
            drop_skill_ids: vec!["bbbb2222".into()],
        };
        *library.pending.lock().unwrap() = Some(pending.clone());
        let outcome = maintenance(library.clone(), Arc::new(FakeObservations::default()))
            .resolve_conflict(ResolveConflictRequest {
                slug: "outline".into(),
                keep_skill_id: "cccc3333".into(),
                expected_skill_ids: Vec::new(),
            })
            .expect("resume");
        assert_eq!(outcome.kept_skill_id, "aaaa1111");
        assert_eq!(*library.journal_writes.lock().unwrap(), 0);
        assert_eq!(*library.applied.lock().unwrap(), vec![pending]);
    }

    #[test]
    fn indexed_metadata_is_removed_for_dropped_skills() {
        let library = conflicting_library();
        let observations = Arc::new(FakeObservations::default());
        maintenance(library, observations.clone())
            .resolve_conflict(ResolveConflictRequest {
                slug: "caveman".into(),
                keep_skill_id: "aaaa1111".into(),
                expected_skill_ids: vec!["aaaa1111".into(), "bbbb2222".into()],
            })
            .expect("resolution");
        assert_eq!(*observations.removed.lock().unwrap(), vec!["bbbb2222"]);
        let _: Option<IndexedSkillMetadata> = None;
    }
}
