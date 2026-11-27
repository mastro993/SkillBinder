//! `sync status` and `sync history` as the Sync page words them: each skill's kind as `+`, `~`,
//! or `-`, then why it changed.

use super::state;
use skillbinder_proto::ChangeKind;
use skillbinder_proto::command_line::{
    ConflictRow, ConflictVersion, HistoryOutput, HistoryRow, SyncSkillRow, SyncStatusOutput,
};

/// How many characters of a commit's id text shows; `--to` and `--before` take it.
const SHORT: usize = 12;

/// The state, this device's name, then each group that holds a skill.
pub(super) fn status(status: &SyncStatusOutput) -> String {
    let SyncStatusOutput {
        state: sync,
        remote,
        device,
        local,
        incoming,
        devices,
        conflicts,
    } = status;
    let mut text = format!("Sync: {}\n", state(*sync));
    if let Some(remote) = remote {
        text.push_str(&format!("Remote: {remote}\n"));
    }
    text.push_str(&format!("This device: {device}\n"));
    if !local.is_empty() {
        text.push_str("Not pushed yet:\n");
        text.extend(local.iter().map(skill_line));
    }
    if !incoming.is_empty() {
        match devices.as_slice() {
            [] => text.push_str("Not pulled yet:\n"),
            devices => text.push_str(&format!("Not pulled yet, from {}:\n", devices.join(", "))),
        }
        text.extend(incoming.iter().map(skill_line));
    }
    if !conflicts.is_empty() {
        text.push_str("Changed on both sides:\n");
        text.extend(conflicts.iter().map(conflict_line));
        text.push_str("Resolve them with `skillbinder sync resolve <slug>=mine|theirs…`.\n");
    }
    if remote.is_some() && local.is_empty() && incoming.is_empty() {
        text.push_str("Nothing waits to sync.\n");
    }
    text
}

/// One block per change, newest first, then how to list older ones.
pub(super) fn history(history: &HistoryOutput) -> String {
    let HistoryOutput { changes, next } = history;
    if changes.is_empty() {
        return "No changes yet.\n".to_owned();
    }
    let mut text: String = changes.iter().map(change_block).collect();
    if let Some(next) = next {
        text.push_str(&format!(
            "Older changes: `skillbinder sync history --before {}`.\n",
            short(next)
        ));
    }
    text
}

/// `3f2a9c1b4d5e  Today, 09:15  Mac Studio`, its skills, then a merge's choices.
fn change_block(change: &HistoryRow) -> String {
    let HistoryRow {
        commit,
        at: _,
        when,
        device,
        skills,
        choices,
    } = change;
    let device = device.as_deref().unwrap_or("Before device names");
    let mut text = format!("{}  {when}  {device}\n", short(commit));
    text.extend(skills.iter().map(skill_line));
    for choice in choices {
        text.push_str(&format!("  {}", choice.kept));
        if let Some(unpicked) = &choice.unpicked {
            text.push_str(&format!(
                ". Restore the other with `skillbinder sync restore {} --to {}`",
                choice.slug,
                short(unpicked)
            ));
        }
        text.push_str(".\n");
    }
    text
}

/// `  + pdf: Imported from Claude Code`, or without why for a change made before device names.
fn skill_line(skill: &SyncSkillRow) -> String {
    match &skill.why {
        Some(why) => format!("  {} {}: {why}\n", mark(skill.kind), skill.slug),
        None => format!("  {} {}\n", mark(skill.kind), skill.slug),
    }
}

/// `  pdf: mine ~ Updated from its source; theirs ~ Installed from Explore, on Mac Studio`.
fn conflict_line(conflict: &ConflictRow) -> String {
    let side = |version: &ConflictVersion| {
        let mut text = mark(version.kind).to_owned();
        if let Some(why) = &version.why {
            text.push_str(&format!(" {why}"));
        }
        if let Some(device) = &version.device {
            text.push_str(&format!(", on {device}"));
        }
        text
    };
    format!(
        "  {}: mine {}; theirs {}\n",
        conflict.slug,
        side(&conflict.mine),
        side(&conflict.theirs)
    )
}

fn mark(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Added => "+",
        ChangeKind::Changed => "~",
        ChangeKind::Removed => "-",
    }
}

fn short(commit: &str) -> &str {
    commit.get(..SHORT).unwrap_or(commit)
}

#[cfg(test)]
mod tests;
