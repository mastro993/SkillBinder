//! `sync restore`, `sync discard`, `sync resolve`, and `device rename`, as History's Restore,
//! Not synced yet's Discard, Resolve…, and Rename this device run them. All but renaming need
//! Git (spec rule 5).

use super::super::plan::Plan;
use super::super::resolve;
use super::super::run::{free, held, on_runtime};
use super::super::updates::needs_git;
use super::{connected, done};
use gpui_kit::AsyncApp;
use skillbinder_client::Client;
use skillbinder_proto::command_line::{Changed, CommandError, ErrorCode};
use skillbinder_proto::{AppSnapshot, Side, SkillId, SyncAction};
use std::collections::BTreeMap;

/// `sync restore`: the engine refuses a commit that holds no version of the skill.
pub(super) fn plan_restore(
    snapshot: &AppSnapshot,
    skill: &str,
    to: &str,
) -> Result<Plan, CommandError> {
    needs_git(snapshot)?;
    Ok(Plan::Restore {
        skill: SkillId(skill.trim().to_owned()),
        commit: to.trim().to_owned(),
    })
}

/// `sync discard`: a skill Not synced yet lists, as only its rows offer Discard.
pub(super) fn plan_discard(snapshot: &AppSnapshot, skill: &str) -> Result<Plan, CommandError> {
    needs_git(snapshot)?;
    let local = &snapshot.sync.local;
    let slug = |row: &skillbinder_proto::ChangedSkill| row.slug.clone();
    let changed =
        resolve::one("change", skill, local, slug, |row| vec![slug(row)]).map_err(|error| {
            CommandError {
                message: format!("No change to {skill} waits to sync."),
                ..error
            }
        })?;
    Ok(Plan::Discard(SkillId(changed.slug.clone())))
}

/// `sync resolve`: a side for exactly the skills changed on both sides, by their slugs.
pub(super) fn plan_resolve(
    snapshot: &AppSnapshot,
    choices: &BTreeMap<String, Side>,
) -> Result<Plan, CommandError> {
    needs_git(snapshot)?;
    connected(snapshot)?;
    let conflicts = &snapshot.sync.conflicts;
    if conflicts.is_empty() {
        return Err(
            CommandError::new(ErrorCode::Usage, "No skill changed on both sides.")
                .hint("Sync with `skillbinder sync`."),
        );
    }
    let mut chosen = BTreeMap::new();
    for (skill, side) in choices {
        let conflict = resolve::one(
            "skill changed on both sides",
            skill,
            conflicts,
            |conflict| conflict.slug.clone(),
            |conflict| vec![conflict.slug.clone()],
        )?;
        chosen.insert(conflict.slug.clone(), *side);
    }
    let missing: Vec<&str> = conflicts
        .iter()
        .map(|conflict| conflict.slug.as_str())
        .filter(|slug| !chosen.contains_key(*slug))
        .collect();
    if !missing.is_empty() {
        return Err(CommandError::new(
            ErrorCode::Usage,
            format!("Choose a version of {} too.", missing.join(", ")),
        )
        .hint("Name each as <slug>=mine or <slug>=theirs."));
    }
    Ok(Plan::Resolve(chosen))
}

/// Puts `skill` back to its version in `commit`, as History's Restore does.
pub(in crate::commands) async fn restore(
    skill: SkillId,
    commit: String,
    client: &Client,
    cx: &mut AsyncApp,
) -> Result<Changed, CommandError> {
    free(client, cx)?;
    let done = format!("Restored {skill}. Push to share it.");
    let restored = on_runtime(client, move |client| async move {
        client.restore_skill(skill, commit).await
    });
    held(cx, restored).await?;
    Ok(Changed::said(done))
}

/// Undoes this device's change to `skill`, as Discard does.
pub(in crate::commands) async fn discard(
    skill: SkillId,
    client: &Client,
    cx: &mut AsyncApp,
) -> Result<Changed, CommandError> {
    free(client, cx)?;
    let done = format!("Discarded the change to {skill}.");
    let discarded = on_runtime(client, move |client| async move {
        client.discard_change(skill).await
    });
    held(cx, discarded).await?;
    Ok(Changed::said(done))
}

/// Syncs keeping the side `chosen` for each skill changed on both sides, as Resolve…'s Sync
/// does; the client shows it as syncing until it returns.
pub(in crate::commands) async fn resolve_sync(
    chosen: BTreeMap<String, Side>,
    client: &Client,
    cx: &mut AsyncApp,
) -> Result<Changed, CommandError> {
    free(client, cx)?;
    let remote = client.snapshot().sync.remote.clone();
    let resolved = on_runtime(client, move |client| async move {
        client.resolve_sync(chosen).await
    });
    held(cx, resolved).await?;
    let url = remote.map_or_else(|| "the remote".to_owned(), |remote| remote.url);
    Ok(Changed::said(done(SyncAction::Sync, &url)))
}

/// Renames this device; the engine trims the name and refuses one it can't keep.
pub(in crate::commands) async fn rename_device(
    name: String,
    client: &Client,
    cx: &mut AsyncApp,
) -> Result<Changed, CommandError> {
    free(client, cx)?;
    let renamed = on_runtime(client, move |client| async move {
        client.rename_device(name).await
    });
    held(cx, renamed).await?;
    let device = client.snapshot().sync.device.clone();
    Ok(Changed::said(format!("This device is now named {device}.")))
}
