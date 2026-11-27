use super::size;
use crate::{Shown, SkillId, SkillSource, UpdateReview};

#[test]
fn a_review_names_the_repository_without_its_scheme_then_the_folder() {
    let source = |url: &str, path: &str| SkillSource {
        url: url.to_owned(),
        reference: None,
        path: path.to_owned(),
    };
    assert_eq!(
        source("https://github.com/Anthropics/skills.git", "skills/pdf").place(),
        "github.com/anthropics/skills · skills/pdf"
    );
    assert_eq!(
        source("git@example.com:team/skills", "").place(),
        "git@example.com/team/skills"
    );
}

#[test]
fn a_binary_file_shows_both_sizes_only_when_both_sides_have_it() {
    let binary = |before, after| Shown::Binary { before, after }.note();
    assert_eq!(
        binary(Some(18_200), Some(21_000)).as_deref(),
        Some("Binary file changed · 18 KB → 21 KB")
    );
    assert_eq!(
        binary(None, Some(21_000)).as_deref(),
        Some("Binary file changed")
    );
}

#[test]
fn files_without_lines_say_why() {
    assert_eq!(Shown::Lines.note(), None);
    assert_eq!(
        Shown::TooLarge.note().as_deref(),
        Some("Too large to show (over 128 KiB)")
    );
    assert_eq!(
        Shown::PastCap.note().as_deref(),
        Some("Not shown: this update's changes are too large to show in full.")
    );
}

#[test]
fn sizes_read_as_finder_shows_them() {
    assert_eq!(size(512), "512 B");
    assert_eq!(size(18_499), "18 KB");
    assert_eq!(size(999_999), "1.0 MB");
    assert_eq!(size(1_250_000), "1.3 MB");
}

#[test]
fn only_a_drifted_review_warns_that_local_edits_show_as_removed_lines() {
    let mut review = UpdateReview {
        id: SkillId("pdf".to_owned()),
        source: SkillSource {
            url: "https://github.com/acme/skills".to_owned(),
            reference: None,
            path: "skills/pdf".to_owned(),
        },
        from: String::new(),
        to: String::new(),
        drifted: false,
        files: Vec::new(),
        patch: String::new(),
    };
    assert_eq!(review.drift_note(), None);
    review.drifted = true;
    assert_eq!(
        review.drift_note().as_deref(),
        Some(
            "pdf was changed outside SkillBinder. Updating replaces those changes too; they show as removed lines."
        )
    );
}
