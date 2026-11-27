use super::super::{grammar, request};
use skillbinder_proto::Side;
use skillbinder_proto::command_line::{
    Change, CommandError, ErrorCode, Find, Query, Request, SyncCommand, SyncKind,
};
use std::collections::BTreeMap;

fn parsed(line: &[&str]) -> Result<Request, CommandError> {
    let matches = grammar()
        .try_get_matches_from(std::iter::once("skillbinder").chain(line.iter().copied()))
        .unwrap_or_else(|error| panic!("{line:?}: {error}"));
    request(&matches, false)
}

fn change(line: &[&str]) -> SyncCommand {
    match parsed(line) {
        Ok(Request::Change {
            change: Change::Sync { sync },
            ..
        }) => sync,
        other => panic!("{line:?}: {other:?} changes"),
    }
}

fn usage(line: &[&str]) -> String {
    match parsed(line) {
        Err(error) if error.code == ErrorCode::Usage => error.message,
        other => panic!("{line:?}: {other:?} is a usage error"),
    }
}

#[test]
fn bare_sync_still_syncs_and_status_and_history_only_read() {
    assert_eq!(
        change(&["sync"]),
        SyncCommand::Run {
            action: SyncKind::Sync
        }
    );
    assert_eq!(
        parsed(&["sync", "status"]).ok(),
        Some(Request::Read {
            query: Query::SyncStatus
        })
    );
    assert_eq!(
        parsed(&["sync", "history"]).ok(),
        Some(Request::Find {
            find: Find::History {
                before: None,
                limit: None
            }
        })
    );
    assert_eq!(
        parsed(&["sync", "history", "--limit", "10", "--before", "3f2a9c1b"]).ok(),
        Some(Request::Find {
            find: Find::History {
                before: Some("3f2a9c1b".into()),
                limit: Some(10)
            }
        })
    );
    for limit in ["0", "ten"] {
        let refused =
            grammar().try_get_matches_from(["skillbinder", "sync", "history", "--limit", limit]);
        assert!(refused.is_err(), "--limit {limit} is refused");
    }
}

#[test]
fn restore_discard_and_rename_name_their_skill_or_name() {
    assert_eq!(
        change(&["sync", "restore", "pdf", "--to", "3f2a9c1b"]),
        SyncCommand::Restore {
            skill: "pdf".into(),
            to: "3f2a9c1b".into()
        }
    );
    assert!(
        grammar()
            .try_get_matches_from(["skillbinder", "sync", "restore", "pdf"])
            .is_err(),
        "restore needs --to"
    );
    assert_eq!(
        change(&["sync", "discard", "pdf"]),
        SyncCommand::Discard {
            skill: "pdf".into()
        }
    );
    assert_eq!(
        change(&["device", "rename", "Work laptop"]),
        SyncCommand::RenameDevice {
            name: "Work laptop".into()
        }
    );
    assert!(
        grammar()
            .try_get_matches_from(["skillbinder", "device", "rename", "Work", "laptop"])
            .is_err(),
        "a name with spaces is one quoted value"
    );
}

#[test]
fn resolve_reads_mine_as_this_device_and_theirs_as_the_remote() {
    assert_eq!(
        change(&["sync", "resolve", "pdf=mine", "sql=THEIRS"]),
        SyncCommand::Resolve {
            choices: BTreeMap::from([("pdf".into(), Side::Local), ("sql".into(), Side::Remote)])
        }
    );
    assert_eq!(
        usage(&["sync", "resolve", "pdf"]),
        "Write pdf as pdf=mine or pdf=theirs."
    );
    assert_eq!(
        usage(&["sync", "resolve", "pdf=ours"]),
        "Choose mine or theirs for pdf, as pdf=mine."
    );
    assert_eq!(
        usage(&["sync", "resolve", "pdf=mine", "pdf=theirs"]),
        "Choose one version of pdf."
    );
}
