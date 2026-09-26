use crate::{
    commands::{discovery::map_validation, imports::map_import_error, recorded},
    transport::*,
};
use skillbinder_app::AppState;
use skillbinder_core::import::{LibraryRepository, ObservationStore};
use tauri::State;

#[tauri::command(async)]
pub fn library_list(state: State<'_, AppState>) -> CommandResult<LibraryListResponse> {
    recorded("library_list", || {
        match list(state.library.as_ref(), state.store.as_ref()) {
            Ok(value) => CommandResult::success(value),
            Err(error) => CommandResult::failure(error),
        }
    })
}

#[tauri::command(async)]
pub fn library_skill_preview(
    state: State<'_, AppState>,
    request: LibrarySkillPreviewRequest,
) -> CommandResult<LibrarySkillPreviewResponse> {
    recorded("library_skill_preview", || {
        match state
            .library
            .skill_preview(&request.skill_id, request.path.as_deref())
        {
            Ok(preview) => CommandResult::success(LibrarySkillPreviewResponse {
                skill_id: preview.skill_id,
                last_edited_at: preview.last_edited_at,
                files: preview.files,
                path: preview.path,
                content: preview.content,
                unavailable_reason: preview.unavailable_reason,
            }),
            Err(error) => CommandResult::failure(map_import_error(error)),
        }
    })
}

#[tauri::command(async)]
pub fn library_resolve_conflict(
    state: State<'_, AppState>,
    request: LibraryResolveConflictRequest,
) -> CommandResult<LibraryResolveConflictResponse> {
    recorded("library_resolve_conflict", || {
        match state.library_maintenance.resolve_conflict(
            skillbinder_core::library_maintenance::ResolveConflictRequest {
                slug: request.slug,
                keep_skill_id: request.keep_skill_id,
                expected_skill_ids: request.expected_skill_ids,
            },
        ) {
            Ok(outcome) => match (
                state.library.current_revision(),
                state.library.has_uncommitted_changes(),
            ) {
                (Ok(library_revision), Ok(has_uncommitted_changes)) => {
                    CommandResult::success(LibraryResolveConflictResponse {
                        slug: outcome.slug,
                        kept_skill_id: outcome.kept_skill_id,
                        removed_skill_ids: outcome.removed_skill_ids,
                        payload_directory: outcome.payload_directory,
                        library_revision,
                        has_uncommitted_changes,
                    })
                }
                (revision, dirty) => {
                    let error = revision
                        .err()
                        .or(dirty.err())
                        .map(|error| error.to_string())
                        .unwrap_or_default();
                    CommandResult::failure(internal("library-resolution", error))
                }
            },
            Err(error) => CommandResult::failure(map_import_error(error)),
        }
    })
}

