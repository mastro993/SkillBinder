//! Sync from a terminal: `sync`, `push`, `pull`, and `remote set|remove`, as the Sync page's
//! buttons and its remote setup run them. Each needs Git (spec rule 5).

use super::plan::Plan;
use super::run::{free, held, on_runtime};
use super::updates::needs_git;
use gpui_kit::AsyncApp;
use skillbinder_client::Client;
use skillbinder_proto::command_line::{Changed, CommandError, ErrorCode, SyncKind};
use skillbinder_proto::{AppSnapshot, RemoteConfig, SyncAction};

/// What the command names while `action` runs, such as `SkillBinder is busy: Pushing…`.
pub(super) fn title(action: SyncAction) -> &'static str {
    match action {
        SyncAction::Refresh => "Checking the sync remote",
        SyncAction::Pull => "Pulling",
        SyncAction::Push => "Pushing",
        SyncAction::Sync => "Syncing",
        SyncAction::Disconnect => "Disconnecting the sync remote",
    }
}

/// `sync`, `push`, or `pull`, which need a remote.
pub(super) fn plan(snapshot: &AppSnapshot, kind: SyncKind) -> Result<Plan, CommandError> {
    needs_git(snapshot)?;
    if snapshot.sync.remote.is_none() {
        return Err(
            CommandError::new(ErrorCode::Usage, "No sync remote is set.")
                .hint("Set one with `skillbinder remote set <url>`."),
        );
    }
    Ok(Plan::Sync(match kind {
        SyncKind::Sync => SyncAction::Sync,
        SyncKind::Push => SyncAction::Push,
        SyncKind::Pull => SyncAction::Pull,
    }))
}

/// `remote set`, which replaces a remote already set.
pub(super) fn plan_set(
    snapshot: &AppSnapshot,
    url: &str,
    branch: &str,
) -> Result<Plan, CommandError> {
    needs_git(snapshot)?;
    Ok(Plan::Connect(RemoteConfig {
        url: url.to_owned(),
        branch: branch.to_owned(),
    }))
}

/// `remote remove`: nothing to do with no remote set.
pub(super) fn plan_remove(snapshot: &AppSnapshot) -> Result<Plan, CommandError> {
    needs_git(snapshot)?;
    Ok(match snapshot.sync.remote {
        Some(_) => Plan::Sync(SyncAction::Disconnect),
        None => Plan::Save {
            saves: Vec::new(),
            done: "No sync remote is set.".to_owned(),
        },
    })
}

/// Runs `action` as the Sync page's button does, refusing other changes meanwhile; the client
/// shows it as syncing until it returns.
pub(super) async fn synchronize(
    action: SyncAction,
    client: &Client,
    cx: &mut AsyncApp,
) -> Result<Changed, CommandError> {
    free(client, cx)?;
    let remote = client.snapshot().sync.remote.clone();
    let synchronized = on_runtime(client, move |client| async move {
        client.synchronize(action).await
    });
    held(cx, synchronized).await?;
    let url = remote.map_or_else(|| "the remote".to_owned(), |remote| remote.url);
    Ok(Changed::said(done(action, &url)))
}

/// What `action` did with the remote at `url`.
fn done(action: SyncAction, url: &str) -> String {
    match action {
        SyncAction::Sync => format!("Synced the library with {url}."),
        SyncAction::Push => format!("Pushed the library to {url}."),
        SyncAction::Pull => format!("Pulled the library from {url}."),
        SyncAction::Disconnect => format!("Disconnected {url}. The library stays as it is."),
        SyncAction::Refresh => format!("Checked {url}."),
    }
}

/// Connects the library to `remote`, as the Sync page's Connect remote does: it checks and
/// fetches the remote, committing nothing.
pub(super) async fn connect(
    remote: RemoteConfig,
    client: &Client,
    cx: &mut AsyncApp,
) -> Result<Changed, CommandError> {
    free(client, cx)?;
    let done = format!(
        "Connected the library to {} ({}).",
        remote.url, remote.branch
    );
    let connected = on_runtime(client, move |client| async move {
        client.connect_remote(remote).await
    });
    held(cx, connected).await?;
    Ok(Changed::said(done))
}

#[cfg(test)]
mod tests;
