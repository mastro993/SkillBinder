//! Lists a curated repository's skills: a shallow, blobless clone that checks out only SKILL.md
//! files, the `skills` CLI's discovery rules, and each SKILL.md's frontmatter.

use super::{curation::Repository, discovery, frontmatter, rules};
use anyhow::{Context as _, ensure};
use std::{collections::BTreeSet, ffi::OsStr, fs, path::Path, process::Command};

/// A skill as the catalog lists it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Listed {
    pub(super) slug: String,
    pub(super) description: String,
    /// The skill's folder in its repository, empty for a root SKILL.md.
    pub(super) path: String,
}

/// Clones `repository` under `scratch` and lists its skills. `git_config` holds extra
/// `name=value` settings for every Git command, which tests use to map URLs to local copies.
pub(super) fn list(
    repository: &Repository,
    agent_dirs: &[String],
    git_config: &[String],
    scratch: &Path,
) -> anyhow::Result<Vec<Listed>> {
    let clone = tempfile::tempdir_in(scratch)?;
    let dir = clone.path();
    let url = repository.url.as_str();
    let clone_args = [
        "clone",
        "--quiet",
        "--depth",
        "1",
        "--filter=blob:none",
        "--no-checkout",
    ];
    let mut args: Vec<&OsStr> = clone_args.iter().map(OsStr::new).collect();
    args.extend([OsStr::new("--"), OsStr::new(url), dir.as_os_str()]);
    git(git_config, None, &args)?;
    let tree = git(
        git_config,
        Some(dir),
        &["ls-tree", "-r", "-z", "--full-tree", "HEAD"],
    )?;
    let files = regular_files(&tree);
    let folders = discovery::skill_folders(&files, agent_dirs);
    // Fetches only the SKILL.md blobs, in one batch.
    git(
        git_config,
        Some(dir),
        &["sparse-checkout", "set", "--no-cone", "SKILL.md"],
    )?;
    git(git_config, Some(dir), &["checkout", "--quiet"])?;
    let mut slugs = BTreeSet::new();
    let mut listed = Vec::new();
    for path in folders {
        if repository.excluded.contains(&path) {
            continue;
        }
        let file = dir.join(&path).join("SKILL.md");
        let skip = |reason: &str| println!("::warning::Skipped {url} {path:?}: {reason}");
        if !rules::path(&path) {
            skip("its folder is not a plain relative path");
            continue;
        }
        let Ok(text) = fs::read(&file) else {
            skip("SKILL.md is unreadable");
            continue;
        };
        let Some(front) = frontmatter::parse(&text) else {
            skip("its frontmatter is missing or malformed");
            continue;
        };
        if front.internal {
            continue;
        }
        let Some(slug) = front.name.filter(|name| rules::slug(name)) else {
            skip("its name is missing or breaks the library's name rule");
            continue;
        };
        let Some(description) = front.description.filter(|d| !d.trim().is_empty()) else {
            skip("its description is missing");
            continue;
        };
        if !slugs.insert(slug.clone()) {
            skip("another skill in the repository has its name");
            continue;
        }
        listed.push(Listed {
            slug,
            description: rules::description(&description),
            path,
        });
    }
    Ok(listed)
}

/// The paths of regular files in `git ls-tree -r -z` output; symbolic links and submodules are
/// never skills.
fn regular_files(tree: &str) -> Vec<String> {
    tree.split('\0')
        .filter_map(|entry| {
            let (meta, path) = entry.split_once('\t')?;
            let mut meta = meta.split(' ');
            let mode = meta.next()?;
            (meta.next()? == "blob" && matches!(mode, "100644" | "100755")).then(|| path.to_owned())
        })
        .collect()
}

/// Runs Git with no user or system configuration and no prompts, in `dir` when given.
fn git<S: AsRef<OsStr>>(
    config: &[String],
    dir: Option<&Path>,
    args: &[S],
) -> anyhow::Result<String> {
    let mut command = Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", null)
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(["-c", "core.symlinks=false"]);
    for setting in config {
        command.args(["-c", setting]);
    }
    if let Some(dir) = dir {
        command.arg("-C").arg(dir);
    }
    let output = command.args(args).output().context("Could not run Git")?;
    ensure!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(String::from_utf8(output.stdout)?)
}

#[cfg(test)]
mod tests {
    use super::regular_files;

    #[test]
    fn only_regular_files_are_listed() {
        let tree = "100644 blob a1\tskills/a/SKILL.md\0\
                    100755 blob a2\trun.sh\0\
                    120000 blob a3\tskills/b/SKILL.md\0\
                    160000 commit a4\tvendor\0";
        assert_eq!(regular_files(tree), ["skills/a/SKILL.md", "run.sh"]);
    }
}
