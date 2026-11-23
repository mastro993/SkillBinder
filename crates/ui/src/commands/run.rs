//! Running a planned change as the window would (spec rule 1): a switch through the window's
//! strip while one is open, otherwise behind the gate as the menu runs it; everything else
//! through the client. Its progress goes to the command while it runs, and the copies it leaves
//! waiting are settled once it lands. The work never depends on the connection, so a command
//! that stops waiting leaves it running (rule 3).

use super::consent::{confirm, copies_wait, decide, left_waiting};
use super::plan::Plan;
use super::reads::{copy_row, running};
use super::{Peer, resolve};
use crate::gate::{self, Gate};
use crate::native_view::{Activity, Landing, switch_note};
use gpui_kit::{AsyncApp, Task};
use skillbinder_client::Client;
use skillbinder_proto::command_line::{
    Changed, CommandError, Consent, ErrorCode, Failure, Output, Progress,
};
use skillbinder_proto::{
    AppError, AppResult, AppSnapshot, ManagedCopy, OrganizationChange, PresetId, PresetSwitch,
};
use std::future::Future;

/// Runs `plan` with `consent`, telling `peer` how it goes.
pub(super) async fn run(
    plan: Plan,
    consent: Consent,
    peer: &Peer,
    client: &Client,
    cx: &mut AsyncApp,
) -> Result<Output, Failure> {
    let before = copies(&client.snapshot()).to_vec();
    let progress = forward_progress(title(&plan), peer, client, cx);
    let changed = apply(plan, consent, peer, client, cx).await;
    drop(progress);
    let mut changed = changed?;
    let after = client.snapshot();
    let home = after.home.clone();
    let waiting = left_waiting(&before, copies(&after));
    let (mut pending, mut left) = decide(waiting, consent.replace, peer, &home).await;
    let replaced = replace_copies(&mut pending, client, cx).await;
    // A copy not replaced, refused or failed, still waits beside those answered no.
    pending.append(&mut left);
    changed.waiting = pending.iter().map(|copy| copy_row(copy, &home)).collect();
    let error = match replaced {
        Err(error) => error,
        Ok(()) if pending.is_empty() => return Ok(Output::Changed(changed)),
        Ok(()) => copies_wait(&pending, &home),
    };
    // The change landed, so what it did comes back beside why it stopped.
    Err(Failure {
        error,
        result: Some(Box::new(Output::Changed(changed))),
    })
}

/// Replaces each of `pending`, removing it once replaced. A question may have waited on the user
/// meanwhile, so it is refused when something else started since, as the window would refuse it.
/// The command's own change landing behind a strip that still shows does not count.
async fn replace_copies(
    pending: &mut Vec<ManagedCopy>,
    client: &Client,
    cx: &mut AsyncApp,
) -> Result<(), CommandError> {
    if pending.is_empty() {
        return Ok(());
    }
    let snapshot = client.snapshot();
    if let Some(refused) = cx.update(|cx| interrupted(&Gate::of(cx), &snapshot)) {
        return Err(refused);
    }
    let replaced = async {
        while let Some(copy) = pending.first().cloned() {
            on_runtime(client, move |client| async move {
                client
                    .resolve_copy(&copy, skillbinder_proto::CopyChoice::Replace)
                    .await
            })
            .await?;
            pending.remove(0);
        }
        AppResult::Ok(())
    };
    held(cx, replaced).await.map_err(CommandError::from)
}

