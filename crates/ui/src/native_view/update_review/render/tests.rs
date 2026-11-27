use super::{drift_text, files_changed, moved_text, place};
use skillbinder_proto::{Moved, SkillId, SkillSource};

#[test]
fn the_summary_names_the_repository_without_its_scheme_and_counts_files() {
    let source = |url: &str, path: &str| SkillSource {
        url: url.to_owned(),
        reference: None,
        path: path.to_owned(),
    };
    assert_eq!(
        place(&source(
            "https://github.com/Anthropics/skills.git",
            "skills/pdf"
        )),
        "github.com/anthropics/skills · skills/pdf"
    );
    assert_eq!(
        place(&source("git@example.com:team/skills", "")),
        "git@example.com/team/skills"
    );
    assert_eq!(files_changed(1), "· 1 file changed ·");
    assert_eq!(files_changed(5), "· 5 files changed ·");
}

#[test]
fn the_moved_notice_says_which_end_moved() {
    let pdf = SkillId("pdf".to_owned());
    assert!(moved_text(&pdf, Moved::Source).starts_with("pdf's source changed while you were"));
    assert!(moved_text(&pdf, Moved::Library).starts_with("pdf changed in your library while"));
}

#[test]
fn the_drift_notice_says_local_edits_show_as_removed_lines() {
    assert_eq!(
        drift_text(&SkillId("pdf".to_owned())),
        "pdf was changed outside SkillBinder. Updating replaces those changes too; they show as removed lines."
    );
}
