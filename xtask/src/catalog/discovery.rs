//! Finds a repository's skill folders the way the `skills` CLI does (`vercel-labs/skills`,
//! `discoverSkills` in `src/skills.ts`): a root SKILL.md alone; else the root's direct children
//! and the known containers to depth 3; else every folder to depth 5.

use std::collections::BTreeSet;

/// Containers the CLI searches beside the agents' workspace skill directories.
const CONTAINERS: [&str; 4] = [
    "skills",
    "skills/.curated",
    "skills/.experimental",
    "skills/.system",
];

/// Workspace skill directories agents used before the universal `.agents/skills`, which
/// repositories still publish to (`LEGACY_AGENT_PROJECT_SKILL_DIRS` in `src/agents.ts`).
const LEGACY_AGENT_DIRS: [&str; 8] = [
    ".cline/skills",
    ".codex/skills",
    ".factory/skills",
    ".github/skills",
    ".kilo/skills",
    ".kilocode/skills",
    ".opencode/skills",
    ".pi/skills",
];

/// Folders the CLI never descends through.
const SKIP: [&str; 5] = ["node_modules", ".git", "dist", "build", "__pycache__"];

const CONTAINER_DEPTH: usize = 3;
const RECURSIVE_DEPTH: usize = 5;

/// The skill folders among `files` (a repository's file paths), in discovery order. `agent_dirs`
/// are the registry's workspace skill directories.
pub(super) fn skill_folders(files: &[String], agent_dirs: &[String]) -> Vec<String> {
    let folders: BTreeSet<&str> = files
        .iter()
        .filter_map(|file| match file.strip_suffix("SKILL.md")? {
            "" => Some(""),
            folder => folder.strip_suffix('/'),
        })
        .collect();
    if folders.contains("") {
        return vec![String::new()];
    }
    let mut containers: Vec<&str> = agent_dirs
        .iter()
        .map(String::as_str)
        .chain(LEGACY_AGENT_DIRS)
        .filter(|dir| *dir != "skills")
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    containers.splice(0..0, CONTAINERS);
    let mut found: Vec<&str> = folders
        .iter()
        .copied()
        .filter(|folder| !folder.contains('/'))
        .collect();
    for container in containers {
        let prefix = format!("{container}/");
        for folder in &folders {
            let Some(rest) = folder.strip_prefix(&prefix) else {
                continue;
            };
            if within(&folders, &prefix, rest, CONTAINER_DEPTH) && !found.contains(folder) {
                found.push(folder);
            }
        }
    }
    if found.is_empty() {
        found = folders
            .iter()
            .copied()
            .filter(|folder| {
                let segments: Vec<&str> = folder.split('/').collect();
                segments.len() <= RECURSIVE_DEPTH && !segments.iter().any(|s| SKIP.contains(s))
            })
            .collect();
    }
    found.into_iter().map(str::to_owned).collect()
}

/// Whether the folder `rest` under `prefix` is at most `depth` deep, under no skipped folder and
/// no other skill: the CLI stops descending at a skill.
fn within(folders: &BTreeSet<&str>, prefix: &str, rest: &str, depth: usize) -> bool {
    let segments: Vec<&str> = rest.split('/').collect();
    let ancestors = &segments[..segments.len() - 1];
    segments.len() <= depth
        && !ancestors.iter().any(|segment| SKIP.contains(segment))
        && !(1..segments.len())
            .any(|end| folders.contains(format!("{prefix}{}", segments[..end].join("/")).as_str()))
}

#[cfg(test)]
mod tests {
    use super::skill_folders;

    fn find(files: &[&str]) -> Vec<String> {
        let files: Vec<String> = files.iter().map(|file| (*file).to_owned()).collect();
        skill_folders(&files, &[".claude/skills".to_owned(), "skills".to_owned()])
    }

    #[test]
    fn a_root_skill_is_the_only_one() {
        assert_eq!(find(&["SKILL.md", "skills/a/SKILL.md"]), [""]);
    }

    #[test]
    fn containers_are_searched_to_depth_three_and_stop_at_a_skill() {
        assert_eq!(
            find(&[
                "README.md",
                "top/SKILL.md",
                "skills/a/SKILL.md",
                "skills/group/b/SKILL.md",
                "skills/x/y/c/SKILL.md",
                "skills/x/y/z/too-deep/SKILL.md",
                "skills/a/nested/SKILL.md",
                "skills/node_modules/d/SKILL.md",
                "skills/.curated/e/SKILL.md",
                ".claude/skills/f/SKILL.md",
                ".codex/skills/g/SKILL.md",
                "examples/deep/h/SKILL.md",
            ]),
            [
                "top",
                "skills/.curated/e",
                "skills/a",
                "skills/group/b",
                "skills/x/y/c",
                ".claude/skills/f",
                ".codex/skills/g",
            ]
        );
    }

    #[test]
    fn without_known_locations_every_folder_to_depth_five_counts() {
        assert_eq!(
            find(&[
                "a/b/SKILL.md",
                "a/b/c/SKILL.md",
                "1/2/3/4/5/SKILL.md",
                "1/2/3/4/5/6/SKILL.md",
                "x/dist/y/SKILL.md",
            ]),
            ["1/2/3/4/5", "a/b", "a/b/c"]
        );
    }

    #[test]
    fn only_files_named_skill_md_mark_a_skill() {
        assert!(find(&["skills/a/NOTSKILL.md", "skills/b/skill.md"]).is_empty());
    }
}
