#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
use skillbinder_engine::{Engine, EngineConfig};
use skillbinder_proto::{
    AppResult, Candidate, ErrorCategory, OnboardingStep, ScanSnapshot, SkillId, Workspace,
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

    pub fn skill(&self, slug: &str, body: &str) -> PathBuf {
        write_skill(&self.workspace.join(".agents/skills"), slug, body)
    }

    pub async fn ready(&self) -> Engine {
        let engine = self.open();
        for step in [
            OnboardingStep::Boundaries,
            OnboardingStep::SyncChoice,
            OnboardingStep::Ready,
        ] {
            engine.set_onboarding(step).await.unwrap();
        }
        engine.complete_onboarding().await.unwrap();
        engine
    }

    /// Adds the fixture workspace.
    pub async fn register(&self, engine: &Engine) -> Workspace {
        add_workspace(engine, &self.workspace).await
    }

    pub async fn scan(&self, engine: &Engine) -> ScanSnapshot {
        scan(engine).await
    }

    pub async fn import_one(&self, engine: &Engine, candidate: &Candidate) -> SkillId {
        let plan = engine
            .prepare_import(vec![candidate.id.clone()], false)
            .await
            .unwrap();
        let outcome = engine.apply_import(plan.id).await.unwrap();
        assert_eq!(outcome.imported.len(), 1);
        outcome.imported[0].clone()
    }
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

pub fn category<T>(result: AppResult<T>) -> ErrorCategory {
    match result {
        Ok(_) => panic!("operation unexpectedly succeeded"),
        Err(error) => error.category,
    }
}
