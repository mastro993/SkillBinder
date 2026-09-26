use crate::payload_filesystem::FilesystemPayloadSource;
use skillbinder_core::library::{Manifest, ManifestKind, ValidationLimits, inspect_payload};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum BindingFileError {
    #[error("target is occupied by an unmanaged or changed copy")]
    Conflict,
    #[error("source library skill changed")]
    SourceChanged,
    #[error("unsafe target path")]
    UnsafePath,
    #[error("filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
}

pub fn matches_manifest(path: &Path, manifest: &Manifest) -> bool {
    if fs::symlink_metadata(path).is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound) {
        return false;
    }
    if !fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir() && !meta.file_type().is_symlink())
    {
        return false;
    }
    if !manifest.entries.iter().all(|entry| {
        fs::symlink_metadata(path.join(&entry.path)).is_ok_and(|metadata| match entry.kind {
            ManifestKind::File => metadata.file_type().is_file(),
            ManifestKind::Directory => metadata.file_type().is_dir(),
        })
    }) {
        return false;
    }
    inspect_payload(path, &FilesystemPayloadSource, &ValidationLimits::default())
        .is_ok_and(|model| model.manifest.equivalent(manifest))
}

pub fn deploy_copy(
    source: &Path,
    target: &Path,
    manifest: &Manifest,
) -> Result<(), BindingFileError> {
    if target
        .components()
        .any(|part| matches!(part, Component::ParentDir))
        || !target.is_absolute()
    {
        return Err(BindingFileError::UnsafePath);
    }
    if fs::symlink_metadata(target).is_ok() {
        return Err(BindingFileError::Conflict);
    }
    let model = inspect_payload(
        source,
        &FilesystemPayloadSource,
        &ValidationLimits::default(),
    )
    .map_err(|_| BindingFileError::SourceChanged)?;
    if !model.manifest.equivalent(manifest) {
        return Err(BindingFileError::SourceChanged);
    }
    let parent = target.parent().ok_or(BindingFileError::UnsafePath)?;
    ensure_directories_without_symlinks(parent)?;
    let staged = parent.join(format!(".skillbinder-{}", Uuid::new_v4()));
    fs::create_dir(&staged)?;
    let result = (|| {
        for entry in &manifest.entries {
            let relative = Path::new(&entry.path);
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(BindingFileError::UnsafePath);
            }
            let output = staged.join(relative);
            match entry.kind {
                ManifestKind::Directory => fs::create_dir_all(&output)?,
                ManifestKind::File => {
                    if let Some(parent) = output.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    let input = source.join(relative);
                    let metadata = fs::symlink_metadata(&input)?;
                    if !metadata.is_file() || metadata.file_type().is_symlink() {
                        return Err(BindingFileError::SourceChanged);
                    }
                    fs::copy(input, &output)?;
                    #[cfg(unix)]
                    if entry.executable {
                        use std::os::unix::fs::PermissionsExt;
                        let mut perms = fs::metadata(&output)?.permissions();
                        perms.set_mode(perms.mode() | 0o111);
                        fs::set_permissions(&output, perms)?;
                    }
                }
            }
        }
        if !matches_manifest(&staged, manifest) {
            return Err(BindingFileError::SourceChanged);
        }
        fs::create_dir(target).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                BindingFileError::Conflict
            } else {
                error.into()
            }
        })?;
        for entry in fs::read_dir(&staged)? {
            let entry = entry?;
            fs::rename(entry.path(), target.join(entry.file_name()))?;
        }
        if !matches_manifest(target, manifest) {
            return Err(BindingFileError::SourceChanged);
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(&staged);
    result
}

fn ensure_directories_without_symlinks(path: &Path) -> Result<(), BindingFileError> {
    let mut current = PathBuf::new();
    for part in path.components() {
        if matches!(part, Component::ParentDir) {
            return Err(BindingFileError::UnsafePath);
        }
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => return Err(BindingFileError::UnsafePath),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::create_dir(&current)?,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_can_be_repaired_after_deletion_without_overwriting_changed_content() {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("skillbinder-bindings-{}", Uuid::new_v4()));
        let source = root.join("library/review");
        let target = root.join("project/.agents/skills/review");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            "---\nname: review\ndescription: Reviews code.\n---\nReview changes.\n",
        )
        .unwrap();
        let manifest = inspect_payload(
            &source,
            &FilesystemPayloadSource,
            &ValidationLimits::default(),
        )
        .unwrap()
        .manifest;
        deploy_copy(&source, &target, &manifest).unwrap();
        assert!(matches_manifest(&target, &manifest));
        fs::remove_dir_all(&target).unwrap();
        assert!(!matches_manifest(&target, &manifest));
        deploy_copy(&source, &target, &manifest).unwrap();
        fs::write(target.join("SKILL.md"), "changed").unwrap();
        assert!(matches!(
            deploy_copy(&source, &target, &manifest),
            Err(BindingFileError::Conflict)
        ));
        assert_eq!(
            fs::read_to_string(target.join("SKILL.md")).unwrap(),
            "changed"
        );
        #[cfg(unix)]
        {
            fs::remove_file(target.join("SKILL.md")).unwrap();
            std::os::unix::fs::symlink(source.join("SKILL.md"), target.join("SKILL.md")).unwrap();
            assert!(!matches_manifest(&target, &manifest));
        }
        fs::remove_dir_all(root).unwrap();
    }
}
