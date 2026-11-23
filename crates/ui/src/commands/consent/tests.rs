use super::{confirm, copies_wait, decide, left_waiting};
use crate::commands::Peer;
use crate::commands::testing::{asked, command, now};
use crate::native_view::fixture::id;
use skillbinder_proto::command_line::{ErrorCode, Permission};
use skillbinder_proto::{CopyState, ManagedCopy};

fn copy(slug: &str, state: CopyState) -> ManagedCopy {
    ManagedCopy {
        skill: id(slug),
        directory: "/home/.claude/skills".into(),
        state,
    }
}

/// Decides on `waiting` as `replace` allows, returning what still waits and what is replaced.
fn settled(
    waiting: Vec<ManagedCopy>,
    replace: Permission,
    peer: &Peer,
) -> (Vec<String>, Vec<String>) {
    let (replaced, left) = now(decide(waiting, replace, peer, "/home"));
    let slugs =
        |copies: Vec<ManagedCopy>| copies.iter().map(|copy| copy.skill.to_string()).collect();
    (slugs(left), slugs(replaced))
}

#[test]
fn only_copies_the_change_left_asking_wait() {
    let before = [
        copy("sql", CopyState::Conflict),
        copy("pdf", CopyState::Applied),
    ];
    let after = [
        copy("sql", CopyState::Conflict),
        copy("pdf", CopyState::Applied),
        copy("docx", CopyState::Conflict),
        copy("xlsx", CopyState::NotApplied("No access.".into())),
    ];
    let waiting = left_waiting(&before, &after);
    assert_eq!(waiting, [copy("docx", CopyState::Conflict)]);
}

#[test]
fn without_a_terminal_or_flag_copies_wait_and_exit_3() {
    let (peer, replied) = command(&[]);
    let (left, replaced) = settled(
        vec![copy("pdf", CopyState::Conflict)],
        Permission::No,
        &peer,
    );
    assert_eq!((left, replaced), (vec!["pdf".to_owned()], Vec::new()));
    assert!(asked(&replied).is_empty(), "no prompt without a terminal");
    let error = copies_wait(&[copy("pdf", CopyState::Conflict)], "/home");
    assert_eq!(
        (error.code, error.code.exit_code()),
        (ErrorCode::NeedsAnswer, 3)
    );
    assert_eq!(error.message, "1 copy needs your answer");
    assert_eq!(
        error.hint.as_deref(),
        Some(
            "Run `skillbinder copies replace pdf --in ~/.claude/skills`, or review it in \
             SkillBinder."
        ),
        "a command any change can name, where running it again finds nothing to do"
    );
}

#[test]
fn replace_replaces_every_copy_without_asking() {
    let (peer, replied) = command(&[]);
    let waiting = vec![
        copy("pdf", CopyState::Conflict),
        copy("sql", CopyState::Edited { active: true }),
    ];
    let (left, replaced) = settled(waiting, Permission::Yes, &peer);
    assert!(left.is_empty());
    assert_eq!(replaced, ["pdf", "sql"]);
    assert!(asked(&replied).is_empty());
}

#[test]
fn in_a_terminal_each_copy_is_asked_and_no_keeps_it_waiting() {
    let (peer, replied) = command(&[false, true]);
    let waiting = vec![
        copy("pdf", CopyState::Conflict),
        copy("sql", CopyState::Edited { active: false }),
        copy("docx", CopyState::Conflict),
    ];
    let (left, replaced) = settled(waiting, Permission::Ask, &peer);
    assert_eq!(replaced, ["sql"]);
    assert_eq!(
        left,
        ["pdf", "docx"],
        "no, and a command gone, keep the copy waiting"
    );
    assert_eq!(
        asked(&replied),
        [
            "Replace your copy of pdf in ~/.claude/skills?",
            "Remove your edited copy of sql from ~/.claude/skills?",
            "Replace your copy of docx in ~/.claude/skills?",
        ]
    );
}

#[test]
fn a_deletion_needs_yes_without_a_terminal() {
    let prompt = || "Delete Work? Its 3 skills stay in the library.".to_owned();
    let (peer, replied) = command(&[]);
    let Err(error) = now(confirm(prompt(), Permission::No, &peer)) else {
        panic!("no --yes, no terminal");
    };
    assert_eq!(
        (error.code, error.code.exit_code()),
        (ErrorCode::NeedsAnswer, 3)
    );
    assert_eq!(error.message, prompt());
    assert_eq!(error.hint.as_deref(), Some("Run again with --yes."));
    assert!(now(confirm(prompt(), Permission::Yes, &peer)).is_ok());
    assert!(asked(&replied).is_empty());

    let (peer, replied) = command(&[false]);
    let Err(declined) = now(confirm(prompt(), Permission::Ask, &peer)) else {
        panic!("the user said no");
    };
    assert_eq!(declined.message, "Nothing changed.");
    assert_eq!(asked(&replied), [prompt()]);
    let (peer, _replied) = command(&[true]);
    assert!(now(confirm(prompt(), Permission::Ask, &peer)).is_ok());
}
