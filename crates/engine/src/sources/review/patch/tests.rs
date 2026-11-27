//! Every shape the review's patch takes reads back through GPUI Kit's `Diff` parser, which the
//! window draws it with, with the same files, statuses, and figures.

use super::{Files, PATCH_CAP, build};
use gpui_kit::component::diff::{DiffFile, DiffFileStatus};
use skillbinder_proto::{FileChange, ReviewedFile, Shown};

fn side<'a>(files: &[(&'a str, &'a str, bool)]) -> Files<'a> {
    files
        .iter()
        .map(|(path, text, executable)| (*path, (text.as_bytes(), *executable)))
        .collect()
}

/// The patch from `before` to `after`, checked to parse into the files it lists.
fn parsed(before: &Files, after: &Files) -> (Vec<ReviewedFile>, Vec<DiffFile>, String) {
    let (files, patch) = build(before, after);
    let parsed = DiffFile::parse(&patch).unwrap_or_else(|error| panic!("{error}:\n{patch}"));
    assert_eq!(parsed.len(), files.len(), "{patch}");
    for (file, read) in files.iter().zip(&parsed) {
        assert_eq!(read.path().as_ref(), file.path, "{patch}");
        assert_eq!(read.additions() as u64, file.additions, "{patch}");
        assert_eq!(read.deletions() as u64, file.deletions, "{patch}");
    }
    (files, parsed, patch)
}

#[test]
fn added_changed_and_removed_files_read_back_sorted_by_path() {
    let skill = "---\nname: pdf\n---\n# PDF\n\nRead a PDF.\n";
    let before = side(&[
        ("SKILL.md", skill, false),
        ("reference.md", "Old notes.\n", false),
        ("scripts/extract.py", "print('a')\n", false),
    ]);
    let changed = skill.replace("Read", "Read or fill");
    let after = side(&[
        ("SKILL.md", &changed, false),
        ("scripts/extract.py", "print('a')\nprint('b')\n", false),
        ("scripts/fill_form.py", "fill()\n", true),
    ]);
    let (files, parsed, _) = parsed(&before, &after);
    let shapes: Vec<_> = files
        .iter()
        .map(|file| {
            (
                file.path.as_str(),
                file.change,
                file.additions,
                file.deletions,
            )
        })
        .collect();
    assert_eq!(
        shapes,
        [
            ("SKILL.md", FileChange::Changed, 1, 1),
            ("reference.md", FileChange::Removed, 0, 1),
            ("scripts/extract.py", FileChange::Changed, 1, 0),
            ("scripts/fill_form.py", FileChange::Added, 1, 0),
        ]
    );
    let statuses: Vec<_> = parsed.iter().map(DiffFile::status).collect();
    assert_eq!(
        statuses,
        [
            DiffFileStatus::Modified,
            DiffFileStatus::Deleted,
            DiffFileStatus::Modified,
            DiffFileStatus::Added,
        ]
    );
    assert!(
        parsed[3]
            .extended_headers()
            .iter()
            .any(|header| header.as_ref() == "new file mode 100755")
    );
}

#[test]
fn the_whole_text_is_context_so_every_unchanged_line_can_unfold() {
    let before: String = (1..=40).map(|line| format!("line {line}\n")).collect();
    let after = before.replace("line 20\n", "line twenty\n");
    let (_, _, patch) = parsed(
        &side(&[("SKILL.md", &before, false)]),
        &side(&[("SKILL.md", &after, false)]),
    );
    assert!(patch.contains("@@ -1,40 +1,40 @@"), "{patch}");
}

#[test]
fn a_mode_change_alone_reads_back_as_extended_headers() {
    let (files, parsed, patch) = parsed(
        &side(&[("run.sh", "echo\n", false)]),
        &side(&[("run.sh", "echo\n", true)]),
    );
    assert_eq!(files[0].change, FileChange::Changed);
    assert!(
        patch.contains("old mode 100644\nnew mode 100755\n"),
        "{patch}"
    );
    assert!(parsed[0].has_changes());
}

#[test]
fn a_missing_final_newline_and_an_empty_added_file_read_back() {
    parsed(
        &side(&[("SKILL.md", "a\nb", false)]),
        &side(&[("SKILL.md", "a\nc", false), ("empty.txt", "", false)]),
    );
}

