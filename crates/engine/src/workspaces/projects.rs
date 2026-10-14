//! Which folders a picked folder adds as workspaces: the folder itself and each direct child
//! that has an agent skill directory, or the folder alone when none has one. An agent folder,
//! such as the home folder, is never a workspace.

use crate::discovery::paths::resolve_absent;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

/// What picking a folder adds.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Projects {
    /// The folders to add, the picked folder first, then its children by name.
    pub add: Vec<PathBuf>,
    /// Agent folders left out: the picked folder itself, or children with agent skill
    /// directories. When nothing is added, these explain the refusal.
    pub skipped: Vec<PathBuf>,
}

/// The folders `picked` adds. Linked children are skipped, so every folder stays inside the
/// picked one, and so are hidden ones, such as a project's own `.agents` folder, whose `skills`
/// would otherwise match an agent that reads `skills`. Agent folders are skipped too, so a
/// picked agent folder, or a folder whose only projects are agent folders, adds nothing; a
/// folder with no projects at all adds itself.
pub(super) fn projects(
    picked: &Path,
    skill_dirs: &BTreeSet<String>,
    global_dirs: &BTreeSet<PathBuf>,
) -> Projects {
    let has_skills = |folder: &Path| skill_dirs.iter().any(|dir| folder.join(dir).is_dir());
    let agent_folder = |folder: &Path| is_agent_folder(folder, skill_dirs, global_dirs);
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
    let (mut skipped, children): (Vec<_>, Vec<_>) =
        children.into_iter().partition(|child| agent_folder(child));
    let picked_is_project = if agent_folder(picked) {
        skipped.insert(0, picked.to_owned());
        false
    } else {
        has_skills(picked)
    };
    let mut add: Vec<_> = picked_is_project
        .then(|| picked.to_owned())
        .into_iter()
        .chain(children)
        .collect();
    if add.is_empty() && skipped.is_empty() {
        add.push(picked.to_owned());
    }
    Projects { add, skipped }
}

/// An agent folder is one whose agent workspace skill directories include a global skill
/// directory, such as the home folder with `~/.claude/skills` or a project whose `.claude/skills`
/// links to it, or one at or inside a global skill directory, such as `~/.claude/skills` itself.
/// Its skills are shown under Agents.
pub(super) fn is_agent_folder(
    folder: &Path,
    skill_dirs: &BTreeSet<String>,
    global_dirs: &BTreeSet<PathBuf>,
) -> bool {
    // Links are followed as the global directories' own are, including to a missing target.
    let global = |path: PathBuf| {
        let root = path.ancestors().last().unwrap_or(Path::new("/"));
        global_dirs.contains(&path)
            || resolve_absent(root, &path).is_ok_and(|resolved| global_dirs.contains(&resolved))
    };
    global_dirs.iter().any(|dir| folder.starts_with(dir))
        || skill_dirs.iter().any(|dir| global(folder.join(dir)))
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
            projects(picked, &dirs(), &BTreeSet::new()).add,
            [picked.to_owned(), picked.join("api"), picked.join("web")]
        );
        Ok(())
    }

    #[test]
    fn a_folder_without_agent_skill_directories_adds_itself() -> std::io::Result<()> {
        let root = tempfile::tempdir()?;
        fs::create_dir_all(root.path().join("docs"))?;
        assert_eq!(
            projects(root.path(), &dirs(), &BTreeSet::new()).add,
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
            projects(&picked, &dirs(), &BTreeSet::new()).add,
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
            Projects {
                add: vec![home.join("api")],
                skipped: vec![home.clone()],
            },
            "a picked agent folder still adds its projects"
        );
        assert_eq!(
            projects(root.path(), &dirs(), &global),
            Projects {
                add: vec![root.path().join("web")],
                skipped: vec![home.clone()],
            },
            "an agent folder among the children is skipped"
        );
        fs::remove_dir_all(home.join("api"))?;
        assert!(projects(&home, &dirs(), &global).add.is_empty());
        fs::remove_dir_all(root.path().join("web"))?;
        assert_eq!(
            projects(root.path(), &dirs(), &global),
            Projects {
                add: Vec::new(),
                skipped: vec![home],
            },
            "a folder whose only project is an agent folder adds nothing"
        );
        Ok(())
    }

    #[test]
    fn a_global_skill_directory_and_its_contents_are_agent_folders() -> std::io::Result<()> {
        let root = tempfile::tempdir()?;
        let global = root.path().join("home/.claude/skills");
        fs::create_dir_all(global.join("review"))?;
        let globals: BTreeSet<PathBuf> = [global.clone()].into();
        for picked in [global.clone(), global.join("review")] {
            assert_eq!(
                projects(&picked, &dirs(), &globals),
                Projects {
                    add: Vec::new(),
                    skipped: vec![picked.clone()],
                },
                "{}",
                picked.display()
            );
        }
        let sibling = root.path().join("home/.claude/skills-old");
        fs::create_dir_all(&sibling)?;
        assert!(
            !is_agent_folder(&sibling, &dirs(), &globals),
            "a sibling sharing a prefix is not inside"
        );
        Ok(())
    }
}
