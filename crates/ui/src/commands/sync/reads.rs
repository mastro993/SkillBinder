//! `sync status` and `sync history`: the Sync page's skills not synced yet, those changed on both
//! sides, and History, each skill with its kind and why in the page's words.

use super::super::updates::needs_git;
use crate::native_view::{HISTORY_PAGE, change_day, change_time, kept, why, why_changed};
use chrono::{DateTime, TimeZone};
use skillbinder_client::Client;
use skillbinder_proto::command_line::{
    ChoiceRow, CommandError, ConflictRow, ConflictVersion, ErrorCode, HistoryOutput, HistoryRow,
    Output, SyncSkillRow, SyncStatusOutput,
};
use skillbinder_proto::{AppSnapshot, ChangedSkill, ConflictSide, SyncChange};
use std::fmt::Display;

/// `sync status`: what waits on each side, as Not synced yet lists it, and the skills changed on
/// both sides.
pub(in crate::commands) fn status(snapshot: &AppSnapshot) -> SyncStatusOutput {
    let sync = &snapshot.sync;
    let incoming = sync.incoming.as_ref();
    let version = |side: &ConflictSide| ConflictVersion {
        kind: side.kind,
        why: why_changed(side.kind, side.reason.as_ref()),
        device: side.device.clone(),
    };
    SyncStatusOutput {
        state: sync.state,
        remote: sync.remote.as_ref().map(|remote| remote.url.clone()),
        device: sync.device.clone(),
        local: sync.local.iter().map(row).collect(),
        incoming: incoming
            .map(|incoming| incoming.skills.iter().map(row).collect())
            .unwrap_or_default(),
        devices: incoming
            .map(|incoming| incoming.devices.clone())
            .unwrap_or_default(),
        conflicts: sync
            .conflicts
            .iter()
            .map(|conflict| ConflictRow {
                slug: conflict.slug.clone(),
                mine: version(&conflict.local),
                theirs: version(&conflict.remote),
            })
            .collect(),
    }
}

/// `sync history`: a page of up to `limit` changes after `before`, or of the newest, as History
/// reads it 50 at a time (A3).
pub(in crate::commands) async fn history(
    client: &Client,
    before: Option<String>,
    limit: Option<usize>,
) -> Result<Output, CommandError> {
    let limit = page_size(&client.snapshot(), limit)?;
    let page = client.sync_history(before, limit).await?;
    let now = chrono::Local::now();
    Ok(Output::History(HistoryOutput {
        changes: page
            .changes
            .iter()
            .map(|change| history_row(change, &now))
            .collect(),
        next: page.next,
    }))
}

/// How many changes `sync history` reads: `limit`, at least 1, or History's 50. It needs Git.
pub(super) fn page_size(
    snapshot: &AppSnapshot,
    limit: Option<usize>,
) -> Result<usize, CommandError> {
    needs_git(snapshot)?;
    match limit.unwrap_or(HISTORY_PAGE) {
        0 => Err(CommandError::new(
            ErrorCode::Usage,
            "Ask for at least 1 change.",
        )),
        limit => Ok(limit),
    }
}

/// `change` as History lists it, its day and time in `now`'s time zone.
pub(super) fn history_row<Tz: TimeZone>(change: &SyncChange, now: &DateTime<Tz>) -> HistoryRow
where
    Tz::Offset: Display,
{
    HistoryRow {
        commit: change.commit.clone(),
        at: change.at,
        when: format!("{}, {}", change_day(change, now), change_time(change, now)),
        device: change.device.clone(),
        skills: change.skills.iter().map(row).collect(),
        choices: change
            .choices
            .iter()
            .map(|choice| ChoiceRow {
                slug: choice.slug.clone(),
                side: choice.side,
                kept: kept(choice.device.as_deref(), &choice.slug),
                unpicked: choice.unpicked.clone(),
            })
            .collect(),
    }
}

fn row(skill: &ChangedSkill) -> SyncSkillRow {
    SyncSkillRow {
        slug: skill.slug.clone(),
        kind: skill.kind,
        why: why(skill),
    }
}
