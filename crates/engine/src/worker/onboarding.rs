//! The engine's onboarding operations: rechecking its facts, and finishing it with its first
//! import.

use super::Engine;
use skillbinder_proto::*;
use std::future::Future;

impl Engine {
    /// Rechecks Git and storage, creating the library once onboarding reaches its import and
    /// making it a repository once Git is found.
    pub async fn bootstrap(&self) -> AppResult<Bootstrap> {
        self.call(|state| {
            state.recheck_git();
            state.ensure_library()?;
            state.ensure_repository()?;
            state.bootstrap()
        })
        .await
    }
    /// Skips the Git step, so onboarding goes on without Git and creates the library without it.
    pub async fn skip_git(&self) -> AppResult<Bootstrap> {
        self.call(|state| state.skip_git()).await
    }
    /// Applies the ticked skills of onboarding's import, if any, and finishes onboarding,
    /// counting the pairs the import brings in line, like [`Self::apply_import`].
    pub fn finish_onboarding(
        &self,
        import: Option<ImportSelection>,
    ) -> impl Future<Output = AppResult<Bootstrap>> + Send + 'static {
        self.tracked(move |state| state.finish_onboarding(import))
    }
}
