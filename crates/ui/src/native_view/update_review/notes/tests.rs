use super::{note, size};
use skillbinder_proto::Shown;

#[test]
fn a_binary_file_shows_both_sizes_only_when_both_sides_have_it() {
    let binary = |before, after| note(Shown::Binary { before, after });
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
    assert_eq!(note(Shown::Lines), None);
    assert_eq!(
        note(Shown::TooLarge).as_deref(),
        Some("Too large to show (over 128 KiB)")
    );
    assert_eq!(
        note(Shown::PastCap).as_deref(),
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
