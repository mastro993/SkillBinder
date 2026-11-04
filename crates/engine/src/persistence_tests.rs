#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use crate::imports::placement::Held;
use skillbinder_proto::{Preset, PresetId, WorkspaceId};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn fixture() -> (tempfile::TempDir, State, Journal) {
    let directory = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(directory.path()).unwrap();
    let state = State::isolated_test_state(root.clone());
    let mut skills = Vec::new();
    let mut records = Vec::new();
    let moves = (0..2)
        .map(|index| {
            let from = root.join(format!("staging/source-{index}"));
            fs::create_dir(&from).unwrap();
            fs::write(
                from.join("SKILL.md"),
                format!("---\nname: skill-{index}\ndescription: test\n---\nbody-{index}\n"),
            )
            .unwrap();
            let inspection = crate::payload::inspect(&from).unwrap();
            let record = crate::records::Record::of(&inspection, None);
            records.push(RecordChange {
                slug: format!("skill-{index}"),
                record: Some(record.clone()),
                previous: None,
            });
            skills.push(SavedSkill::new(
                format!("skill-{index}"),
                &inspection,
                None,
                record,
                None,
            ));
            Move {
                from,
                to: root.join(format!("library/skills/skill-{index}")),
            }
        })
        .collect();
    let plan_id = skillbinder_proto::PlanId::new();
    diesel::insert_into(operation_plans::table)
        .values((
            operation_plans::id.eq(plan_id.as_str()),
            operation_plans::payload.eq("{}"),
            operation_plans::expires_at.eq(0),
        ))
        .execute(&mut *state.database.borrow_mut())
        .unwrap();
    let journal = Journal {
        id: "recovery-test".into(),
        moves,
        completed_moves: 0,
        progress: Default::default(),
        skills,
        outcome: Some(ImportOutcome {
            plan_id,
            imported: vec![],
            existing: vec![],
        }),
        records,
        held: held(),
    };
    (directory, state, journal)
}
/// An import's activations: `skill-0` for Claude Code in the Default preset, with a confirmed
/// replacement there, and in a workspace removed meanwhile, whose entries recovery skips.
fn held() -> Vec<Held> {
    use crate::imports::placement::Replaced;
    let gone = WorkspaceId("removed".into());
    let slug = || "skill-0".to_owned();
    vec![
        Held::Preset {
            preset: PresetId(Preset::DEFAULT_ID.into()),
            slug: slug(),
            agents: ["claude-code".to_owned()].into(),
        },
        Held::Workspace {
            workspace: gone.clone(),
            slug: slug(),
            dirs: [".claude/skills".to_owned()].into(),
        },
        Held::Replaced {
            directory: "/home/.claude/skills".into(),
            slug: slug(),
            copy: Replaced {
                digest: "theirs".into(),
                workspace: None,
            },
        },
        Held::Replaced {
            directory: "/removed/.claude/skills".into(),
            slug: slug(),
            copy: Replaced {
                digest: "theirs".into(),
                workspace: Some((gone, ".claude/skills".into())),
            },
        },
    ]
}
fn inject(points: &[(&'static str, bool)]) {
    FAULTS.with(|faults| *faults.borrow_mut() = points.iter().copied().collect());
}
fn journals_kept(state: &State) -> i64 {
    journals::table
        .count()
        .get_result(&mut *state.database.borrow_mut())
        .unwrap()
}
fn assert_result(state: &State, applied: bool) {
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
        let record = state.read_record(&format!("skill-{index}"));
        assert_eq!(record.is_some(), applied, "record of skill-{index}");
        let actual = fs::read_to_string(if applied { to } else { from }.join("SKILL.md")).unwrap();
        assert!(actual.contains(&format!("body-{index}")));
    }
    let (details, outcomes, consumed): (i64, i64, i64) = {
        let database = &mut *state.database.borrow_mut();
        (
            crate::schema::skills::table
                .count()
                .get_result(database)
                .unwrap(),
            idempotency_records::table
                .count()
                .get_result(database)
                .unwrap(),
            operation_plans::table
                .select(operation_plans::consumed)
                .first(database)
                .unwrap(),
        )
    };
    assert_eq!(details, 2 * i64::from(applied));
    let (presets, copies): (Vec<(String, Option<String>)>, Vec<String>) = {
        let database = &mut *state.database.borrow_mut();
        (
            crate::schema::preset_skills::table
                .select((
                    crate::schema::preset_skills::slug,
                    crate::schema::preset_skills::agents,
                ))
                .load(database)
                .unwrap(),
            crate::schema::managed_copies::table
                .select(crate::schema::managed_copies::directory)
                .load(database)
                .unwrap(),
        )
    };
    if applied {
        assert_eq!(
            presets,
            [("skill-0".to_owned(), Some(r#"["claude-code"]"#.to_owned()))]
        );
        assert_eq!(copies, ["/home/.claude/skills"]);
    } else {
        assert_eq!((presets.len(), copies.len()), (0, 0));
    }
    assert_eq!(outcomes, i64::from(applied));
    assert_eq!(consumed, i64::from(applied));
    let outcome = state.setting::<ImportOutcome>("import_outcome").unwrap();
    assert_eq!(outcome.is_some(), applied);
    if let Some(outcome) = outcome {
        let stored: String = idempotency_records::table
            .find(outcome.plan_id.as_str())
            .select(idempotency_records::result)
            .first(&mut *state.database.borrow_mut())
            .unwrap();
        assert_eq!(serde_json::to_string(&outcome).unwrap(), stored);
    }
    assert_eq!(journals_kept(state), 0);
    assert!(!state.recovery_required);
}
#[test]
fn crashes_at_apply_boundaries_roll_forward_after_reopen() {
    for point in [
        "before_rename",
        "after_rename",
        "before_progress",
        "after_progress",
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
fn failures_before_database_commit_restore_payload() {
    for point in [
        "before_rename",
        "after_rename",
        "before_progress",
        "after_progress",
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
        inject(&[("before_database", false), (point, true)]);
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
    assert_eq!(journals_kept(&state), 1);
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
    assert_eq!(journals_kept(&state), 1);
}

#[test]
fn rollback_io_failure_keeps_journal_and_blocks_writes_until_reopen() {
    let (_directory, mut state, journal) = fixture();
    inject(&[("before_database", false), ("after_rollback", false)]);
    assert!(state.commit(journal).is_err());
    assert!(state.require_recovered().is_err());
    assert_eq!(journals_kept(&state), 1);
    let root = state.config.data_dir.clone();
    drop(state);
    let mut reopened = State::isolated_test_state(root);
    reopened.recover().unwrap();
    assert_result(&reopened, false);
}
