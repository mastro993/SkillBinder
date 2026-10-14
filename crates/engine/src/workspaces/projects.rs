//! Which folders a picked folder adds as workspaces: the folder itself and each direct child
//! that has an agent skill directory, or the folder alone when none has one. An agent folder,
//! such as the home folder, is never a workspace.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

/// The folders `picked` adds, the picked folder first, then its children by name. Linked
/// children are skipped, so every folder stays inside the picked one, and so are hidden ones,
/// such as a project's own `.agents` folder, whose `skills` would otherwise match an agent that
/// reads `skills`. Agent folders are skipped too, so a picked agent folder with no projects in
/// it adds nothing.
pub(super) fn projects(
    picked: &Path,
    skill_dirs: &BTreeSet<String>,
    global_dirs: &BTreeSet<PathBuf>,
) -> Vec<PathBuf> {
    let has_skills = |folder: &Path| skill_dirs.iter().any(|dir| folder.join(dir).is_dir());
    let agent_folder = |folder: &Path| is_agent_folder(folder, skill_dirs, global_dirs);
    let mut children: Vec<PathBuf> = fs::read_dir(picked)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
        .map(|entry| entry.path())
        .filter(|child| has_skills(child) && !agent_folder(child))
        .collect();
    children.sort();
    if agent_folder(picked) {
        return children;
    }
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

/// An agent folder is one whose agent workspace skill directories include a global skill
/// directory, such as the home folder with `~/.claude/skills`. Its skills are shown under Agents.
pub(super) fn is_agent_folder(
    folder: &Path,
    skill_dirs: &BTreeSet<String>,
    global_dirs: &BTreeSet<PathBuf>,
) -> bool {
    skill_dirs
        .iter()
        .any(|dir| global_dirs.contains(&folder.join(dir)))
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
            projects(picked, &dirs(), &BTreeSet::new()),
            [picked.to_owned(), picked.join("api"), picked.join("web")]
        );
        Ok(())
    }

    #[test]
    fn a_folder_without_agent_skill_directories_adds_itself() -> std::io::Result<()> {
        let root = tempfile::tempdir()?;
        fs::create_dir_all(root.path().join("docs"))?;
        assert_eq!(
            projects(root.path(), &dirs(), &BTreeSet::new()),
            [root.path().to_owned()]
        );
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
        assert_eq!(
            projects(&picked, &dirs(), &BTreeSet::new()),
            [picked.join("real")]
        );
        Ok(())
    }

    #[test]
    fn agent_folders_are_never_projects() -> std::io::Result<()> {
        let root = tempfile::tempdir()?;
        let home = root.path().join("home");
        for folder in [
            "home/.claude/skills",
            "home/api/.claude/skills",
            "web/.agents/skills",
        ] {
            fs::create_dir_all(root.path().join(folder))?;
        }
        let global: BTreeSet<PathBuf> = [home.join(".claude/skills")].into();
        assert_eq!(
            projects(&home, &dirs(), &global),
            [home.join("api")],
            "a picked agent folder still adds its projects"
        );
        fs::remove_dir_all(home.join("api"))?;
        assert!(projects(&home, &dirs(), &global).is_empty());
        assert_eq!(
            projects(root.path(), &dirs(), &global),
            [root.path().join("web")],
            "an agent folder among the children is skipped"
        );
        Ok(())
    }
}
