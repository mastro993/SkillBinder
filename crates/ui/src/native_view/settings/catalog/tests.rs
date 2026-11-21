use super::{Group, Section, SettingRow, search};

fn groups() -> Vec<Group> {
    vec![
        Group::titled(
            "Library",
            vec![
                SettingRow::new("folder", "Library folder").detail("~/Library/SkillBinder"),
                SettingRow::new("storage", "Storage").keywords(&["writable"]),
            ],
        ),
        Group::titled(
            "Sync",
            vec![
                SettingRow::new("git", "Git").detail("Syncs your library."),
                SettingRow::new("remote", "Remote"),
            ],
        ),
    ]
}

fn titles(groups: &[Group]) -> Vec<(Option<&str>, Vec<&str>)> {
    groups
        .iter()
        .map(|group| {
            let rows = group.rows.iter().map(|row| row.title.as_ref()).collect();
            (group.title, rows)
        })
        .collect()
}

#[test]
fn an_empty_query_keeps_every_group_and_what_an_empty_one_shows() {
    let mut all = groups();
    all.push(Group::default());
    let kept = search(Section::Library, all, "  ");
    assert_eq!(kept.len(), 3);
}

#[test]
fn a_query_finds_rows_by_title_line_or_keyword_ignoring_case() {
    let cases = [
        ("FOLDER", vec![(Some("Library"), vec!["Library folder"])]),
        ("writable", vec![(Some("Library"), vec!["Storage"])]),
        ("syncs", vec![(Some("Sync"), vec!["Git"])]),
        ("zzz", vec![]),
    ];
    for (query, expected) in cases {
        let kept = search(Section::Library, groups(), query);
        assert_eq!(titles(&kept), expected, "{query}");
    }
}

#[test]
fn a_group_or_section_title_finds_all_of_its_rows_and_drops_empty_groups() {
    let kept = search(Section::Library, groups(), "sync");
    assert_eq!(titles(&kept).len(), 2, "the section title holds 'sync'");
    let mut with_empty = groups();
    with_empty.push(Group::default());
    let kept = search(Section::Updates, with_empty, "sync");
    assert_eq!(titles(&kept), vec![(Some("Sync"), vec!["Git", "Remote"])]);
}