fn list(
    library: &dyn LibraryRepository,
    observations: &dyn ObservationStore,
) -> Result<LibraryListResponse, AppError> {
    let skills = library
        .catalog()
        .map_err(|error| internal("library-list", error.to_string()))?
        .into_iter()
        .map(|record| {
            let skill_id = record.skill_id.clone();
            let sources = observations
                .list(&skill_id)
                .map_err(|error| database("library-observations", error.to_string()))?
                .into_iter()
                .map(|observation| SkillSource {
                    display_path: observation.source.display().to_string(),
                    reader_agent_ids: observation.reader_agent_ids,
                })
                .collect();
            let metadata = observations
                .indexed_metadata(&skill_id)
                .map_err(|error| database("library-metadata", error.to_string()))?;
            let (description, validation) = match metadata {
                Some(metadata) => (metadata.description, map_validation(metadata.validation)),
                None => {
                    let mut summary = skillbinder_core::library::ValidationSummary::valid();
                    summary.push(
                        skillbinder_core::library::ValidationCode::IndexNotBuilt,
                        skillbinder_core::library::ValidationLevel::Warning,
                        "skill metadata is not indexed",
                    );
                    (None, map_validation(summary))
                }
            };
            Ok(LibrarySkill {
                skill_id: record.skill_id,
                slug: record.slug,
                display_name: None,
                description,
                validation,
                file_count: record.file_count,
                total_bytes: record.total_bytes.to_string(),
                sources,
                digest: record.digest,
                payload_directory: record.payload_directory,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(LibraryListResponse {
        pending_resolution: library
            .pending_resolution()
            .map_err(|error| internal("library-resolution", error.to_string()))?
            .is_some(),
        library_revision: library
            .current_revision()
            .map_err(|error| internal("library-revision", error.to_string()))?,
        has_uncommitted_changes: library
            .has_uncommitted_changes()
            .map_err(|error| internal("library-git", error.to_string()))?,
        skills,
    })
}

fn internal(diagnostic_id: &str, message: String) -> AppError {
    AppError {
        code: ErrorCode::InternalError,
        message,
        retryable: true,
        recovery_action: None,
        diagnostic_id: diagnostic_id.into(),
    }
}

fn database(diagnostic_id: &str, message: String) -> AppError {
    AppError {
        code: ErrorCode::DatabaseUnavailable,
        message,
        retryable: true,
        recovery_action: None,
        diagnostic_id: diagnostic_id.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{ValidationCode as TransportCode, ValidationStatus as TransportStatus};
    use skillbinder_core::{
        import::{ImportError, IndexedSkillMetadata, LibraryRecord, SourceObservation},
        library::{Manifest, ManifestEntry, ManifestKind, ValidationLevel, ValidationSummary},
    };
    use std::path::PathBuf;

    struct FakeLibrary {
        records: Vec<LibraryRecord>,
        revision: Option<String>,
        dirty: bool,
    }

    impl LibraryRepository for FakeLibrary {
        fn catalog(&self) -> Result<Vec<LibraryRecord>, ImportError> {
            Ok(self.records.clone())
        }
        fn stage_payload(
            &self,
            _plan: &str,
            _skill: &str,
            _model: &skillbinder_core::library::PayloadModel,
        ) -> Result<(), ImportError> {
            unreachable!()
        }
        fn verify_staged_manifest(
            &self,
            _plan: &str,
            _skill: &str,
            _manifest: &Manifest,
        ) -> Result<(), ImportError> {
            unreachable!()
        }
        fn move_staged_payload(
            &self,
            _plan: &str,
            _skill: &str,
            _slug: &str,
        ) -> Result<(), ImportError> {
            unreachable!()
        }
        fn write_record_and_manifest(
            &self,
            _skill: &str,
            _slug: &str,
            _model: &skillbinder_core::library::PayloadModel,
        ) -> Result<(), ImportError> {
            unreachable!()
        }
        fn delete_staged(&self, _plan: &str) -> Result<(), ImportError> {
            unreachable!()
        }
        fn current_revision(&self) -> Result<Option<String>, ImportError> {
            Ok(self.revision.clone())
        }
        fn has_uncommitted_changes(&self) -> Result<bool, ImportError> {
            Ok(self.dirty)
        }
        fn apply_resolution(
            &self,
            _resolution: &skillbinder_core::library_maintenance::ConflictResolution,
        ) -> Result<String, ImportError> {
            unreachable!()
        }
    }

    struct FakeObservations {
        observations: Vec<SourceObservation>,
        metadata: Vec<(String, IndexedSkillMetadata)>,
    }

    impl ObservationStore for FakeObservations {
        fn append(&self, _observation: &SourceObservation) -> Result<(), ImportError> {
            unreachable!()
        }
        fn list(&self, skill_id: &str) -> Result<Vec<SourceObservation>, ImportError> {
            Ok(self
                .observations
                .iter()
                .filter(|observation| observation.skill_id == skill_id)
                .cloned()
                .collect())
        }
        fn index_metadata(
            &self,
            _skill_id: &str,
            _metadata: &IndexedSkillMetadata,
        ) -> Result<(), ImportError> {
            unreachable!()
        }
        fn indexed_metadata(
            &self,
            skill_id: &str,
        ) -> Result<Option<IndexedSkillMetadata>, ImportError> {
            Ok(self
                .metadata
                .iter()
                .find(|(id, _)| id == skill_id)
                .map(|(_, metadata)| metadata.clone()))
        }
    }

    fn record(skill_id: &str, slug: &str) -> LibraryRecord {
        let manifest = Manifest::new(vec![
            ManifestEntry {
                kind: ManifestKind::File,
                path: "SKILL.md".into(),
                sha256: "ab".repeat(32),
                bytes: 40,
                executable: false,
            },
            ManifestEntry {
                kind: ManifestKind::Directory,
                path: "assets".into(),
                sha256: "0".repeat(64),
                bytes: 0,
                executable: false,
            },
            ManifestEntry {
                kind: ManifestKind::File,
                path: "assets/notes.md".into(),
                sha256: "cd".repeat(32),
                bytes: 10,
                executable: true,
            },
        ]);
        LibraryRecord {
            skill_id: skill_id.into(),
            slug: slug.into(),
            digest: manifest.digest().to_owned(),
            file_count: manifest.file_count(),
            total_bytes: manifest.total_bytes(),
            payload_directory: slug.into(),
        }
    }

    #[test]
    fn listed_skills_carry_sources_counts_and_indexed_metadata() {
        let indexed = IndexedSkillMetadata {
            description: Some("Review code.".into()),
            validation: ValidationSummary::valid(),
            updated_at: 1_700_000_000,
        };
        let library = FakeLibrary {
            records: vec![record("skill-1", "review"), record("skill-2", "outline")],
            revision: Some("revision-1".into()),
            dirty: true,
        };
        let observations = FakeObservations {
            observations: vec![SourceObservation {
                source: PathBuf::from("/home/dev/.claude/skills/review"),
                skill_id: "skill-1".into(),
                digest: "sha256:x".into(),
                warnings: Vec::new(),
                reader_agent_ids: vec!["claude-code".into(), "zed".into()],
            }],
            metadata: vec![("skill-1".into(), indexed)],
        };

        let response = list(&library, &observations).unwrap();

        assert_eq!(response.library_revision.as_deref(), Some("revision-1"));
        assert!(response.has_uncommitted_changes);
        assert_eq!(response.skills.len(), 2);

        let review = &response.skills[0];
        assert_eq!(review.description.as_deref(), Some("Review code."));
        assert_eq!(review.validation.status, TransportStatus::Valid);
        assert_eq!(review.file_count, 2);
        assert_eq!(review.total_bytes, "50");
        assert_eq!(review.payload_directory, "review");
        assert!(review.digest.starts_with("sha256:"));
        assert!(!response.pending_resolution);
        assert_eq!(review.sources.len(), 1);
        assert_eq!(
            review.sources[0].display_path,
            "/home/dev/.claude/skills/review"
        );
        assert_eq!(
            review.sources[0].reader_agent_ids,
            vec!["claude-code".to_owned(), "zed".to_owned()]
        );

        let outline = &response.skills[1];
        assert_eq!(outline.description, None);
        assert_eq!(outline.sources.len(), 0);
        assert_eq!(outline.validation.status, TransportStatus::Warning);
        assert_eq!(outline.validation.messages.len(), 1);
        assert_eq!(
            outline.validation.messages[0].code,
            TransportCode::IndexNotBuilt
        );
    }

    #[test]
    fn warning_summary_keeps_the_highest_level() {
        let mut summary = ValidationSummary::valid();
        summary.push(
            skillbinder_core::library::ValidationCode::IndexNotBuilt,
            ValidationLevel::Warning,
            "not indexed",
        );
        assert_eq!(
            summary.status,
            skillbinder_core::library::ValidationStatus::Warning
        );
    }
}
