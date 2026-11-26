//! Running a planned change as the window would (spec rule 1): a switch through the window's
//! strip while one is open, otherwise behind the gate as the menu runs it; everything else
//! through the client. Its progress goes to the command while it runs, and the copies it leaves
//! waiting are settled once it lands. The work never depends on the connection, so a command
//! that stops waiting leaves it running (rule 3).

use super::consent::{confirm, copies_wait, decide, left_waiting};
use super::plan::{Plan, Save, Work};
use super::reads::{copy_row, running};
use super::{Peer, explore, import, repositories, resolve, sync, updates};
use crate::gate::{self, Gate};
use crate::native_view::{Activity, Landing, switch_note};
use gpui_kit::{AsyncApp, Task};
use skillbinder_client::Client;
use skillbinder_proto::command_line::{
    Changed, CommandError, Consent, ErrorCode, Failure, Output, Progress,
};
use skillbinder_proto::{
    AppError, AppResult, AppSnapshot, ErrorCategory, ManagedCopy, OrganizationChange, PresetId,
    PresetSwitch, RepositoryId, Workspace, owner_repo,
};
use std::future::Future;
use std::path::PathBuf;

/// Runs `plan` with `consent`, telling `peer` how it goes.
pub(super) async fn run(
    plan: Plan,
    consent: Consent,
    peer: &Peer,
    client: &Client,
    cx: &mut AsyncApp,
) -> Result<Output, Failure> {
    let before = copies(&client.snapshot()).to_vec();
    let progress = forward_progress(&plan, peer, client, cx);
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
        Ok(()) if pending.is_empty() => return Ok(Output::Changed(changed)),
        Ok(()) => copies_wait(&pending, &home),
        // Running the change again finds it done, so the hint names what still answers them.
        Err(error) => CommandError {
            hint: copies_wait(&pending, &home).hint,
            ..error
        },
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
) -> Result<Changed, Failure> {
    match plan {
        Plan::Save { saves, done } => {
            if !saves.is_empty() {
                free(client, cx)?;
                let saved = async {
                    for change in saves {
                        save(client, change).await?;
                    }
                    AppResult::Ok(())
                };
                held(cx, saved).await?;
            }
            Ok(Changed::said(done))
        }
        Plan::Block {
            activity,
            work,
            done,
        } => {
            free(client, cx)?;
            let worker = client.clone();
            let work = async move {
                match work {
                    Work::Add(change) => worker.add_skills(change).await,
                    Work::TurnOn(dirs) => worker.set_agents_chosen(dirs, true).await,
                }
                .map(|()| Landing::default())
            };
            blocking(client, activity, None, work, cx).await?;
            Ok(Changed::said(done))
        }
        Plan::Switch { id, name, switch } => {
            free(client, cx)?;
            let activity = Activity::Switching {
                preset: name.clone(),
            };
            let change = OrganizationChange::ActivatePreset { id: id.clone() };
            blocking(client, activity, Some(id), organizing(client, change), cx).await?;
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
                held(cx, save(client, Save::Organize(change))).await?;
                return Ok(Changed::said(format!("Deleted {name}.")));
            };
            let activity = Activity::DeletingActive {
                preset: name.clone(),
            };
            blocking(client, activity, None, organizing(client, change), cx).await?;
            let default = resolve::library(&client.snapshot())
                .and_then(resolve::active)
                .map_or_else(|_| "Default".to_owned(), |preset| preset.name.clone());
            let message = format!("Deleted {name}. {default} is active");
            Ok(switched(message, default, switch))
        }
        Plan::Confirm { prompt, save, done } => {
            confirm(prompt, consent.confirm, peer).await?;
            free(client, cx)?;
            held(cx, self::save(client, save)).await?;
            Ok(Changed::said(done))
        }
        Plan::AddWorkspace { folder } => {
            free(client, cx)?;
            let added = on_runtime(client, move |client| async move {
                let grant = client.grant_directory(PathBuf::from(folder)).await?;
                client.add_workspaces(grant.id).await
            });
            let added = held(cx, added).await?;
            Ok(Changed::said(added_message(&added)))
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
        Plan::Import { skills, all } => {
            Ok(import::import(skills, all, consent.replace, peer, client, cx).await?)
        }
        Plan::Install(skills) => explore::install(skills, client).await,
        Plan::AddRepository { url } => Ok(repositories::add_repository(url, client).await?),
        Plan::RemoveRepository(repository) => {
            Ok(repositories::remove_repository(repository, client).await?)
        }
        Plan::InstallAll(repository) => repositories::install_all(repository, client).await,
        Plan::Update { skills, current } => updates::update(skills, current, client, cx).await,
        Plan::UpdateAll => updates::update_all(client, cx).await,
        Plan::Reinstall(skill) => Ok(updates::reinstall(skill, client, cx).await?),
        Plan::Sync(action) => Ok(sync::synchronize(action, client, cx).await?),
        Plan::Connect(remote) => Ok(sync::connect(remote, client, cx).await?),
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
        ..Changed::default()
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
pub(super) fn busy(gate: &Gate, snapshot: &AppSnapshot) -> CommandError {
    // A change whose strip still shows refuses as it does while it runs.
    let blocking = gate.change.as_ref().map(|change| change.activity.title());
    let message = match blocking.or_else(|| running(gate, snapshot)) {
        Some(what) => format!("SkillBinder is busy: {what}…"),
        None => "SkillBinder is busy.".to_owned(),
    };
    CommandError::new(ErrorCode::Busy, message).hint("Try again when it finishes.")
}

/// Checks again right before changing anything, since a question may have waited on the user.
pub(super) fn free(client: &Client, cx: &mut AsyncApp) -> Result<(), CommandError> {
    let snapshot = client.snapshot();
    match cx.update(|cx| refusal(&Gate::of(cx), &snapshot)) {
        Some(refusal) => Err(refusal),
        None => Ok(()),
    }
}

/// Runs `work`, a short change of the command's own, refusing other changes meanwhile as the
/// window does during its saves.
pub(super) async fn held<T>(cx: &mut AsyncApp, work: impl Future<Output = T>) -> T {
    cx.update(|cx| gate::command_change(true, cx));
    let result = work.await;
    cx.update(|cx| gate::command_change(false, cx));
    result
}

/// Runs `work`, a blocking change, as the window or the menu does, until it returns.
pub(super) async fn blocking(
    client: &Client,
    activity: Activity,
    preset: Option<PresetId>,
    work: impl Future<Output = AppResult<Landing>> + Send + 'static,
    cx: &mut AsyncApp,
) -> Result<(), CommandError> {
    let (report, reported) = async_channel::bounded(1);
    let operation = async move {
        let result = work.await;
        let _ = report.try_send(result.clone());
        result
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

/// `change` as a blocking change's work.
fn organizing(
    client: &Client,
    change: OrganizationChange,
) -> impl Future<Output = AppResult<Landing>> + Send + 'static {
    let client = client.clone();
    async move {
        client
            .change_organization(change)
            .await
            .map(|_| Landing::default())
    }
}

async fn save(client: &Client, save: Save) -> AppResult<()> {
    on_runtime(client, move |client| async move {
        match save {
            Save::Organize(change) => client.change_organization(change).await.map(|_| ()),
            Save::RenameWorkspace { id, name } => client.rename_workspace(id, name).await,
            Save::RemoveWorkspace(id) => client.remove_workspace(id).await,
            Save::Deactivate { workspace, skill } => {
                client.set_workspace_skill(workspace, skill, None).await
            }
            Save::TurnOff(dirs) => client.set_agents_chosen(dirs, false).await,
            Save::Ignore { skills, ignored } => {
                client.set_ignored(skills, ignored).await.map_err(scan_hint)
            }
        }
    })
    .await
}

/// An unknown slug's error, pointing at `scan`, which lists the found and ignored skills.
fn scan_hint(error: AppError) -> AppError {
    match error.category {
        ErrorCategory::NotFound => AppError {
            recovery: "Run `skillbinder scan` to list the found and ignored skills.".to_owned(),
            ..error
        },
        _ => error,
    }
}

/// `Added workspace "invoices-app" (.agents/skills, .claude/skills).`, or the added workspaces'
/// names.
fn added_message(added: &[Workspace]) -> String {
    let quoted = |workspace: &Workspace| format!("\"{}\"", workspace.name);
    match added {
        [workspace] if workspace.dirs.is_empty() => {
            format!("Added workspace {}.", quoted(workspace))
        }
        [workspace] => {
            let agents: Vec<&str> = workspace.dirs.iter().map(String::as_str).collect();
            format!(
                "Added workspace {} ({}).",
                quoted(workspace),
                agents.join(", ")
            )
        }
        added => {
            let names: Vec<String> = added.iter().map(quoted).collect();
            format!("Added {} workspaces: {}.", added.len(), names.join(", "))
        }
    }
}

/// Runs a client operation on its runtime, as the window does.
pub(super) async fn on_runtime<T, F>(
    client: &Client,
    operation: impl FnOnce(Client) -> F,
) -> AppResult<T>
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

/// Sends how far `plan` has got whenever it moves, until the returned task drops: the copies a
/// change writes, the skills an Install all installs, or those an Update all updates.
fn forward_progress(plan: &Plan, peer: &Peer, client: &Client, cx: &mut AsyncApp) -> Task<()> {
    let activity = title(plan);
    let counted = match plan {
        Plan::InstallAll(repository) => Counted::Installing(repository.id.clone()),
        Plan::UpdateAll => Counted::Updating,
        _ => Counted::Copies,
    };
    let mut snapshots = client.subscribe();
    let peer = peer.clone();
    cx.spawn(async move |_| {
        let mut sent = None;
        while snapshots.changed().await.is_ok() {
            let Some((done, total)) = counted.of(&snapshots.borrow_and_update()) else {
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

/// What a change's progress counts.
enum Counted {
    /// The copies it writes or removes.
    Copies,
    /// The skills an Install all of this repository installs.
    Installing(RepositoryId),
    /// The skills an Update all updates.
    Updating,
}

impl Counted {
    /// How far it has got in `snapshot`, while it runs.
    fn of(&self, snapshot: &AppSnapshot) -> Option<(usize, usize)> {
        match self {
            Self::Copies => {
                let progress = snapshot.activation.as_ref()?;
                Some((progress.done, progress.total))
            }
            Self::Installing(id) => {
                let progress = snapshot.explore.install_all.get(id)?;
                Some((progress.done, progress.total))
            }
            Self::Updating => snapshot.updating,
        }
    }
}

/// What the command's progress line names.
fn title(plan: &Plan) -> String {
    match plan {
        Plan::Switch { name, .. } => Activity::Switching {
            preset: name.clone(),
        }
        .title(),
        Plan::DeletePreset { .. } => "Switching to Default".to_owned(),
        Plan::Block { activity, .. } => activity.title(),
        Plan::Import { .. } => "Importing skills".to_owned(),
        Plan::InstallAll(repository) => format!("Installing {}", owner_repo(&repository.url)),
        Plan::Update { .. } | Plan::UpdateAll => "Updating skills".to_owned(),
        Plan::Sync(action) => sync::title(*action).to_owned(),
        _ => "Writing copies".to_owned(),
    }
}

fn copies(snapshot: &AppSnapshot) -> &[ManagedCopy] {
    snapshot
        .library
        .as_ref()
        .map_or(&[], |library| library.copies.as_slice())
}
