//! Writing, replacing, and removing one skill's copy in a global skill directory. Each copy is
//! built in a hidden staging directory beside it and renamed into place, so an agent never reads
//! a half-written skill, and a removed copy leaves at once. Nothing else in the directory is
//! touched.

use super::plan::Disk;
use crate::{payload, persistence::sync_directory};
use skillbinder_proto::{AppError, AppResult, ErrorCategory, ValidationStatus};
use std::{fs, io, path::Path};
use tempfile::TempDir;

/// The name prefix of the staging directories copies are built in, which scans skip.
pub(crate) const STAGING_PREFIX: &str = ".skillbinder-";

/// The content a copy may hold to be moved aside without asking; empty accepts any content, for
/// a replacement the user confirmed.
pub(super) type Expected<'a> = &'a [&'a str];

/// What happened to a copy SkillBinder moved aside.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum Moved {
    /// The copy was replaced or removed.
    Done,
    /// The copy turned out to be edited, so it was put back untouched.
    Edited,
}

/// What `target` holds, read without following a link at its root. A link, a file, unreadable or
/// unsafe content, and Git metadata anywhere inside are never taken for a copy.
pub(super) fn probe(target: &Path) -> Disk {
    match fs::symlink_metadata(target) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Disk::Absent,
        Ok(metadata) if metadata.is_dir() => match payload::inspect(target) {
            Ok(found)
                if found.status != ValidationStatus::Blocked
                    && !found
                        .messages
                        .iter()
                        .any(|finding| finding.message == payload::GIT_EXCLUDED) =>
            {
                Disk::Copy(found.manifest.digest)
            }
            _ => Disk::Other,
        },
        _ => Disk::Other,
    }
}

/// Writes a copy of `source`, a fresh inspection of the library skill, as `directory/slug`, which
/// must be absent. A missing directory is created.
pub(super) fn write(directory: &Path, slug: &str, source: &payload::Inspection) -> AppResult<()> {
    fs::create_dir_all(directory).map_err(unwritable)?;
    let stage = staged(directory, slug, source)?;
    let target = directory.join(slug);
    if fs::symlink_metadata(&target).is_ok() {
        return Err(AppError::stale("Another copy appeared meanwhile."));
    }
    fs::rename(stage.path().join(slug), &target).map_err(unwritable)?;
    sync_directory(directory).map_err(unwritable)
}

/// Replaces `directory/slug` with a copy of `source`, unless what it holds, read once it is moved
/// aside, is not `expected`; then it is put back untouched.
pub(super) fn replace(
    directory: &Path,
    slug: &str,
    source: &payload::Inspection,
    expected: Expected<'_>,
) -> AppResult<Moved> {
    let stage = staged(directory, slug, source)?;
    let (target, previous) = (directory.join(slug), stage.path().join(".previous"));
    fs::rename(&target, &previous).map_err(unwritable)?;
    if !holds(&previous, expected) {
        return put_back(&previous, &target, stage).map(|()| Moved::Edited);
    }
    if let Err(error) = fs::rename(stage.path().join(slug), &target) {
        put_back(&previous, &target, stage)?;
        return Err(unwritable(error));
    }
    sync_directory(directory).map_err(unwritable)?;
    Ok(Moved::Done)
}

/// Removes `directory/slug`, unless what it holds, read once it is moved aside, is not
/// `expected`; then it is put back untouched. A removed copy leaves the directory at once and is
/// deleted with its staging directory.
pub(super) fn remove(directory: &Path, slug: &str, expected: Expected<'_>) -> AppResult<Moved> {
    let stage = stage(directory)?;
    let (target, removed) = (directory.join(slug), stage.path().join(slug));
    fs::rename(&target, &removed).map_err(unwritable)?;
    if !holds(&removed, expected) {
        return put_back(&removed, &target, stage).map(|()| Moved::Edited);
    }
    sync_directory(directory).map_err(unwritable)?;
    Ok(Moved::Done)
}

/// Deletes the staging directories an interrupted change left in `directory`. No change runs
/// meanwhile: the engine worker makes every change, one at a time.
pub(super) fn sweep(directory: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let leftover = name
            .to_str()
            .and_then(|name| name.strip_prefix(STAGING_PREFIX));
        let ours = leftover.is_some_and(|rest| {
            rest.len() == 6 && rest.bytes().all(|byte| byte.is_ascii_alphanumeric())
        });
        if ours && entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// Whether `path` holds content `expected` accepts.
fn holds(path: &Path, expected: Expected<'_>) -> bool {
    expected.is_empty()
        || matches!(probe(path), Disk::Copy(digest) if expected.contains(&digest.as_str()))
}

/// Moves `from` back to `to`. When it cannot, the staging directory is kept rather than deleted,
/// so the content survives, under a name `sweep` never deletes.
fn put_back(from: &Path, to: &Path, stage: TempDir) -> AppResult<()> {
    fs::rename(from, to).map_err(|error| {
        let kept = stage.keep();
        let suffix = kept
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_prefix(STAGING_PREFIX))
            .map(str::to_owned);
        if let (Some(parent), Some(suffix)) = (kept.parent(), suffix) {
            let _ = fs::rename(&kept, parent.join(format!("{STAGING_PREFIX}kept-{suffix}")));
        }
        unwritable(error)
    })
}

