//! The engine's onboarding operations: rechecking its facts, and finishing it with its first
//! import.

use super::Engine;
use crate::git::Git;
use skillbinder_proto::*;
use std::future::Future;

impl Engine {
    /// Rechecks Git and storage, creating the library once onboarding reaches its import.
    pub async fn bootstrap(&self) -> AppResult<Bootstrap> {
        self.call(|state| {
            state.git = Git::discover(state.config.git_home(), state.shutdown.clone()).ok();
            state.ensure_library()?;
            state.bootstrap()
        })
        .await
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
