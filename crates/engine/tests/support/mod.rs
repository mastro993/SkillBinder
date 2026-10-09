#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
use skillbinder_engine::{Engine, EngineConfig};
use skillbinder_proto::{
    AppResult, Candidate, ErrorCategory, OnboardingStep, ScanSnapshot, ScanStatus, SkillId,
};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};
use tempfile::TempDir;

pub struct Fixture {
    _temp: TempDir,
    pub data: PathBuf,
    pub home: PathBuf,
    pub project: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("data");
        let home = temp.path().join("home");
        let project = temp.path().join("project");
        fs::create_dir_all(&project).unwrap();
        Self {
            _temp: temp,
            data,
            home,
            project,
        }
    }

    pub fn open(&self) -> Engine {
        Engine::open(EngineConfig::isolated(self.data.clone(), self.home.clone())).unwrap()
    }

    pub fn skill(&self, slug: &str, body: &str) -> PathBuf {
        let directory = self.project.join(".agents/skills").join(slug);
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("SKILL.md"),
            format!("---\nname: {slug}\ndescription: Test fixture\n---\n{body}\n"),
        )
        .unwrap();
        directory
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

    pub async fn register(&self, engine: &Engine) -> skillbinder_proto::ProjectRoot {
        let grant = engine.grant_directory(self.project.clone()).await.unwrap();
        engine
            .register_root(grant.id, "Fixture".into())
            .await
            .unwrap()
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

pub fn category<T>(result: AppResult<T>) -> ErrorCategory {
    match result {
        Ok(_) => panic!("operation unexpectedly succeeded"),
        Err(error) => error.category,
    }
}
