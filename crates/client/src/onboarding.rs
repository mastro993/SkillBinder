//! Onboarding operations: the steps before the library exists, its creation, and its first
//! import, after which the app refreshes as it does on launch.

use crate::Client;
use skillbinder_proto::*;

impl Client {
    /// Rechecks prerequisites.
    pub async fn bootstrap(&self) -> AppResult<Bootstrap> {
        self.refresh_preserving_outcome(self.shared.engine.bootstrap().await)
            .await
    }
    /// Advances or revisits an allowed onboarding step.
    pub async fn set_onboarding(&self, step: OnboardingStep) -> AppResult<Bootstrap> {
        self.refresh_preserving_outcome(self.shared.engine.set_onboarding(step).await)
            .await
    }
    /// Creates the local library atomically; onboarding then reviews its first import.
    pub async fn create_library(&self) -> AppResult<Bootstrap> {
        self.refresh_preserving_outcome(self.shared.engine.create_library().await)
            .await
    }
    /// Prepares the review of the library's first import from your agents' skills, or `None`
    /// when there is nothing to import.
    pub async fn prepare_first_import(&self) -> AppResult<Option<ImportPlan>> {
        self.shared.engine.prepare_first_import().await
    }
    /// Applies the reviewed first import, if any, and finishes onboarding, then refreshes the app
    /// as on launch, so Discovery and the agents show what the import changed.
    pub async fn complete_onboarding(&self, plan: Option<PlanId>) -> AppResult<Bootstrap> {
        let completed = self.shared.engine.complete_onboarding(plan).await;
        let boot = self.refresh_preserving_outcome(completed).await?;
        self.refresh_app();
        Ok(boot)
    }
}