async fn apply(
    plan: Plan,
    consent: Consent,
    peer: &Peer,
    client: &Client,
    cx: &mut AsyncApp,
) -> Result<Changed, CommandError> {
    match plan {
        Plan::Organize { changes, done } => {
            if !changes.is_empty() {
                free(client, cx)?;
                let saved = async {
                    for change in changes {
                        organize(client, change).await?;
                    }
                    AppResult::Ok(())
                };
                held(cx, saved).await?;
            }
            Ok(Changed::said(done))
        }
        Plan::Switch { id, name, switch } => {
            free(client, cx)?;
            let activity = Activity::Switching {
                preset: name.clone(),
            };
            let change = OrganizationChange::ActivatePreset { id: id.clone() };
            blocking(client, activity, Some(id), change, cx).await?;
            Ok(switched(format!("{name} is active"), name, switch))
        }
        Plan::DeletePreset { id, name } => {
            let preview = on_runtime(client, move |client| async move {
                client.preview_delete(id).await
            })
            .await?;
            // The window's confirmation.
            let mut prompt = format!(
                "Delete {name}? Its {} skills stay in the library.",
                preview.affected_skills
            );
            if let Some(switch) = &preview.switch {
                prompt = format!("{prompt} {}", switch_note(switch));
            }
            confirm(prompt, consent.confirm, peer).await?;
            free(client, cx)?;
            let change = OrganizationChange::DeletePreset {
                id: preview.preset.id.clone(),
                revision: preview.revision.clone(),
            };
            let Some(switch) = preview.switch else {
                held(cx, organize(client, change)).await?;
                return Ok(Changed::said(format!("Deleted {name}.")));
            };
            let activity = Activity::DeletingActive {
                preset: name.clone(),
            };
            blocking(client, activity, None, change, cx).await?;
            let default = resolve::library(&client.snapshot())
                .and_then(resolve::active)
                .map_or_else(|_| "Default".to_owned(), |preset| preset.name.clone());
            let message = format!("Deleted {name}. {default} is active");
            Ok(switched(message, default, switch))
        }
        Plan::DeleteFolder {
            change,
            name,
            prompt,
        } => {
            confirm(prompt, consent.confirm, peer).await?;
            free(client, cx)?;
            held(cx, organize(client, change)).await?;
            Ok(Changed::said(format!("Deleted folder {name}.")))
        }
        Plan::Copy { copy, choice, done } => {
            free(client, cx)?;
            let resolved = on_runtime(client, move |client| async move {
                client.resolve_copy(&copy, choice).await
            });
            held(cx, resolved).await?;
            Ok(Changed::said(done))
        }
        Plan::RetryCopies => {
            free(client, cx)?;
            let retried = on_runtime(client, |client| async move { client.retry_copies().await });
            held(cx, retried).await?;
            Ok(Changed::said("Tried every unwritten copy again."))
        }
    }
}

/// `<message>: <n> added, <n> removed.` for a switch to `activated`.
fn switched(message: String, activated: String, switch: PresetSwitch) -> Changed {
    let names = |skills: Vec<skillbinder_proto::SkillId>| -> Vec<String> {
        skills.into_iter().map(|skill| skill.to_string()).collect()
    };
    let (added, removed) = (names(switch.adds), names(switch.removes));
    Changed {
        message: format!(
            "{message}: {} added, {} removed.",
            added.len(),
            removed.len()
        ),
        activated: Some(activated),
        added,
        removed,
        waiting: Vec::new(),
    }
}

/// Refuses at once while something the window would refuse runs (spec rule 2).
pub(super) fn refusal(gate: &Gate, snapshot: &AppSnapshot) -> Option<CommandError> {
    gate.blocked(snapshot).then(|| busy(gate, snapshot))
}

/// Refuses once something started after the command's change landed: another change, the
/// window's save, another command's, or an Update all.
pub(super) fn interrupted(gate: &Gate, snapshot: &AppSnapshot) -> Option<CommandError> {
    let started =
        gate.running().is_some() || gate.busy || gate.commands > 0 || snapshot.updating.is_some();
    started.then(|| busy(gate, snapshot))
}

