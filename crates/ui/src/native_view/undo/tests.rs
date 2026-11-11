use super::super::snapshot_query::fixture::{Fixture, dirs, id};
use super::*;
use skillbinder_proto::WorkspaceId;

fn copy(skill: &str, directory: &str, state: CopyState) -> ManagedCopy {
    ManagedCopy {
        skill: id(skill),
        directory: directory.to_owned(),
        managed: true,
        state,
    }
}

fn set(dirs: &[&str]) -> BTreeSet<String> {
    dirs.iter().map(|dir| (*dir).to_owned()).collect()
}

#[test]
fn only_an_edited_copy_left_behind_in_the_place_asks_instead_of_undo() {
    let here = set(&["/ws/.claude/skills"]);
    let asking = CopyState::Edited { active: false };
    let ask = |copies: &[ManagedCopy]| edited_asks(copies, &id("review"), &here);
    assert!(ask(&[copy("review", "/ws/.claude/skills", asking.clone())]));
    assert!(
        !ask(&[copy(
            "review",
            "/ws/.claude/skills",
            CopyState::Edited { active: true }
        )]),
        "a copy still active there is not being removed"
    );
    assert!(
        !ask(&[copy("pdf", "/ws/.claude/skills", asking.clone())]),
        "another skill's copy"
    );
    assert!(
        !ask(&[copy("review", "/home/.claude/skills", asking)]),
        "a copy in another place"
    );
    assert!(!ask(&[copy(
        "review",
        "/ws/.claude/skills",
        CopyState::Conflict
    )]));
}

#[test]
fn undo_restores_the_captured_choice_while_its_workspace_and_skill_remain() {
    let snapshot = Fixture::new()
        .workspace("acme-web", &[".claude/skills"])
        .skill("review", "", None)
        .build();
    let undo = |workspace: &str, skill: &str| Undo::Workspace {
        id: WorkspaceId(workspace.to_owned()),
        skill: id(skill),
        previous: dirs(&[".claude/skills"]),
    };
    assert!(restorable(&undo("acme-web", "review"), &snapshot));
    assert!(!restorable(&undo("gone", "review"), &snapshot));
    assert!(!restorable(&undo("acme-web", "gone"), &snapshot));
    let missing = Fixture::new()
        .workspace("acme-web", &[".claude/skills"])
        .missing()
        .skill("review", "", None)
        .build();
    assert!(
        !restorable(&undo("acme-web", "review"), &missing),
        "the engine would refuse a missing folder"
    );
    let Undo::Workspace { previous, .. } = undo("acme-web", "review");
    assert_eq!(
        previous,
        dirs(&[".claude/skills"]),
        "Undo keeps the exact directories"
    );
}
