use super::super::snapshot_query::fixture::{Fixture, HOME, id};
use super::*;
use skillbinder_proto::ManagedCopy;

fn copy(directory: &str, state: CopyState) -> ManagedCopy {
    ManagedCopy {
        skill: id("review"),
        directory: directory.to_owned(),
        managed: true,
        state,
    }
}

#[test]
fn the_edited_mark_names_the_folders_holding_an_edited_copy() {
    let snapshot = Fixture::new()
        .agent(
            "claude-code",
            "Claude Code",
            ".claude/skills",
            ".claude/skills",
            true,
        )
        .agent("zed", "Zed", ".zed/skills", ".zed/skills", true)
        .workspace("acme-web", &[".claude/skills", ".zed/skills"])
        .build();
    let workspace = &snapshot.workspaces[0];
    let folders = folders(&snapshot.agents, workspace, HOME);
    let claude = format!("{}/.claude/skills", workspace.path);
    let zed = format!("{}/.zed/skills", workspace.path);
    let edited = edited_in(
        &[
            copy(&claude, CopyState::Edited { active: true }),
            copy(&zed, CopyState::Applied),
        ],
        &id("review"),
        &folders,
    );
    assert_eq!(edited.as_deref(), Some("Claude Code"));
    assert_eq!(
        edited_in(&[copy(&zed, CopyState::Applied)], &id("review"), &folders),
        None
    );
}

#[test]
fn the_overlap_mark_keeps_the_four_labels() {
    assert_eq!(overlap_label(Overlap::Twice), "Also in your agents");
    assert_eq!(overlap_label(Overlap::Hidden), "Hidden by your agents");
    assert_eq!(overlap_label(Overlap::Overrides), "Overrides your agents");
    assert_eq!(overlap_label(Overlap::OneCopy), "One copy is used");
}