/// "SkillBinder is busy", naming what blocks.
fn busy(gate: &Gate, snapshot: &AppSnapshot) -> CommandError {
    // A change whose strip still shows refuses as it does while it runs.
    let blocking = gate.change.as_ref().map(|change| change.activity.title());
    let message = match blocking.or_else(|| running(gate, snapshot)) {
        Some(what) => format!("SkillBinder is busy: {what}…"),
        None => "SkillBinder is busy.".to_owned(),
    };
    CommandError::new(ErrorCode::Busy, message).hint("Try again when it finishes.")
}

/// Checks again right before changing anything, since a question may have waited on the user.
fn free(client: &Client, cx: &mut AsyncApp) -> Result<(), CommandError> {
    let snapshot = client.snapshot();
    match cx.update(|cx| refusal(&Gate::of(cx), &snapshot)) {
        Some(refusal) => Err(refusal),
        None => Ok(()),
    }
}

/// Runs `work`, a short change of the command's own, refusing other changes meanwhile as the
/// window does during its saves.
async fn held<T>(cx: &mut AsyncApp, work: impl Future<Output = T>) -> T {
    cx.update(|cx| gate::command_change(true, cx));
    let result = work.await;
    cx.update(|cx| gate::command_change(false, cx));
    result
}

/// Runs `change`, a blocking one, as the window or the menu does, until it returns.
async fn blocking(
    client: &Client,
    activity: Activity,
    preset: Option<PresetId>,
    change: OrganizationChange,
    cx: &mut AsyncApp,
) -> Result<(), CommandError> {
    let (report, reported) = async_channel::bounded(1);
    let operation = {
        let client = client.clone();
        async move {
            let result = client
                .change_organization(change)
                .await
                .map(|_| Landing::default());
            let _ = report.try_send(result.clone());
            result
        }
    };
    let started = client.clone();
    cx.update(|cx| gate::block(&started, activity, preset, operation, cx));
    match reported.recv().await {
        Ok(result) => result.map(|_| ()).map_err(CommandError::from),
        // The window or the gate refused to start it, dropping it.
        Err(_) => Err(CommandError::new(ErrorCode::Busy, "SkillBinder is busy.")
            .hint("Try again when it finishes.")),
    }
}

async fn organize(client: &Client, change: OrganizationChange) -> AppResult<()> {
    on_runtime(client, move |client| async move {
        client.change_organization(change).await.map(|_| ())
    })
    .await
}

/// Runs a client operation on its runtime, as the window does.
async fn on_runtime<T, F>(client: &Client, operation: impl FnOnce(Client) -> F) -> AppResult<T>
where
    T: Send + 'static,
    F: Future<Output = AppResult<T>> + Send + 'static,
{
    client
        .runtime()
        .spawn(operation(client.clone()))
        .await
        .unwrap_or_else(|_| Err(AppError::storage()))
}

/// Sends how far the change has got whenever it moves, until the returned task drops.
fn forward_progress(activity: String, peer: &Peer, client: &Client, cx: &mut AsyncApp) -> Task<()> {
    let mut snapshots = client.subscribe();
    let peer = peer.clone();
    cx.spawn(async move |_| {
        let mut sent = None;
        while snapshots.changed().await.is_ok() {
            let Some((done, total)) = snapshots
                .borrow_and_update()
                .activation
                .as_ref()
                .map(|progress| (progress.done, progress.total))
            else {
                continue;
            };
            // Nothing planned yet is no progress to show.
            if total > 0 && sent != Some((done, total)) {
                sent = Some((done, total));
                peer.progress(Progress {
                    activity: activity.clone(),
                    done,
                    total,
                });
            }
        }
    })
}

/// What the command's progress line names.
fn title(plan: &Plan) -> String {
    match plan {
        Plan::Switch { name, .. } => Activity::Switching {
            preset: name.clone(),
        }
        .title(),
        Plan::DeletePreset { .. } => "Switching to Default".to_owned(),
        _ => "Writing copies".to_owned(),
    }
}

fn copies(snapshot: &AppSnapshot) -> &[ManagedCopy] {
    snapshot
        .library
        .as_ref()
        .map_or(&[], |library| library.copies.as_slice())
}
