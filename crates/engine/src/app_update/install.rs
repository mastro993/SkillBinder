//! Puts a bundle in place of another by renames on one volume (spec rule 12, ADR 0018), so an app
//! update installs at quit quickly and can be undone.

use std::{fs, io, path::Path};

/// Puts `staged` at `target`. A bundle already at `target` is renamed to `old` first, and put back
/// when `staged` can't take its place. The caller removes `old`.
pub(super) fn swap(staged: &Path, target: &Path, old: &Path) -> io::Result<()> {
    if fs::symlink_metadata(target).is_err() {
        return fs::rename(staged, target);
    }
    let _ = fs::remove_dir_all(old);
    fs::rename(target, old)?;
    if let Err(error) = fs::rename(staged, target) {
        let _ = fs::rename(old, target);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "Test fixture failures must abort the test."
    )]
    use super::*;

    fn bundle(path: &Path, marker: &str) {
        fs::create_dir_all(path.join("Contents")).unwrap();
        fs::write(path.join("Contents/marker"), marker).unwrap();
    }

    fn marker(path: &Path) -> String {
        fs::read_to_string(path.join("Contents/marker")).unwrap()
    }

    #[test]
    fn the_staged_bundle_takes_the_installed_ones_place() {
        let temp = tempfile::tempdir().unwrap();
        let (staged, target, old) = (
            temp.path().join(".skillbinder-update-0.2.0.app"),
            temp.path().join("SkillBinder.app"),
            temp.path().join(".skillbinder-old-0.1.0.app"),
        );
        bundle(&staged, "new");
        bundle(&target, "running");
        swap(&staged, &target, &old).unwrap();
        assert_eq!(marker(&target), "new");
        assert_eq!(marker(&old), "running");
        assert!(!staged.exists());
    }

    #[test]
    fn a_failed_second_rename_restores_the_installed_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let (target, old) = (
            temp.path().join("SkillBinder.app"),
            temp.path().join(".skillbinder-old-0.1.0.app"),
        );
        bundle(&target, "running");
        let missing = temp.path().join(".skillbinder-update-0.2.0.app");
        assert!(swap(&missing, &target, &old).is_err());
        assert_eq!(marker(&target), "running");
        assert!(!old.exists());
    }
}
