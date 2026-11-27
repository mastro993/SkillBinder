#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;
use crate::scan::ImportSource;
use std::{fs, path::Path};

fn source(directory: &Path) -> ImportSource {
    fs::create_dir_all(directory).unwrap();
    fs::write(
        directory.join("SKILL.md"),
        "---\nname: moving\ndescription: test\n---\nbody\n",
    )
    .unwrap();
    ImportSource {
        display_path: directory.display().to_string(),
        inspection: crate::payload::inspect(directory).unwrap(),
        locks: Vec::new(),
    }
}

#[test]
fn a_source_that_changed_since_the_scan_is_left_out_of_the_plan() {
    let root = tempfile::tempdir().unwrap();
    let directory = dunce::canonicalize(root.path()).unwrap().join("moving");
    let found = source(&directory);
    assert!(plannable(&found).unwrap().is_some(), "unchanged source");

    // The replacement exists beside the original before it takes its place, so the file
    // system cannot hand it the original's freed inode, as Linux does on a delete and recreate.
    let replacement = directory.with_file_name("replacement");
    source(&replacement);
    let removed = directory.with_file_name("removed");
    fs::rename(&directory, &removed).unwrap();
    assert!(plannable(&found).unwrap().is_none(), "removed source");

    fs::rename(&replacement, &directory).unwrap();
    assert!(plannable(&found).unwrap().is_none(), "replaced source");
}

#[test]
fn storing_a_plan_prunes_the_expired_ones() {
    let root = tempfile::tempdir().unwrap();
    let state = State::isolated_test_state(dunce::canonicalize(root.path()).unwrap());
    let row = |id: &str, expires_at: i64| {
        diesel::insert_into(operation_plans::table)
            .values((
                operation_plans::id.eq(id),
                operation_plans::payload.eq("{}"),
                operation_plans::expires_at.eq(expires_at),
                operation_plans::consumed.eq(0),
            ))
            .execute(&mut *state.database.borrow_mut())
            .unwrap();
    };
    let now = persistence::now() as i64;
    row("expired", now - 1);
    row("live", now + PLAN_LIFETIME as i64);
    let prepared = Prepared {
        review: ImportPlan {
            id: PlanId::new(),
            expires_at: persistence::now() + PLAN_LIFETIME,
            revision: String::new(),
            items: Vec::new(),
        },
        sources: Vec::new(),
        origins: Vec::new(),
        placements: Vec::new(),
        found_in: Vec::new(),
    };

    state.store(&prepared).unwrap();

    let mut ids: Vec<String> = operation_plans::table
        .select(operation_plans::id)
        .load(&mut *state.database.borrow_mut())
        .unwrap();
    ids.sort();
    let mut expected = vec!["live".to_owned(), prepared.review.id.as_str().to_owned()];
    expected.sort();
    assert_eq!(ids, expected);
}
