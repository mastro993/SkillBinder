//! Which folders a picked folder adds as workspaces: the folder itself and each direct child
//! that has an agent skill directory, or the folder alone when none has one.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

/// The folders `picked` adds, the picked folder first, then its children by name. Linked
/// children are skipped, so every folder stays inside the picked one, and so are hidden ones,
/// such as a project's own `.agents` folder, whose `skills` would otherwise match an agent that
/// reads `skills`.
pub(super) fn projects(picked: &Path, skill_dirs: &BTreeSet<String>) -> Vec<PathBuf> {
    let has_skills = |folder: &Path| skill_dirs.iter().any(|dir| folder.join(dir).is_dir());
    let mut children: Vec<PathBuf> = fs::read_dir(picked)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .map(|entry| entry.path())
        .filter(|child| has_skills(child))
        .collect();
    children.sort();
    let found: Vec<_> = has_skills(picked)
        .then(|| picked.to_owned())
        .into_iter()
        .chain(children)
        .collect();
    if found.is_empty() {
        vec![picked.to_owned()]
    } else {
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs() -> BTreeSet<String> {
        [".agents/skills", ".claude/skills"]
            .map(String::from)
            .into()
    }

    #[test]
    fn a_folder_adds_itself_and_each_child_with_an_agent_skill_directory() -> std::io::Result<()> {
        let root = tempfile::tempdir()?;
        let picked = root.path();
        for folder in [
            "api/.claude/skills",
            "web/.agents/skills",
            "docs",
            ".agents/skills",
        ] {
            fs::create_dir_all(picked.join(folder))?;
        }
        fs::create_dir_all(picked.join("deep/nested/.claude/skills"))?;
        fs::write(picked.join("notes.txt"), "")?;
        assert_eq!(
            projects(picked, &dirs()),
            [picked.to_owned(), picked.join("api"), picked.join("web")]
        );
        Ok(())
    }

    #[test]
    fn a_folder_without_agent_skill_directories_adds_itself() -> std::io::Result<()> {
        let root = tempfile::tempdir()?;
        fs::create_dir_all(root.path().join("docs"))?;
        assert_eq!(projects(root.path(), &dirs()), [root.path().to_owned()]);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn linked_children_are_skipped() -> std::io::Result<()> {
        let root = tempfile::tempdir()?;
        let outside = root.path().join("outside");
        fs::create_dir_all(outside.join(".claude/skills"))?;
        let picked = root.path().join("picked");
        fs::create_dir_all(picked.join("real/.claude/skills"))?;
        std::os::unix::fs::symlink(&outside, picked.join("linked"))?;
        assert_eq!(projects(&picked, &dirs()), [picked.join("real")]);
        Ok(())
    }
}
