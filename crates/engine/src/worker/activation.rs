//! The engine's activation operations: answering for a copy that asks, and retrying directories
//! where the active preset is not applied.

use super::Engine;
use skillbinder_proto::*;

impl Engine {
    /// Replaces or keeps a conflicting, kept, or edited copy of `skill` in `directory`, a global
    /// skill directory as [`ManagedCopy::directory`] names it.
    pub async fn resolve_copy(
        &self,
        directory: String,
        skill: SkillId,
        choice: CopyChoice,
    ) -> AppResult<LibrarySnapshot> {
        self.call(move |state| {
            state.resolve_copy(directory, skill, choice)?;
            state.library_snapshot()
        })
        .await
    }
    /// Tries again every directory where the active preset is not applied, or a managed copy
    /// was not removed.
    pub async fn retry_copies(&self) -> AppResult<LibrarySnapshot> {
        self.call(|state| {
            state.retry_copies()?;
            state.library_snapshot()
        })
        .await
    }
}
