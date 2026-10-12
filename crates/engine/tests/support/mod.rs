#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
use skillbinder_engine::{Engine, EngineConfig};
use skillbinder_proto::{
    AppResult, Candidate, ErrorCategory, OnboardingStep, ScanSnapshot, ScanStatus, SkillId,
    WorkspaceAddition,
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

    /// Adds the fixture workspace and waits for its scan to be saved.
    pub async fn register(&self, engine: &Engine) -> WorkspaceAddition {
        add_workspace(engine, &self.workspace).await
    }

    pub async fn scan(&self, engine: &Engine) -> ScanSnapshot {
        let started = engine.start_scan().await.unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let current = engine.current_scan().unwrap().unwrap();
            if current.status != ScanStatus::Running {
                assert_eq!(
                    current.status,
                    ScanStatus::Finished,
                    "scan failed: {:?}",
                    current.error
                );
                assert_eq!(current.id, started.id);
                return current;
            }
            assert!(
                Instant::now() < deadline,
                "scan did not finish within 10 seconds"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    pub async fn import_one(
        &self,
        engine: &Engine,
        candidate: &Candidate,
        scan: &ScanSnapshot,
    ) -> SkillId {
        let plan = engine
            .prepare_import(scan.id.clone(), vec![candidate.id.clone()], false)
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

pub async fn add_workspace(engine: &Engine, path: &Path) -> WorkspaceAddition {
    let grant = engine.grant_directory(path.to_owned()).await.unwrap();
    let addition = engine.add_workspace(grant.id).await.unwrap().unwrap();
    assert_eq!(
        addition.status,
        ScanStatus::Finished,
        "{:?}",
        addition.error
    );
    engine.dismiss_workspace_addition().unwrap();
    addition
}

pub fn category<T>(result: AppResult<T>) -> ErrorCategory {
    match result {
        Ok(_) => panic!("operation unexpectedly succeeded"),
        Err(error) => error.category,
    }
}