#[test]
fn binary_or_non_utf8_content_is_one_line_not_garbage() {
    let before: Files = [("logo.png", (&b"\x89PNG\0\x01"[..], false))].into();
    let after: Files = [
        ("logo.png", (&b"\x89PNG\0\x02"[..], false)),
        ("latin1.txt", (&b"caf\xe9\n"[..], false)),
    ]
    .into();
    let (files, parsed, patch) = parsed(&before, &after);
    assert!(parsed.iter().all(DiffFile::is_binary), "{patch}");
    assert!(patch.contains("Binary files /dev/null and b/latin1.txt differ"));
    let shown: Vec<_> = files
        .iter()
        .map(|file| (file.path.as_str(), file.shown))
        .collect();
    assert_eq!(
        shown,
        [
            (
                "latin1.txt",
                Shown::Binary {
                    before: None,
                    after: Some(5)
                }
            ),
            (
                "logo.png",
                Shown::Binary {
                    before: Some(6),
                    after: Some(6)
                }
            ),
        ]
    );
    assert!(
        files
            .iter()
            .all(|file| file.additions + file.deletions == 0)
    );
}

#[test]
fn a_text_file_over_the_preview_limit_on_either_side_shows_without_lines() {
    let large = "a line of text\n".repeat(200 * 1024 / 15);
    let (files, parsed, patch) = parsed(
        &side(&[("big.md", &large, false), ("SKILL.md", "old\n", false)]),
        &side(&[("big.md", "small\n", false), ("SKILL.md", "new\n", false)]),
    );
    let shown: Vec<_> = files
        .iter()
        .map(|file| (file.path.as_str(), file.shown))
        .collect();
    assert_eq!(
        shown,
        [("SKILL.md", Shown::Lines), ("big.md", Shown::TooLarge)]
    );
    assert_eq!(parsed[1].status(), DiffFileStatus::Modified);
    assert!(
        !patch.contains("a line of text"),
        "big.md's lines are not in the patch"
    );
}

#[test]
fn files_whose_lines_pass_the_cap_show_without_them_and_so_do_all_later_ones() {
    // Each file is under the preview limit; together they pass the cap.
    let text = "x".repeat(99) + "\n";
    let chunk = text.repeat(1200);
    let names: Vec<String> = (0..40).map(|ix| format!("part-{ix:02}.md")).collect();
    let mut after: Vec<(&str, &str, bool)> = names
        .iter()
        .map(|name| (name.as_str(), chunk.as_str(), false))
        .collect();
    after.push(("zz.md", "small\n", false));
    let (files, _, patch) = parsed(&side(&[]), &side(&after));
    assert!(
        patch.len() <= PATCH_CAP + 64 * files.len(),
        "only headers pass the cap"
    );
    let first = files
        .iter()
        .position(|file| file.shown == Shown::PastCap)
        .unwrap_or_else(|| panic!("the cap is reached"));
    assert!(first > 0);
    assert!(files[..first].iter().all(|file| file.shown == Shown::Lines));
    // A small file after the cap shows without its lines too.
    assert!(
        files[first..]
            .iter()
            .all(|file| file.shown == Shown::PastCap && file.additions == 0)
    );
}

#[test]
fn identical_files_are_not_listed() {
    let same = side(&[("SKILL.md", "same\n", false)]);
    assert_eq!(build(&same, &same), (Vec::new(), String::new()));
}

#[test]
fn a_bare_carriage_return_stays_inside_its_line() {
    // Kit ends lines only at `\n`, so a stray `\r` must not end one in the patch.
    let (files, _, _) = parsed(
        &side(&[("notes.md", "a\rb\nkeep\n", false), ("z.md", "x\r", false)]),
        &side(&[("notes.md", "a\rc\nkeep\n", false), ("z.md", "y\r", false)]),
    );
    let figures: Vec<_> = files
        .iter()
        .map(|file| (file.path.as_str(), file.additions, file.deletions))
        .collect();
    assert_eq!(figures, [("notes.md", 1, 1), ("z.md", 1, 1)]);
}
