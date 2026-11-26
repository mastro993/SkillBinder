//! Found skills: preparing their import from a fresh scan, and ignoring the ones the user never
//! wants offered (ADR 0033).

use crate::Client;
use skillbinder_proto::*;

impl Client {
    /// Scans to completion, replacing a running scan, and prepares one import plan for every found
    /// skill not in library, taking the versions `versions` chooses. The fresh scan is published,
    /// so the count of found skills follows it.
    pub async fn prepare_found_import(&self, versions: Vec<CandidateId>) -> AppResult<FoundImport> {
        let prepared = self.shared.engine.prepare_found_import(versions).await;
        self.refresh_preserving_outcome(prepared).await
    }
    /// Plans another version of one skill in `plan`, keeping its revision and expiry.
    pub async fn prepare_import_version(
        &self,
        plan: PlanId,
        version: CandidateId,
    ) -> AppResult<ImportPlan> {
        self.shared
            .engine
            .prepare_import_version(plan, version)
            .await
    }
    /// Ignores the found skills `slugs`, or stops ignoring them, then publishes the snapshot, so
    /// the list and Scan for skills' count follow at once. Nothing changes on disk, so no scan
    /// follows.
    pub async fn set_ignored(&self, slugs: Vec<String>, ignored: bool) -> AppResult<()> {
        self.refresh_preserving_outcome(self.shared.engine.set_ignored(slugs, ignored).await)
            .await
    }
}
