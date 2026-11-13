//! The engine's onboarding operations: the steps before the library exists, its creation, and the
//! review of its first import.

use super::Engine;
use crate::git::Git;
use skillbinder_proto::*;

impl Engine {
    /// Rechecks prerequisites and persisted onboarding progress.
    pub async fn bootstrap(&self) -> AppResult<Bootstrap> {
        self.call(|state| {
            state.git = Git::discover(state.config.git_home(), state.shutdown.clone()).ok();
            state.bootstrap()
        })
        .await
    }
    /// Persists one legal onboarding transition before the library exists.
    pub async fn set_onboarding(&self, step: OnboardingStep) -> AppResult<Bootstrap> {
        self.call(move |state| state.set_onboarding(step)).await
    }
    /// Atomically creates the local library at the Ready step, moving onboarding to the review of
    /// its first import.
    pub async fn create_library(&self) -> AppResult<Bootstrap> {
        self.call(|state| state.create_library()).await
    }
    /// Scans your agents' global skill directories and prepares the review of the library's first
    /// import, or `None` when nothing there can be imported.
    pub async fn prepare_first_import(&self) -> AppResult<Option<ImportPlan>> {
        self.call(|state| state.prepare_first_import()).await
    }
    /// Applies the reviewed first import, if any, and finishes onboarding.
    pub async fn complete_onboarding(&self, plan: Option<PlanId>) -> AppResult<Bootstrap> {
        self.call(move |state| state.complete_onboarding(plan))
            .await
    }
}
