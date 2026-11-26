use super::catalog::Section;
use super::shown_tab;

#[test]
fn a_chosen_tab_shows_while_the_search_finds_something_in_it() {
    let found = [Section::General, Section::Agents];
    assert_eq!(
        shown_tab(Some(Section::Agents), &found),
        Some(Section::Agents)
    );
}

#[test]
fn a_chosen_tab_the_search_finds_nothing_in_shows_all() {
    let found = [Section::General];
    assert_eq!(shown_tab(Some(Section::Agents), &found), None);
    assert_eq!(shown_tab(Some(Section::Agents), &[]), None);
}

#[test]
fn all_stays_all() {
    assert_eq!(shown_tab(None, &Section::ALL), None);
}
