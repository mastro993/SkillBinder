//! Sync from a terminal: `sync`, `push`, `pull`, and `remote set|remove`, as the Sync page's
//! buttons and its remote setup run them, then what waits, History, Restore, Discard, Resolve…,
//! and renaming this device. Each needs Git (spec rule 5), but for reading what waits and
//! renaming.

mod changes;
mod reads;

pub(super) use changes::{discard, rename_device, resolve_sync, restore};

use changes::{plan_discard, plan_resolve, plan_restore};
pub(super) use reads::{history, status};

use super::plan::Plan;
use super::run::{free, held, on_runtime};
use super::updates::needs_git;
use crate::native_view::skill_count;
use gpui_kit::AsyncApp;
use skillbinder_client::Client;
use skillbinder_proto::command_line::{Changed, CommandError, ErrorCode, SyncCommand, SyncKind};
use skillbinder_proto::{AppSnapshot, RemoteConfig, SyncAction, SyncState};

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

/// The plan for one of the Sync page's changes.
pub(super) fn plan_change(
    snapshot: &AppSnapshot,
    command: &SyncCommand,
) -> Result<Plan, CommandError> {
    match command {
        SyncCommand::Run { action } => plan(snapshot, *action),
        SyncCommand::SetRemote { url, branch } => plan_set(snapshot, url, branch),
        SyncCommand::RemoveRemote => plan_remove(snapshot),
        SyncCommand::Restore { skill, to } => plan_restore(snapshot, skill, to),
        SyncCommand::Discard { skill } => plan_discard(snapshot, skill),
        SyncCommand::Resolve { choices } => plan_resolve(snapshot, choices),
        SyncCommand::RenameDevice { name } => Ok(Plan::RenameDevice(name.clone())),
    }
}

/// `sync`, `push`, or `pull`, which need a remote. A Sync while skills changed on both sides
/// needs a choice for each, which `sync resolve` makes (Design 11).
fn plan(snapshot: &AppSnapshot, kind: SyncKind) -> Result<Plan, CommandError> {
    needs_git(snapshot)?;
    connected(snapshot)?;
    let sync = &snapshot.sync;
    if kind == SyncKind::Sync && sync.state == SyncState::NeedsResolve {
        return Err(CommandError::new(
            ErrorCode::Usage,
            format!(
                "{} changed on both sides. Resolve them with `skillbinder sync resolve`.",
                skill_count(sync.conflicts.len())
            ),
        ));
    }
    Ok(Plan::Sync(match kind {
        SyncKind::Sync => SyncAction::Sync,
        SyncKind::Push => SyncAction::Push,
        SyncKind::Pull => SyncAction::Pull,
    }))
}

/// Fails while no remote is set.
fn connected(snapshot: &AppSnapshot) -> Result<(), CommandError> {
    match snapshot.sync.remote {
        Some(_) => Ok(()),
        None => Err(
            CommandError::new(ErrorCode::Usage, "No sync remote is set.")
                .hint("Set one with `skillbinder remote set <url>`."),
        ),
    }
}

/// `remote set`, which replaces a remote already set.
fn plan_set(snapshot: &AppSnapshot, url: &str, branch: &str) -> Result<Plan, CommandError> {
    needs_git(snapshot)?;
    Ok(Plan::Connect(RemoteConfig {
        url: url.to_owned(),
        branch: branch.to_owned(),
    }))
}

/// `remote remove`: nothing to do with no remote set.
fn plan_remove(snapshot: &AppSnapshot) -> Result<Plan, CommandError> {
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

/// Connects the library to `remote`, as the Sync page's Connect does: it checks and
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
