use super::{files_changed, moved_text};
use skillbinder_proto::{Moved, SkillId};

#[test]
fn the_summary_counts_files() {
    assert_eq!(files_changed(1), "· 1 file changed ·");
    assert_eq!(files_changed(5), "· 5 files changed ·");
}

#[test]
fn the_moved_notice_says_which_end_moved() {
    let pdf = SkillId("pdf".to_owned());
    assert!(moved_text(&pdf, Moved::Source).starts_with("pdf's source changed while you were"));
    assert!(moved_text(&pdf, Moved::Library).starts_with("pdf changed in your library while"));
}
