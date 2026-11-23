use super::{confirm, copies_wait, left_waiting, settle};
use crate::commands::Peer;
use crate::native_view::fixture::id;
use skillbinder_proto::command_line::{ErrorCode, Permission, Question, Reply};
use skillbinder_proto::{AppResult, CopyState, ManagedCopy};
use std::cell::RefCell;
use std::future::Future;
use std::task::{Context, Poll, Waker};

/// Runs `future`, which never waits here: its answers are queued before it asks.
fn now<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("the future waits"),
    }
}

/// A command that answers `answers` in turn, and the questions it was asked.
fn command(answers: &[bool]) -> (Peer, async_channel::Receiver<Reply>) {
    let (replies, replied) = async_channel::unbounded();
    let (answer, received) = async_channel::unbounded();
    for yes in answers {
        let _ = answer.try_send(*yes);
    }
    // A command that answered everything it will, then went.
    drop(answer);
    (
        Peer {
            replies,
            answers: received,
        },
        replied,
    )
}

fn asked(replied: &async_channel::Receiver<Reply>) -> Vec<String> {
    std::iter::from_fn(|| replied.try_recv().ok())
        .filter_map(|reply| match reply {
            Reply::Ask(Question { prompt }) => Some(prompt),
            _ => None,
        })
        .collect()
}

fn copy(slug: &str, state: CopyState) -> ManagedCopy {
    ManagedCopy {
        skill: id(slug),
        directory: "/home/.claude/skills".into(),
        state,
    }
}

/// Settles `waiting` as `replace` allows, returning what still waits and what was replaced.
fn settled(
    waiting: Vec<ManagedCopy>,
    replace: Permission,
    peer: &Peer,
) -> (Vec<String>, Vec<String>) {
    let replaced = RefCell::new(Vec::new());
    let left = now(settle(waiting, replace, peer, "/home", |copy| {
        replaced.borrow_mut().push(copy.skill.to_string());
        async { AppResult::Ok(()) }
    }));
    let left = left.map(|left| left.iter().map(|copy| copy.skill.to_string()).collect());
    (left.unwrap_or_default(), replaced.into_inner())
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
    let error = copies_wait(1);
    assert_eq!(
        (error.code, error.code.exit_code()),
        (ErrorCode::NeedsAnswer, 3)
    );
    assert_eq!(error.message, "1 copy needs your answer");
    assert_eq!(
        error.hint.as_deref(),
        Some("Run again with --replace, or review it in SkillBinder.")
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
