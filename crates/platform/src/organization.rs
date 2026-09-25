use crate::{library_repository::FilesystemLibraryRepository, portable_metadata::PortableMetadata};
use skillbinder_core::{
    import::ImportError,
    library::organization::{Change, Organization, SkillAssignment},
};
use std::{
    collections::BTreeMap,
    sync::{Mutex, MutexGuard},
};

/// One lock for every writer of `.skillbinder.json`. An import that appends a skill and an
/// organization change both read the file, edit it, and write it back, so the two must not
/// interleave.
static ORGANIZATION_LOCK: Mutex<()> = Mutex::new(());

fn map(error: impl ToString) -> ImportError {
    ImportError::Library(error.to_string())
}

pub(crate) fn lock() -> Result<MutexGuard<'static, ()>, ImportError> {
    ORGANIZATION_LOCK.lock().map_err(map)
}

fn organization(metadata: &PortableMetadata) -> Organization {
    Organization {
        folders: metadata.folders.clone(),
        tags: metadata.tags.clone(),
        skills: metadata
            .skills
            .iter()
            .map(|(id, skill)| {
                (
                    id.clone(),
                    SkillAssignment {
                        display_name: skill.display_name.clone(),
                        folder_id: skill.folder_id.clone(),
                        tag_ids: skill.tag_ids.clone(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
    }
}

/// Writes the organization back into the metadata, leaving every other skill field (digest,
/// counts, provenance) as the caller had it.
fn apply(metadata: &mut PortableMetadata, organization: &Organization) {
    metadata.folders = organization.folders.clone();
    metadata.tags = organization.tags.clone();
    for (id, assignment) in &organization.skills {
        if let Some(skill) = metadata.skills.get_mut(id) {
            skill.folder_id = assignment.folder_id.clone();
            skill.tag_ids = assignment.tag_ids.clone();
        }
    }
}

impl FilesystemLibraryRepository {
    pub(crate) fn read_organization(&self) -> Result<Organization, ImportError> {
        Ok(organization(&self.metadata()?))
    }

    pub(crate) fn write_organization(
        &self,
        change: Change,
        expected_revision: Option<&str>,
    ) -> Result<Organization, ImportError> {
        let _guard = lock()?;
        let mut metadata = self.metadata()?;
        let old = organization(&metadata);
        if expected_revision.is_some_and(|expected| expected != revision(&old).unwrap_or_default())
        {
            return Err(map(
                "Organization changed. Review it again before deleting.",
            ));
        }
        let new = old.apply(change).map_err(map)?;
        apply(&mut metadata, &new);
        self.write_metadata(&metadata)?;
        Ok(new)
    }
}

pub fn revision(graph: &Organization) -> Result<String, ImportError> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(&graph.folders).map_err(map)?);
    hasher.update(serde_json::to_vec(&graph.tags).map_err(map)?);
    hasher.update(serde_json::to_vec(&graph.skills).map_err(map)?);
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        paths::AppPaths,
        portable_metadata::{PortableMetadata, PortableSkill},
    };
    use skillbinder_core::import::LibraryRepository;

    fn repository(root: &std::path::Path) -> FilesystemLibraryRepository {
        let paths = AppPaths::new(root.join("data"), root.join("config"), root.join("cache"));
        paths.create_base_directories().expect("base directories");
        std::fs::create_dir_all(paths.library()).expect("library directory");
        FilesystemLibraryRepository::new(paths)
    }

    #[test]
    fn persists_assignments_without_touching_skill_totals() {
        let root =
            std::env::temp_dir().join(format!("skillbinder-organization-{}", uuid::Uuid::new_v4()));
        let repo = repository(&root);
        let mut metadata = PortableMetadata::new("library".into(), "created".into());
        metadata.skills.insert(
            "skill".into(),
            PortableSkill {
                id: "skill".into(),
                slug: "sample".into(),
                display_name: None,
                folder_id: None,
                tag_ids: Vec::new(),
                upstream_bindings: Vec::new(),
                digest: "digest".into(),
                file_count: 2,
                total_bytes: 20,
            },
        );
        repo.write_metadata(&metadata).unwrap();

        repo.change_organization(
            Change::CreateFolder {
                id: "parent".into(),
                name: "Parent".into(),
                parent_id: None,
            },
            None,
        )
        .unwrap();
        repo.change_organization(
            Change::CreateTag {
                id: "tag".into(),
                name: "Useful".into(),
            },
            None,
        )
        .unwrap();
        let assigned = repo
            .change_organization(
                Change::Assign {
                    skill_ids: vec!["skill".into()],
                    folder_id: Some("parent".into()),
                    set_folder: true,
                    add_tag_ids: vec!["tag".into()],
                    remove_tag_ids: vec![],
                },
                None,
            )
            .unwrap();
        assert_eq!(assigned.folders.len(), 1);

        let written = repo.metadata().unwrap();
        assert_eq!(written.folders["parent"].name, "Parent");
        assert_eq!(written.tags["tag"].name, "Useful");
        assert_eq!(written.skills["skill"].folder_id.as_deref(), Some("parent"));
        assert_eq!(written.skills["skill"].tag_ids, vec!["tag".to_owned()]);
        assert_eq!(written.skills["skill"].digest, "digest");

        let stale = revision(&assigned).unwrap();
        repo.change_organization(
            Change::DeleteFolder {
                id: "parent".into(),
            },
            None,
        )
        .unwrap();
        let refused =
            repo.change_organization(Change::DeleteTag { id: "tag".into() }, Some(&stale));
        assert!(refused.is_err());

        let recovered = repo.organization_snapshot().unwrap();
        assert_eq!(recovered.skills["skill"].folder_id, None);
        assert_eq!(recovered.skills["skill"].tag_ids, vec!["tag".to_owned()]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
