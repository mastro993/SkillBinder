//! `copies replace` and `copies keep`: the copy a command answers for, resolved against the
//! copies waiting for the user's answer.

use super::Plan;
use crate::commands::resolve;
use crate::native_view::shown_dir;
use skillbinder_proto::command_line::{CommandError, ErrorCode};
use skillbinder_proto::{AppSnapshot, CopyChoice, CopyState, LibrarySnapshot, ManagedCopy};

/// Answering `choice` for the copy of `slug` waiting in `directory`, or in its one directory.
pub(super) fn copy(
    snapshot: &AppSnapshot,
    library: &LibrarySnapshot,
    slug: &str,
    directory: Option<&str>,
    choice: CopyChoice,
) -> Result<Plan, CommandError> {
    let skill = &resolve::skill(library, slug)?.id;
    let answerable = |copy: &&ManagedCopy| match choice {
        // A copy the user kept can still be replaced, as its skill page offers.
        CopyChoice::Replace => copy.asks() || copy.state == CopyState::Skipped,
        CopyChoice::Keep => copy.asks(),
    };
    let waiting: Vec<&ManagedCopy> = library
        .copies
        .iter()
        .filter(|copy| &copy.skill == skill)
        .filter(answerable)
        .collect();
    let home = &snapshot.home;
    let copy = match (directory, waiting.as_slice()) {
        (Some(directory), _) => resolve::one(
            "directory",
            directory,
            waiting.iter().copied(),
            |copy| shown_dir(&copy.directory, home),
            |copy| vec![copy.directory.clone(), shown_dir(&copy.directory, home)],
        )
        .map_err(|error| CommandError {
            message: format!("No copy of {skill} waits in {directory}."),
            ..error
        })?,
        (None, [copy]) => copy,
        (None, []) => {
            return Err(CommandError::new(
                ErrorCode::NotFound,
                format!("No copy of {skill} waits for your answer."),
            ));
        }
        (None, several) => {
            let directories: Vec<String> = several
                .iter()
                .map(|copy| shown_dir(&copy.directory, home))
                .collect();
            return Err(CommandError::new(
                ErrorCode::Ambiguous,
                format!("{skill} waits in {}.", directories.join(", ")),
            )
            .hint("Name one with --in DIR."));
        }
    };
    let directory = shown_dir(&copy.directory, home);
    let done = match (choice, &copy.state) {
        (CopyChoice::Replace, CopyState::Edited { active: false }) => {
            format!("Removed your copy of {skill} from {directory}.")
        }
        (CopyChoice::Replace, _) => format!("Replaced your copy of {skill} in {directory}."),
        (CopyChoice::Keep, _) => format!("Kept your copy of {skill} in {directory}."),
    };
    Ok(Plan::Copy {
        copy: copy.clone(),
        choice,
        done,
    })
}