/// A staging directory in `directory` holding a verified copy of `source` named `slug`.
fn staged(directory: &Path, slug: &str, source: &payload::Inspection) -> AppResult<TempDir> {
    if source.status == ValidationStatus::Blocked {
        return Err(AppError::validation("This skill cannot safely be copied."));
    }
    let stage = stage(directory)?;
    payload::materialize(source, &stage.path().join(slug)).map_err(|error| {
        match error.category {
            ErrorCategory::Stale => AppError::stale("The skill changed while it was copied."),
            _ => unwritable(error),
        }
    })?;
    Ok(stage)
}

/// A new staging directory in `directory`, deleted with everything in it when dropped.
fn stage(directory: &Path) -> AppResult<TempDir> {
    tempfile::Builder::new()
        .prefix(STAGING_PREFIX)
        // `sweep` recognizes leftovers by this length.
        .rand_bytes(6)
        .tempdir_in(directory)
        .map_err(unwritable)
}

/// The reason a directory shows when SkillBinder cannot change it.
fn unwritable(_: impl std::fmt::Debug) -> AppError {
    AppError::new(
        ErrorCategory::Storage,
        "SkillBinder could not write to this directory.",
        "Check that the directory can be written, then retry.",
    )
}

#[cfg(test)]
mod tests {
    #![expect(clippy::unwrap_used, reason = "Fixture failures must abort the test.")]
    use super::*;

    fn skill(root: &Path, slug: &str, body: &str) -> payload::Inspection {
        let directory = root.join(slug);
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("SKILL.md"),
            format!("---\nname: {slug}\ndescription: {body}\n---\n{body}\n"),
        )
        .unwrap();
        payload::inspect(&directory).unwrap()
    }

    #[test]
    fn copies_are_written_replaced_and_removed_without_leftovers() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let first = skill(&root.join("library"), "review", "First");
        let agents = root.join("agents/skills");
        write(&agents, "review", &first).unwrap();
        assert_eq!(
            probe(&agents.join("review")),
            Disk::Copy(first.manifest.digest.clone())
        );
        let second = skill(&root.join("next"), "review", "Second");
        assert_eq!(
            replace(&agents, "review", &second, &[]).unwrap(),
            Moved::Done
        );
        assert_eq!(
            probe(&agents.join("review")),
            Disk::Copy(second.manifest.digest.clone())
        );
        assert!(write(&agents, "review", &second).is_err());
        let digest = second.manifest.digest.as_str();
        assert_eq!(
            remove(&agents, "review", &["other"]).unwrap(),
            Moved::Edited
        );
        assert!(
            agents.join("review/SKILL.md").exists(),
            "unexpected content stays"
        );
        assert_eq!(remove(&agents, "review", &[digest]).unwrap(), Moved::Done);
        assert_eq!(probe(&agents.join("review")), Disk::Absent);
        assert_eq!(fs::read_dir(&agents).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn links_and_checkouts_are_never_taken_for_copies() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let source = skill(&root.join("library"), "review", "First");
        let agents = root.join("agents");
        fs::create_dir_all(&agents).unwrap();
        std::os::unix::fs::symlink(&source.canonical_source, agents.join("review")).unwrap();
        assert_eq!(probe(&agents.join("review")), Disk::Other);
        let checkout = skill(&agents, "tracked", "Tracked");
        fs::create_dir_all(checkout.canonical_source.join("scripts/.git")).unwrap();
        assert_eq!(probe(&checkout.canonical_source), Disk::Other);
    }

    #[test]
    fn a_sweep_deletes_only_leftover_staging_directories() {
        let temp = tempfile::tempdir().unwrap();
        let agents = fs::canonicalize(temp.path()).unwrap();
        let leftover = stage(&agents).unwrap().keep();
        fs::write(leftover.join("SKILL.md"), "partial").unwrap();
        for kept in [".skillbinder-notes-of-mine", "review"] {
            fs::create_dir(agents.join(kept)).unwrap();
        }
        sweep(&agents);
        let mut names: Vec<_> = fs::read_dir(&agents)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        names.sort();
        assert_eq!(names, [".skillbinder-notes-of-mine", "review"]);
    }
}
