//! The engine's onboarding operations: the steps before the library exists, its creation, and its
//! first import.

use super::Engine;
use crate::git::Git;
use skillbinder_proto::*;
use std::future::Future;

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
    /// Applies the ticked skills of onboarding's import, if any, and finishes onboarding,
    /// counting the pairs the import brings in line, like [`Self::apply_import`].
    pub fn complete_onboarding(
        &self,
        import: Option<ImportSelection>,
    ) -> impl Future<Output = AppResult<Bootstrap>> + Send + 'static {
        self.tracked(move |state| state.complete_onboarding(import))
    }
}
