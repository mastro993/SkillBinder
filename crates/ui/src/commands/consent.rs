//! What the command may answer for the user (spec rules 6 and 8): replacing copies a change left
//! waiting, after it landed, and confirming a deletion, before anything changes. In a terminal
//! the user answers; otherwise only `--replace` or `--yes` does, and the rest waits.

use super::Peer;
use crate::native_view::shown_dir;
use skillbinder_proto::command_line::{CommandError, ErrorCode, Permission};
use skillbinder_proto::{CopyState, ManagedCopy};

/// The copies a change left asking the user that did not ask before it.
pub(super) fn left_waiting(before: &[ManagedCopy], after: &[ManagedCopy]) -> Vec<ManagedCopy> {
    after
        .iter()
        .filter(|copy| copy.asks() && !before.contains(copy))
        .cloned()
        .collect()
}

/// The copies to replace among `waiting`, and those left waiting, as `replace` allows: in a
/// terminal those answered no stay, which the review list keeps asking about (C2.4), and without a
/// terminal or a flag, all of them. Asking holds nothing, so the user can take their time.
pub(super) async fn decide(
    waiting: Vec<ManagedCopy>,
    replace: Permission,
    peer: &Peer,
    home: &str,
) -> (Vec<ManagedCopy>, Vec<ManagedCopy>) {
    let mut chosen = (Vec::new(), Vec::new());
    for copy in waiting {
        let yes = match replace {
            Permission::Yes => true,
            Permission::No => false,
            Permission::Ask => peer.ask(replace_prompt(&copy, home)).await,
        };
        if yes {
            chosen.0.push(copy);
        } else {
            chosen.1.push(copy);
        }
    }
    chosen
}

/// Asks to go ahead with `prompt`, the window's confirmation text, as `confirm` allows.
pub(super) async fn confirm(
    prompt: String,
    confirm: Permission,
    peer: &Peer,
) -> Result<(), CommandError> {
    match confirm {
        Permission::Yes => Ok(()),
        Permission::No => {
            Err(CommandError::new(ErrorCode::NeedsAnswer, prompt).hint("Run again with --yes."))
        }
        Permission::Ask if peer.ask(prompt).await => Ok(()),
        Permission::Ask => Err(CommandError::new(
            ErrorCode::NeedsAnswer,
            "Nothing changed.",
        )),
    }
}

/// The failure of a change that left the `left` copies waiting (spec rule 6). Its hint names the
/// command that replaces a copy, which works after any change, where running the change again
/// would find nothing left to do.
pub(super) fn copies_wait(left: &[ManagedCopy], home: &str) -> CommandError {
    let (message, hint) = match left {
        [copy] => (
            "1 copy needs your answer".to_owned(),
            format!(
                "Run `skillbinder copies replace {} --in {}`, or review it in SkillBinder.",
                copy.skill,
                shown_dir(&copy.directory, home)
            ),
        ),
        left => (
            format!("{} copies need your answer", left.len()),
            "Run `skillbinder copies replace <slug> --in <directory>` for each, or review them \
             in SkillBinder."
                .to_owned(),
        ),
    };
    CommandError::new(ErrorCode::NeedsAnswer, message).hint(hint)
}

/// What a yes does to `copy`: an edited copy of a skill no longer active there goes, any other is
/// replaced.
fn replace_prompt(copy: &ManagedCopy, home: &str) -> String {
    let directory = shown_dir(&copy.directory, home);
    match copy.state {
        CopyState::Edited { active: false } => {
            format!(
                "Remove your edited copy of {} from {directory}?",
                copy.skill
            )
        }
        _ => format!("Replace your copy of {} in {directory}?", copy.skill),
    }
}

#[cfg(test)]
mod tests;
