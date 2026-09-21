use crate::onboarding::{OnboardingProgress, OnboardingStep, OnboardingTransitionError};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum GitCheckError {
    #[error("Git was not found in the trusted executable search path")]
    NotFound,
    #[error("Git {found} is unsupported; SkillBinder requires Git {minimum} or newer")]
    Unsupported { found: String, minimum: String },
    #[error("Git returned an unreadable version: {0}")]
    InvalidVersion(String),
    #[error("Git could not be started: {0}")]
    Start(String),
    #[error("Git operation failed: {0}")]
    ProcessFailed(String),
    #[error("Git operation timed out")]
    ProcessTimeout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedGit {
    pub executable: String,
    pub version: String,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum BootstrapError {
    #[error(transparent)]
    Git(#[from] GitCheckError),
    #[error("local state is unavailable: {0}")]
    State(String),
    #[error("application storage is unavailable: {0}")]
    Storage(String),
    #[error("another SkillBinder process owns persistent state")]
    AlreadyRunning,
    #[error("library recovery is required: {0}")]
    RecoveryRequired(String),
    #[error(transparent)]
    InvalidOnboarding(#[from] OnboardingTransitionError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryStatus {
    NotCreated,
    Ready,
    RecoveryRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibrarySnapshot {
    pub status: LibraryStatus,
    pub current_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapSnapshot {
    pub git: Result<VerifiedGit, GitCheckError>,
    pub storage: Result<(), String>,
    pub onboarding: OnboardingProgress,
    pub library_status: LibraryStatus,
    pub current_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedOnboarding {
    pub library_id: String,
    pub current_revision: String,
}

pub trait BootstrapEnvironment: Send + Sync {
    fn verify_storage(&self) -> Result<(), BootstrapError>;
    fn verify_git(&self) -> Result<VerifiedGit, GitCheckError>;
    fn inspect_library(&self) -> LibrarySnapshot;
    fn complete_local(&self) -> Result<CompletedOnboarding, BootstrapError>;
}

pub trait OnboardingState: Send + Sync {
    fn load(&self) -> Result<OnboardingProgress, BootstrapError>;
    fn save(&self, progress: &OnboardingProgress) -> Result<(), BootstrapError>;
}

pub struct BootstrapService {
    environment: Arc<dyn BootstrapEnvironment>,
    state: Arc<dyn OnboardingState>,
}

impl BootstrapService {
    pub fn new(
        environment: Arc<dyn BootstrapEnvironment>,
        state: Arc<dyn OnboardingState>,
    ) -> Self {
        Self { environment, state }
    }

    pub fn snapshot(&self) -> Result<BootstrapSnapshot, BootstrapError> {
        let storage = self
            .environment
            .verify_storage()
            .map_err(|error| error.to_string());
        let onboarding = if storage.is_ok() {
            self.state.load()?
        } else {
            OnboardingProgress::default()
        };
        let git = self.environment.verify_git();
        let library = self.environment.inspect_library();
        Ok(BootstrapSnapshot {
            git,
            storage,
            onboarding,
            library_status: library.status,
            current_revision: library.current_revision,
        })
    }

    pub fn save_step(&self, step: OnboardingStep) -> Result<OnboardingProgress, BootstrapError> {
        self.environment.verify_storage()?;
        let mut progress = self.state.load()?;
        progress.resume_at(step)?;
        self.state.save(&progress)?;
        Ok(progress)
    }

    pub fn complete_local(&self) -> Result<CompletedOnboarding, BootstrapError> {
        self.environment.verify_storage()?;
        let mut progress = self.state.load()?;
        progress.complete()?;
        self.environment.verify_git()?;
        let completed = self.environment.complete_local()?;
        self.state.save(&progress)?;
        Ok(completed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    struct FakeEnvironment {
        storage: Result<(), BootstrapError>,
        git: Result<VerifiedGit, GitCheckError>,
        create_calls: AtomicUsize,
    }

    impl Default for FakeEnvironment {
        fn default() -> Self {
            Self {
                storage: Ok(()),
                git: Ok(VerifiedGit {
                    executable: "/fake/git".into(),
                    version: "2.50.1".into(),
                }),
                create_calls: AtomicUsize::new(0),
            }
        }
    }

    impl BootstrapEnvironment for FakeEnvironment {
        fn verify_storage(&self) -> Result<(), BootstrapError> {
            self.storage.clone()
        }

        fn verify_git(&self) -> Result<VerifiedGit, GitCheckError> {
            self.git.clone()
        }

        fn inspect_library(&self) -> LibrarySnapshot {
            LibrarySnapshot {
                status: LibraryStatus::NotCreated,
                current_revision: None,
            }
        }

        fn complete_local(&self) -> Result<CompletedOnboarding, BootstrapError> {
            self.create_calls.fetch_add(1, Ordering::SeqCst);
            Ok(CompletedOnboarding {
                library_id: "library-id".into(),
                current_revision: "revision".into(),
            })
        }
    }

    #[derive(Default)]
    struct FakeState(Mutex<OnboardingProgress>);

    impl OnboardingState for FakeState {
        fn load(&self) -> Result<OnboardingProgress, BootstrapError> {
            Ok(self.0.lock().unwrap().clone())
        }

        fn save(&self, progress: &OnboardingProgress) -> Result<(), BootstrapError> {
            *self.0.lock().unwrap() = progress.clone();
            Ok(())
        }
    }

    #[test]
    fn snapshot_preserves_actionable_prerequisite_outcomes() {
        let environment = Arc::new(FakeEnvironment {
            storage: Err(BootstrapError::Storage("read only".into())),
            git: Err(GitCheckError::Unsupported {
                found: "2.38.0".into(),
                minimum: "2.39.0".into(),
            }),
            create_calls: AtomicUsize::new(0),
        });
        let service = BootstrapService::new(environment, Arc::new(FakeState::default()));

        let snapshot = service.snapshot().unwrap();

        assert!(snapshot.storage.is_err());
        assert!(matches!(
            snapshot.git,
            Err(GitCheckError::Unsupported { .. })
        ));
        assert_eq!(snapshot.onboarding, OnboardingProgress::default());
    }

    #[test]
    fn progress_resumes_through_state_port() {
        let state = Arc::new(FakeState::default());
        let service = BootstrapService::new(Arc::new(FakeEnvironment::default()), state.clone());
        service.save_step(OnboardingStep::Boundaries).unwrap();
        service.save_step(OnboardingStep::SyncChoice).unwrap();

        let resumed = BootstrapService::new(Arc::new(FakeEnvironment::default()), state)
            .snapshot()
            .unwrap();
        assert_eq!(resumed.onboarding.step, OnboardingStep::SyncChoice);
    }

    #[test]
    fn premature_completion_never_reaches_library_port() {
        let environment = Arc::new(FakeEnvironment::default());
        let service = BootstrapService::new(environment.clone(), Arc::new(FakeState::default()));

        assert!(matches!(
            service.complete_local(),
            Err(BootstrapError::InvalidOnboarding(_))
        ));
        assert_eq!(environment.create_calls.load(Ordering::SeqCst), 0);
    }
}
