#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
use diesel::{connection::SimpleConnection, prelude::*};
use skillbinder_engine::{Engine, EngineConfig, GitSearch};
use skillbinder_proto::{
    AppResult, Candidate, CandidateId, ErrorCategory, ImportOutcome, ImportPlan, ScanSnapshot,
    SkillId, Workspace,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tempfile::TempDir;

pub struct Fixture {
    _temp: TempDir,
    pub data: PathBuf,
    pub home: PathBuf,
    pub workspace: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("data");
        let home = temp.path().join("home");
        let workspace = temp.path().join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        Self {
            _temp: temp,
            data,
            home,
            workspace,
        }
    }

    pub fn open(&self) -> Engine {
        Engine::open(EngineConfig::isolated(self.data.clone(), self.home.clone())).unwrap()
    }

    /// Where an engine opened by [`Self::open_without_git`] looks for Git: empty until a test
    /// puts Git there.
    #[allow(dead_code, reason = "Not every test crate runs without Git.")]
    pub fn git_bin(&self) -> PathBuf {
        self.workspace.with_file_name("bin")
    }

    /// Opens the engine as on a computer without Git: it looks for Git only in [`Self::git_bin`].
    #[allow(dead_code, reason = "Not every test crate runs without Git.")]
    pub fn open_without_git(&self) -> Engine {
        let bin = self.git_bin();
        fs::create_dir_all(&bin).unwrap();
        let mut config = EngineConfig::isolated(self.data.clone(), self.home.clone());
        config.git_search = GitSearch::Directory(bin);
        Engine::open(config).unwrap()
    }

    /// A second connection to the engine's database, with the engine's busy timeout.
    #[allow(
        dead_code,
        reason = "Not every test crate edits the database directly."
    )]
    pub fn database(&self) -> SqliteConnection {
        let path = self.data.join("skillbinder.db");
        let mut database = SqliteConnection::establish(path.to_str().unwrap()).unwrap();
        database.batch_execute("PRAGMA busy_timeout=5000;").unwrap();
        database
    }

    #[allow(dead_code, reason = "Not every test crate writes a workspace skill.")]
    pub fn skill(&self, slug: &str, body: &str) -> PathBuf {
        write_skill(&self.workspace.join(".agents/skills"), slug, body)
    }

    pub async fn ready(&self) -> Engine {
        let engine = self.open();
        engine.finish_onboarding(None).await.unwrap();
        engine
    }

    /// Adds the fixture workspace.
    pub async fn register(&self, engine: &Engine) -> Workspace {
        add_workspace(engine, &self.workspace).await
    }

    pub async fn scan(&self, engine: &Engine) -> ScanSnapshot {
        scan(engine).await
    }

    #[allow(dead_code, reason = "Not every test crate installs a skill.")]
    pub async fn import_one(&self, engine: &Engine, candidate: &Candidate) -> SkillId {
        let outcome = import(engine, vec![candidate.id.clone()]).await;
        assert_eq!(outcome.imported.len(), 1);
        outcome.imported[0].clone()
    }
}

/// Scan for skills choosing the versions `ids`, its plan narrowed to them.
#[allow(dead_code, reason = "Not every test crate imports skills.")]
pub async fn prepare(engine: &Engine, ids: &[CandidateId]) -> ImportPlan {
    let mut plan = engine
        .prepare_found_import(ids.to_vec())
        .await
        .unwrap()
        .plan;
    plan.items.retain(|item| ids.contains(&item.candidate_id));
    assert_eq!(
        plan.items.len(),
        ids.len(),
        "every chosen version is planned"
    );
    plan
}

/// Imports every skill `plan` lists.
#[allow(dead_code, reason = "Not every test crate imports skills.")]
pub async fn apply(engine: &Engine, plan: &ImportPlan) -> ImportOutcome {
    let ids = plan.items.iter().map(|item| item.candidate_id.clone());
    engine
        .apply_import(plan.id.clone(), ids.collect(), false)
        .await
        .unwrap()
}

/// Imports the found versions `ids` through Scan for skills.
#[allow(dead_code, reason = "Not every test crate imports skills.")]
pub async fn import(engine: &Engine, ids: Vec<CandidateId>) -> ImportOutcome {
    apply(engine, &prepare(engine, &ids).await).await
}

/// Writes `<directory>/<slug>/SKILL.md` with a body that sets its content digest.
pub fn write_skill(directory: &Path, slug: &str, body: &str) -> PathBuf {
    let directory = directory.join(slug);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("SKILL.md"),
        format!("---\nname: {slug}\ndescription: Test fixture\n---\n{body}\n"),
    )
    .unwrap();
    directory
}

/// Adds a folder that is a single project as one workspace.
pub async fn add_workspace(engine: &Engine, path: &Path) -> Workspace {
    let grant = engine.grant_directory(path.to_owned()).await.unwrap();
    let mut added = engine.add_workspaces(grant.id).await.unwrap();
    assert_eq!(added.len(), 1, "{added:?}");
    added.remove(0)
}

/// Scans the agents' global skill directories and every workspace and waits for the scan to complete.
pub async fn scan(engine: &Engine) -> ScanSnapshot {
    engine.start_scan().await.unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let (found, running) = engine.current_scan().unwrap();
        if !running {
            let found = found.unwrap();
            assert_eq!(found.error, None, "scan failed");
            return found;
        }
        assert!(
            Instant::now() < deadline,
            "scan did not finish within 10 seconds"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[allow(
    dead_code,
    reason = "Not every test crate expects an operation to fail."
)]
pub fn category<T>(result: AppResult<T>) -> ErrorCategory {
    match result {
        Ok(_) => panic!("operation unexpectedly succeeded"),
        Err(error) => error.category,
    }
}
