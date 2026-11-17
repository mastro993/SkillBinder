//! Onboarding operations: rechecking its facts, and finishing it with its first import, after
//! which the app refreshes as it does on launch.

use crate::Client;
use skillbinder_proto::*;

impl Client {
    /// Rechecks Git and storage, creating the library once onboarding reaches its import.
    pub async fn bootstrap(&self) -> AppResult<Bootstrap> {
        self.refresh_preserving_outcome(self.shared.engine.bootstrap().await)
            .await
    }
    /// Skips the Git step, so onboarding goes on without Git.
    pub async fn skip_git(&self) -> AppResult<Bootstrap> {
        self.refresh_preserving_outcome(self.shared.engine.skip_git().await)
            .await
    }
    /// Starts the system's Git installer; the app then checks for Git until it is found.
    pub async fn install_git(&self) -> AppResult<Bootstrap> {
        self.refresh_preserving_outcome(self.shared.engine.install_git().await)
            .await
    }
    /// Looks for Git in the background, as the Git step does every 3 seconds while it waits on
    /// Git; failures surface as the background error.
    pub fn check_git(&self) {
        let client = self.clone();
        self.shared.runtime.spawn(async move {
            if let Err(error) = client.bootstrap().await {
                client.background_error(error);
            }
        });
    }
    /// Applies the ticked skills of onboarding's import, if any, publishing its progress while it
    /// runs, and finishes onboarding, then refreshes the app as on launch, so the library and the
    /// agents show what the import changed.
    pub async fn finish_onboarding(&self, import: Option<ImportSelection>) -> AppResult<Bootstrap> {
        let finished = self
            .tracked(self.shared.engine.finish_onboarding(import))
            .await;
        let boot = self.refresh_preserving_outcome(finished).await?;
        self.refresh_app();
        Ok(boot)
    }
}
