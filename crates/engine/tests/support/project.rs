//! A library imported from Claude Code's global skill directory and a project folder outside the
//! isolated home, for the workspace activation journeys.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#![allow(dead_code, reason = "Not every test crate uses every helper.")]

use crate::support::{Fixture, write_skill};
use skillbinder_engine::Engine;
use skillbinder_proto::{CopyState, LibrarySnapshot, SkillId, Workspace};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

/// The body every library skill is imported with, so a copy written with it matches the library.
pub const LIBRARY: &str = "Library version";

/// A library holding `slugs`, imported from Claude Code's global skill directory, and a project
/// folder that is not a workspace yet.
pub struct Project {
    pub fixture: Fixture,
    pub engine: Engine,
    pub folder: PathBuf,
}
impl Project {
    pub async fn with(slugs: &[&str]) -> Self {
        let fixture = Fixture::new();
        let engine = fixture.ready().await;
        for slug in slugs {
            write_skill(&fixture.home.join(".claude/skills"), slug, LIBRARY);
        }
        if !slugs.is_empty() {
            import_found(&fixture, &engine).await;
        }
        let folder = fs::canonicalize(&fixture.workspace).unwrap();
        Self {
            fixture,
            engine,
            folder,
        }
    }

    /// One of the project's agent skill directories.
    pub fn dir(&self, dir: &str) -> PathBuf {
        self.folder.join(dir)
    }

    pub async fn add(&self) -> Workspace {
        self.fixture.register(&self.engine).await
    }

    pub async fn workspace(&self) -> Workspace {
        let mut workspaces = self.engine.snapshot().await.unwrap().workspaces;
        assert_eq!(workspaces.len(), 1);
        workspaces.remove(0)
    }

    pub async fn library(&self) -> LibrarySnapshot {
        self.engine.snapshot().await.unwrap().library.unwrap()
    }

    /// The project's copies: each directory relative to the project, slug, and state.
    pub async fn states(&self) -> Vec<(String, String, CopyState)> {
        self.library()
            .await
            .copies
            .into_iter()
            .filter_map(|copy| {
                let relative = Path::new(&copy.directory).strip_prefix(&self.folder).ok()?;
                let relative = relative.to_string_lossy().replace('\\', "/");
                Some((relative, copy.skill.to_string(), copy.state))
            })
            .collect()
    }
}

/// Imports every skill a scan finds.
pub async fn import_found(fixture: &Fixture, engine: &Engine) {
    let ids = fixture
        .scan(engine)
        .await
        .candidates
        .into_iter()
        .map(|candidate| candidate.id)
        .collect();
    let plan = engine.prepare_import(ids, false).await.unwrap();
    engine.apply_import(plan.id).await.unwrap();
}

pub fn skill(slug: &str) -> SkillId {
    SkillId(slug.to_owned())
}

pub fn dirs(dirs: &[&str]) -> BTreeSet<String> {
    dirs.iter().map(|dir| (*dir).to_owned()).collect()
}

pub fn row(dir: &str, slug: &str, state: CopyState) -> (String, String, CopyState) {
    (dir.to_owned(), slug.to_owned(), state)
}

pub fn modified(path: &Path) -> SystemTime {
    fs::metadata(path.join("SKILL.md"))
        .unwrap()
        .modified()
        .unwrap()
}

pub fn names(directory: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}
