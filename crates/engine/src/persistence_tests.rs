#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

fn fixture() -> (tempfile::TempDir, State, Journal) {
    let directory = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(directory.path()).unwrap();
    let state = State::isolated_test_state(root.clone());
    let before = Library {
        schema_version: 2,
        library_id: "original".into(),
        created_at: "today".into(),
        content_policy_version: 1,
        folders: vec![],
        tags: vec![],
        skills: vec![],
    };
    let mut after = before.clone();
    after.library_id = "applied".into();
    write_json(&state.library_dir().join(".skillbinder.json"), &before).unwrap();
    let moves = (0..2)
        .map(|index| {
            let from = root.join(format!("staging/source-{index}"));
            fs::create_dir(&from).unwrap();
            fs::write(
                from.join("SKILL.md"),
                format!("---\nname: skill-{index}\ndescription: test\n---\nbody-{index}\n"),
            )
            .unwrap();
            Move {
                from,
                to: root.join(format!("library/skills/skill-{index}")),
            }
        })
        .collect();
    let plan_id = skillbinder_proto::PlanId::new();
    state
        .database
        .execute(
            "INSERT INTO operation_plans(id,payload,expires_at) VALUES(?1,'{}',0)",
            [plan_id.as_str()],
        )
        .unwrap();
    let journal = Journal {
        id: "recovery-test".into(),
        before,
        after,
        moves,
        completed_moves: 0,
        progress: Default::default(),
        details: vec![(
            SkillId::new(),
            SkillDetails {
                description: "durable".into(),
                ..Default::default()
            },
        )],
        outcome: Some(ImportOutcome {
            plan_id,
            imported: vec![],
            attached: vec![],
            conflicts: vec![],
        }),
    };
    (directory, state, journal)
}
fn inject(points: &[(&'static str, bool)]) {
    FAULTS.with(|faults| *faults.borrow_mut() = points.iter().copied().collect());
}
fn assert_result(state: &State, applied: bool) {
    let library: Library =
        serde_json::from_slice(&fs::read(state.library_dir().join(".skillbinder.json")).unwrap())
            .unwrap();
    assert_eq!(
        library.library_id,
        if applied { "applied" } else { "original" }
    );
    for index in 0..2 {
        let from = state
            .config
            .data_dir
            .join(format!("staging/source-{index}"));
        let to = state
            .config
            .data_dir
            .join(format!("library/skills/skill-{index}"));
        assert_eq!(from.exists(), !applied);
        assert_eq!(to.exists(), applied);
        let actual = fs::read_to_string(if applied { to } else { from }.join("SKILL.md")).unwrap();
        assert!(actual.contains(&format!("body-{index}")));
    }
    let details: i64 = state
        .database
        .query_row("SELECT count(*) FROM skill_metadata", [], |row| row.get(0))
        .unwrap();
    assert_eq!(details, i64::from(applied));
    let outcomes: i64 = state
        .database
        .query_row("SELECT count(*) FROM idempotency_records", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(outcomes, i64::from(applied));
    let consumed: i64 = state
        .database
        .query_row("SELECT consumed FROM operation_plans", [], |row| row.get(0))
        .unwrap();
    assert_eq!(consumed, i64::from(applied));
    let outcome = state.setting::<ImportOutcome>("import_outcome").unwrap();
    assert_eq!(outcome.is_some(), applied);
    if let Some(outcome) = outcome {
        let stored: String = state
            .database
            .query_row(
                "SELECT result FROM idempotency_records WHERE operation_id=?1",
                [outcome.plan_id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(serde_json::to_string(&outcome).unwrap(), stored);
    }
    assert_eq!(
        fs::read_dir(state.config.data_dir.join("journals"))
            .unwrap()
            .count(),
        0
    );
    assert!(!state.recovery_required);
}
#[test]
fn crashes_at_apply_boundaries_roll_forward_after_reopen() {
    for point in [
        "before_rename",
        "after_rename",
        "before_progress",
        "after_progress",
        "before_metadata",
        "after_metadata",
        "before_database",
        "after_database",
    ] {
        let (_directory, mut state, journal) = fixture();
        inject(&[(point, true)]);
        assert!(
            catch_unwind(AssertUnwindSafe(|| state.commit(journal))).is_err(),
            "{point}"
        );
        let root = state.config.data_dir.clone();
        drop(state);
        let mut reopened = State::isolated_test_state(root);
        reopened.recover().unwrap();
        assert_result(&reopened, true);
        reopened.recover().unwrap();
        assert_result(&reopened, true);
    }
}
#[test]
fn failures_before_database_commit_restore_payload_and_metadata() {
    for point in [
        "before_rename",
        "after_rename",
        "before_progress",
        "after_progress",
        "before_metadata",
        "after_metadata",
        "before_database",
    ] {
        let (_directory, mut state, journal) = fixture();
        inject(&[(point, false)]);
        assert!(state.commit(journal).is_err(), "{point}");
        let root = state.config.data_dir.clone();
        drop(state);
        let mut reopened = State::isolated_test_state(root);
        reopened.recover().unwrap();
        assert_result(&reopened, false);
    }
}
#[test]
fn crash_during_rollback_resumes_rollback() {
    for point in [
        "before_rollback",
        "before_rename",
        "after_rename",
        "after_rollback",
    ] {
        let (_directory, mut state, journal) = fixture();
        inject(&[("after_metadata", false), (point, true)]);
        assert!(
            catch_unwind(AssertUnwindSafe(|| state.commit(journal))).is_err(),
            "{point}"
        );
        let root = state.config.data_dir.clone();
        drop(state);
        let mut reopened = State::isolated_test_state(root);
        reopened.recover().unwrap();
        assert_result(&reopened, false);
        reopened.recover().unwrap();
        assert_result(&reopened, false);
    }
}
#[test]
fn committed_database_is_never_rolled_back() {
    let (_directory, mut state, journal) = fixture();
    inject(&[("after_database", false)]);
    assert!(state.commit(journal).is_err());
    assert!(state.recovery_required);
    state.recover().unwrap();
    assert_result(&state, true);
}
#[test]
fn replacement_with_identical_bytes_cannot_impersonate_renamed_payload() {
    let (_directory, mut state, journal) = fixture();
    let destination = journal.moves[0].to.clone();
    inject(&[("after_rename", true)]);
    assert!(catch_unwind(AssertUnwindSafe(|| state.commit(journal))).is_err());
    fs::rename(&destination, destination.with_file_name("original-kept")).unwrap();
    fs::create_dir(&destination).unwrap();
    fs::copy(
        destination.with_file_name("original-kept").join("SKILL.md"),
        destination.join("SKILL.md"),
    )
    .unwrap();
    assert!(state.recover().is_err());
    assert!(state.require_recovered().is_err());
    assert!(
        state
            .config
            .data_dir
            .join("journals/recovery-test.json")
            .exists()
    );
    assert!(destination.with_file_name("original-kept").exists());
}
#[test]
fn modified_bytes_cannot_impersonate_renamed_payload() {
    let (_directory, mut state, journal) = fixture();
    let destination = journal.moves[0].to.clone();
    inject(&[("after_rename", true)]);
    assert!(catch_unwind(AssertUnwindSafe(|| state.commit(journal))).is_err());
    fs::write(destination.join("SKILL.md"), "changed bytes").unwrap();
    assert!(state.recover().is_err());
    assert!(state.require_recovered().is_err());
    assert!(
        state
            .config
            .data_dir
            .join("journals/recovery-test.json")
            .exists()
    );
}

#[test]
fn rollback_io_failure_keeps_journal_and_blocks_writes_until_reopen() {
    let (_directory, mut state, journal) = fixture();
    inject(&[("after_metadata", false), ("after_rollback", false)]);
    assert!(state.commit(journal).is_err());
    assert!(state.require_recovered().is_err());
    assert!(
        state
            .config
            .data_dir
            .join("journals/recovery-test.json")
            .exists()
    );
    let root = state.config.data_dir.clone();
    drop(state);
    let mut reopened = State::isolated_test_state(root);
    reopened.recover().unwrap();
    assert_result(&reopened, false);
}

#[test]
fn review_recovery_preserves_external_metadata_changes() {
    let (_directory, mut state, journal) = fixture();
    inject(&[("after_progress", true)]);
    assert!(catch_unwind(AssertUnwindSafe(|| state.commit(journal))).is_err());
    let metadata_path = state.library_dir().join(".skillbinder.json");
    let mut changed = state.read_library().unwrap();
    changed.library_id = "external-change".into();
    write_json(&metadata_path, &changed).unwrap();
    let bytes = fs::read(&metadata_path).unwrap();
    assert!(
        state.recover().is_err(),
        "recovery must refuse externally modified metadata"
    );
    assert_eq!(fs::read(metadata_path).unwrap(), bytes);
}

#[test]
fn rollback_preserves_external_metadata_and_remaining_moves() {
    let (_directory, mut state, journal) = fixture();
    inject(&[("after_metadata", false), ("after_rollback", true)]);
    assert!(catch_unwind(AssertUnwindSafe(|| state.commit(journal))).is_err());
    let metadata_path = state.library_dir().join(".skillbinder.json");
    let mut changed = state.read_library().unwrap();
    changed.library_id = "external-change".into();
    write_json(&metadata_path, &changed).unwrap();
    let bytes = fs::read(&metadata_path).unwrap();
    let destination = state.library_dir().join("skills/skill-0");
    assert!(destination.exists());
    assert!(state.recover().is_err());
    assert_eq!(fs::read(metadata_path).unwrap(), bytes);
    assert!(destination.exists());
    assert!(
        state
            .config
            .data_dir
            .join("journals/recovery-test.json")
            .exists()
    );
}

#[test]
fn committed_journal_refuses_metadata_replaced_with_old_revision() {
    let (_directory, mut state, journal) = fixture();
    let before = journal.before.clone();
    inject(&[("after_database", true)]);
    assert!(catch_unwind(AssertUnwindSafe(|| state.commit(journal))).is_err());
    let metadata_path = state.library_dir().join(".skillbinder.json");
    write_json(&metadata_path, &before).unwrap();
    let bytes = fs::read(&metadata_path).unwrap();
    assert!(state.recover().is_err());
    assert_eq!(fs::read(metadata_path).unwrap(), bytes);
    assert!(
        state
            .config
            .data_dir
            .join("journals/recovery-test.json")
            .exists()
    );
}

#[test]
fn rollback_accepts_reordered_record_maps() {
    let (_directory, mut state, mut journal) = fixture();
    journal.after.folders = vec![
        skillbinder_proto::Folder {
            id: skillbinder_proto::FolderId("z".into()),
            name: "Z".into(),
        },
        skillbinder_proto::Folder {
            id: skillbinder_proto::FolderId("a".into()),
            name: "A".into(),
        },
    ];
    inject(&[("after_metadata", false)]);
    assert!(state.commit(journal).is_err());
    assert_result(&state, false);
}
