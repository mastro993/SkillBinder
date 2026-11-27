//! The engine's library Sync: connecting a remote, and each Git operation the user asks for.

use super::{Engine, State, action::Action};
use crate::{
    git::Committer,
    logging::{Area, Level},
    pending_changes,
    reconcile::RECONCILED_HEAD,
};
use skillbinder_proto::*;
use std::collections::BTreeMap;

impl Engine {
    /// Validates and fetches a remote before saving its configuration.
    pub async fn connect_remote(&self, remote: RemoteConfig) -> AppResult<SyncSnapshot> {
        let subject = vec![("url", remote.url.clone())];
        self.action(Action::ConnectRemote, subject, |engine| async move {
            engine
                .call(move |state| {
                    state.require_recovered()?;
                    let git = state
                        .git
                        .as_ref()
                        .ok_or_else(|| AppError::validation("Git is unavailable."))?;
                    let snapshot = git.connect(&state.library_dir(), &remote)?;
                    state.set_setting("remote", &Some(remote))?;
                    state.set_setting("sync", &snapshot)?;
                    Ok(snapshot)
                })
                .await
        })
        .await
    }
    /// Performs one explicitly requested Git operation.
    pub async fn synchronize(&self, action: SyncAction) -> AppResult<SyncSnapshot> {
        let logged = match action {
            SyncAction::Refresh => Action::Refresh,
            SyncAction::Pull => Action::Pull,
            SyncAction::Push => Action::Push,
            SyncAction::Sync => Action::Sync,
            SyncAction::Disconnect => Action::Disconnect,
        };
        self.action(logged, Vec::new(), |engine| async move {
            engine.call(move |state| state.synchronize(action)).await
        })
        .await
    }
    /// Renames this device; commits Push or Sync makes from now on carry the new name.
    pub async fn rename_device(&self, name: String) -> AppResult<()> {
        self.action(Action::RenameDevice, Vec::new(), |engine| async move {
            engine.call(move |state| state.rename_device(&name)).await
        })
        .await
    }
}

impl State {
    pub(super) fn synchronize(&mut self, action: SyncAction) -> AppResult<SyncSnapshot> {
        self.require_recovered()?;
        let remote: Option<RemoteConfig> = self.setting("remote")?.flatten();
        let git = self
            .git
            .as_ref()
            .ok_or_else(|| AppError::validation("Git is unavailable."))?;
        let repo = self.library_dir();
        let current = matches!(action, SyncAction::Push)
            && git.revision(&repo, "HEAD").ok()
                == Some(self.setting::<Option<String>>(RECONCILED_HEAD)?.flatten());
        let result = git.perform_with(&repo, remote.as_ref(), action, &Commits(self));
        // A Push only commits what the database already holds. A Pull or Sync, including one
        // that fast-forwarded before its push failed, reconciles what moved.
        let caught_up = match action {
            SyncAction::Push if current => git
                .revision(&repo, "HEAD")
                .and_then(|head| self.set_setting(RECONCILED_HEAD, &head)),
            SyncAction::Pull | SyncAction::Sync => {
                let reconciled = self.reconcile_head();
                // Pulled content rewrites library copies, even after a partial reconcile.
                self.apply_library();
                // One that fills an empty library adopts the skills your agents already have.
                self.adopt_agent_copies();
                reconciled
            }
            _ => Ok(()),
        };
        // The recorded commit stays behind on failure, so the next launch retries; the Git
        // result, success or failure, is what the user sees.
        if let Err(error) = caught_up {
            self.log.failure(
                Level::Warn,
                Area::Library,
                (
                    "reconciliation will retry",
                    "reconciliation stopped at quit",
                ),
                &error,
            );
        }
        let snapshot = match result {
            Ok(snapshot) => snapshot,
            // A check that fails shows as Unreachable until a later one, or an operation,
            // reaches the remote.
            Err(error) if matches!(action, SyncAction::Refresh) => {
                let mut checked: SyncSnapshot = self.setting("sync")?.unwrap_or_default();
                checked.state = SyncState::Unreachable;
                checked.last_error = Some(error.clone());
                self.set_setting("sync", &checked)?;
                return Err(error);
            }
            Err(error) => return Err(error),
        };
        if matches!(action, SyncAction::Disconnect) {
            self.set_setting("remote", &Option::<RemoteConfig>::None)?;
        }
        self.set_setting("sync", &snapshot)?;
        Ok(snapshot)
    }
}

/// How the library's commits name this device and say why each skill changed.
struct Commits<'a>(&'a State);

impl Committer for Commits<'_> {
    /// Pending records are committed only when nothing was pulled: written after a pull, they
    /// would replace pulled records with this device's older ones before reconciling.
    fn prepare(&self) -> AppResult<()> {
        self.0.write_pending_records()
    }
    fn device(&self) -> AppResult<String> {
        self.0.device_name()
    }
    fn reasons(&self) -> AppResult<BTreeMap<String, Reason>> {
        pending_changes::reasons(&mut self.0.database.borrow_mut())
    }
    fn committed(&self) -> AppResult<()> {
        pending_changes::clear(&mut self.0.database.borrow_mut())
    }
}
