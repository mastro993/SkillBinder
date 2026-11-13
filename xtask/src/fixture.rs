//! Builds the synthetic world in `fixtures/synthetic/` at a disposable path, so a native-test
//! launch runs on fictional agents, projects, skill sources, and a Sync remote instead of the
//! user's own.

use anyhow::{Context as _, bail, ensure};
use serde_json::{Map, Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Placeholder files that keep an empty directory in Git; the world gets the directory only.
const KEEP: &str = ".gitkeep";

pub(crate) fn build(root: &Path, out: Option<&str>) -> anyhow::Result<()> {
    let Some(out) = out else {
        bail!("Usage: cargo run -p xtask -- fixture <empty-or-missing-directory>");
    };
    let out = PathBuf::from(out);
    if out.exists() {
        ensure!(
            fs::read_dir(&out)?.next().is_none(),
            "{} is not empty; choose a new directory",
            out.display()
        );
    }
    fs::create_dir_all(&out)?;
    let out = fs::canonicalize(&out)?;
    let world = root.join("fixtures/synthetic");
    copy_tree(&world.join("home"), &out.join("home"))?;
    copy_tree(&world.join("projects"), &out.join("projects"))?;
    let mut lock = Map::new();
    for source in children(&world.join("sources"))? {
        install_source(&source, &out, &mut lock)?;
    }
    for remote in children(&world.join("remotes"))? {
        let bare = out.join("remotes").join(repo_name(&remote)?);
        commit_snapshots(&bare, &[remote])?;
    }
    let lock = json!({ "version": 3, "skills": lock });
    fs::write(
        out.join("home/.agents/.skill-lock.json"),
        serde_json::to_string_pretty(&lock)? + "\n",
    )?;
    print_usage(&out);
    Ok(())
}

/// Publishes a source's numbered snapshots as commits, oldest first, and installs the first
/// snapshot's skills into the universal agents' directory, as the `skills` CLI would, so the
/// later snapshots show as updates.
fn install_source(source: &Path, out: &Path, lock: &mut Map<String, Value>) -> anyhow::Result<()> {
    let mut snapshots = children(source)?;
    snapshots.sort_by_key(|snapshot| {
        snapshot
            .file_name()
            .and_then(|name| name.to_str()?.parse::<u32>().ok())
    });
    let Some(first) = snapshots.first() else {
        bail!("{} has no snapshots", source.display());
    };
    let bare = out.join("sources").join(repo_name(source)?);
    let installed = commit_snapshots(&bare, &snapshots)?;
    let url = file_url(&bare)?;
    for skill in children(&first.join("skills"))? {
        let slug = file_name(&skill)?;
        copy_tree(&skill, &out.join("home/.agents/skills").join(&slug))?;
        let object = format!("{installed}:skills/{slug}");
        let tree = git(&bare, None, &["rev-parse", &object])?;
        lock.insert(
            slug.clone(),
            json!({
                "source": url,
                "sourceType": "git",
                "sourceUrl": url,
                "skillPath": format!("skills/{slug}/SKILL.md"),
                "skillFolderHash": tree,
            }),
        );
    }
    Ok(())
}

/// Creates the bare repository `bare` with one commit per snapshot, at fixed dates so every
/// build gives the same commits, and returns the first commit.
fn commit_snapshots(bare: &Path, snapshots: &[PathBuf]) -> anyhow::Result<String> {
    fs::create_dir_all(bare)?;
    git(
        bare,
        None,
        &["init", "--bare", "--quiet", "--initial-branch=main"],
    )?;
    let mut first = None;
    for (day, snapshot) in (1..).zip(snapshots) {
        let files = Some((snapshot.as_path(), format!("2026-01-{day:02}T12:00:00Z")));
        // Forced, so no ignore file outside the fixture leaves a snapshot's file out.
        git(bare, files.clone(), &["add", "--all", "--force", "."])?;
        let message = format!("Publish snapshot {}", file_name(snapshot)?);
        git(bare, files, &["commit", "--quiet", "-m", &message])?;
        let head = git(bare, None, &["rev-parse", "HEAD"])?;
        first.get_or_insert(head);
    }
    first.with_context(|| format!("{} has no snapshots", bare.display()))
}

/// Runs Git on `bare`, reading no user or system configuration. With `files`, its work tree and
/// the date its commits carry.
fn git(bare: &Path, files: Option<(&Path, String)>, args: &[&str]) -> anyhow::Result<String> {
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
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
        ])
        .arg("--git-dir")
        .arg(bare);
    if let Some((work_tree, date)) = files {
        command
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .arg("--work-tree")
            .arg(work_tree);
    }
    let output = command.args(args).output().context("Could not run Git")?;
    ensure!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn copy_tree(from: &Path, to: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if entry.file_name() != KEEP {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// The directories in `parent`, by name.
fn children(parent: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut children = Vec::new();
    for entry in fs::read_dir(parent).with_context(|| format!("Missing {}", parent.display()))? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            children.push(entry.path());
        }
    }
    children.sort();
    Ok(children)
}

fn file_name(path: &Path) -> anyhow::Result<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .with_context(|| format!("{} has no UTF-8 name", path.display()))
}

fn repo_name(path: &Path) -> anyhow::Result<String> {
    Ok(format!("{}.git", file_name(path)?))
}

/// A `file://` URL SkillBinder accepts as a source or remote; it refuses ones that need escaping.
fn file_url(path: &Path) -> anyhow::Result<String> {
    let path = path
        .to_str()
        .context("The fixture path must be valid UTF-8")?
        .replace('\\', "/");
    ensure!(
        !path.chars().any(|c| c.is_whitespace() || "%#?".contains(c)),
        "Choose a fixture path without spaces, %, #, or ?"
    );
    Ok(if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    })
}

fn print_usage(out: &Path) {
    let (data, home, projects) = (out.join("data"), out.join("home"), out.join("projects"));
    let remote = file_url(&out.join("remotes/library.git")).unwrap_or_default();
    println!("Synthetic world ready at {}", out.display());
    println!();
    println!("Launch it:");
    println!(
        "  cargo run -p skillbinder --features native-test --locked -- --data-dir {} --home {}",
        data.display(),
        home.display()
    );
    println!();
    println!(
        "After setup, add workspaces by picking {}",
        projects.display()
    );
    println!(
        "  (or the MCP command add_workspace {{\"path\":\"{}\"}}).",
        projects.display()
    );
    println!("Sync remote: {remote}");
}
