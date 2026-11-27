use super::super::output;
use crate::render::result;
use skillbinder_proto::command_line::{
    ChoiceRow, ConflictRow, ConflictVersion, HistoryOutput, HistoryRow, Output, SyncSkillRow,
    SyncStatusOutput,
};
use skillbinder_proto::{ChangeKind, Side, SyncState};

fn skill(slug: &str, kind: ChangeKind, why: Option<&str>) -> SyncSkillRow {
    SyncSkillRow {
        slug: slug.into(),
        kind,
        why: why.map(str::to_owned),
    }
}

fn waiting() -> SyncStatusOutput {
    SyncStatusOutput {
        state: SyncState::NeedsResolve,
        remote: Some("git@github.com:me/skills.git".into()),
        device: "MacBook".into(),
        local: vec![
            skill("sql", ChangeKind::Added, Some("Installed from Explore")),
            skill("pdf", ChangeKind::Changed, Some("Updated from its source")),
        ],
        incoming: vec![skill("pdf", ChangeKind::Removed, None)],
        devices: vec!["Mac Studio".into()],
        conflicts: vec![ConflictRow {
            slug: "pdf".into(),
            mine: ConflictVersion {
                kind: ChangeKind::Changed,
                why: Some("Updated from its source".into()),
                device: None,
            },
            theirs: ConflictVersion {
                kind: ChangeKind::Removed,
                why: Some("Removed outside SkillBinder".into()),
                device: Some("Mac Studio".into()),
            },
        }],
    }
}

#[test]
fn sync_status_prints_each_side_then_the_skills_changed_on_both() {
    assert_eq!(
        output(&Output::SyncStatus(Box::new(waiting()))),
        "Sync: skills changed on both sides\n\
         Remote: git@github.com:me/skills.git\n\
         This device: MacBook\n\
         Not pushed yet:\n  \
         + sql: Installed from Explore\n  \
         ~ pdf: Updated from its source\n\
         Not pulled yet, from Mac Studio:\n  \
         - pdf\n\
         Changed on both sides:\n  \
         pdf: mine ~ Updated from its source; theirs - Removed outside SkillBinder, on Mac Studio\n\
         Resolve them with `skillbinder sync resolve <slug>=mine|theirs…`.\n"
    );
    let synced = SyncStatusOutput {
        state: SyncState::Synced,
        local: Vec::new(),
        incoming: Vec::new(),
        devices: Vec::new(),
        conflicts: Vec::new(),
        ..waiting()
    };
    assert!(output(&Output::SyncStatus(Box::new(synced))).ends_with("Nothing waits to sync.\n"));
}

#[test]
fn sync_status_json_names_each_kind_and_side() {
    let value = result(&Output::SyncStatus(Box::new(waiting())));
    assert_eq!(value["state"], "needs_resolve");
    assert_eq!(value["local"][0]["kind"], "added");
    assert_eq!(value["conflicts"][0]["theirs"]["device"], "Mac Studio");
}

#[test]
fn history_prints_one_block_per_change_and_how_to_read_older_ones() {
    let history = HistoryOutput {
        changes: vec![
            HistoryRow {
                commit: "3f2a9c1b4d5e6f708192a3b4c5d6e7f809182736".into(),
                at: 1_763_000_000,
                when: "Today, 09:15".into(),
                device: Some("Mac Studio".into()),
                skills: vec![skill(
                    "pdf",
                    ChangeKind::Added,
                    Some("Imported from Claude Code"),
                )],
                choices: vec![ChoiceRow {
                    slug: "sql".into(),
                    side: Side::Remote,
                    kept: "Kept the remote's sql".into(),
                    unpicked: Some("aa11bb22cc33dd44ee55ff66aa77bb88cc99dd00".into()),
                }],
            },
            HistoryRow {
                commit: "0123456789ab".into(),
                at: 1_700_000_000,
                when: "Tue, Nov 14, 2023, 22:13".into(),
                device: None,
                skills: vec![skill("sql", ChangeKind::Changed, None)],
                choices: Vec::new(),
            },
        ],
        next: Some("0123456789abcdef".into()),
    };
    let output = Output::History(history);
    assert_eq!(
        super::super::output(&output),
        "3f2a9c1b4d5e  Today, 09:15  Mac Studio\n  \
         + pdf: Imported from Claude Code\n  \
         Kept the remote's sql. Restore the other with `skillbinder sync restore sql --to aa11bb22cc33`.\n\
         0123456789ab  Tue, Nov 14, 2023, 22:13  Before device names\n  \
         ~ sql\n\
         Older changes: `skillbinder sync history --before 0123456789ab`.\n"
    );
    assert_eq!(
        result(&output)["changes"][0]["commit"],
        "3f2a9c1b4d5e6f708192a3b4c5d6e7f809182736",
        "--json keeps the whole commit"
    );
    let empty = Output::History(HistoryOutput {
        changes: Vec::new(),
        next: None,
    });
    assert_eq!(super::super::output(&empty), "No changes yet.\n");
}
