use super::Choices;
use skillbinder_proto::{ChangeKind, Conflict, ConflictSide, Side};

fn conflict(slug: &str, here: &str, there: &str) -> Conflict {
    let side = |version: &str| ConflictSide {
        kind: ChangeKind::Changed,
        reason: None,
        device: None,
        at: None,
        version: version.to_owned(),
    };
    Conflict {
        slug: slug.to_owned(),
        local: side(here),
        remote: side(there),
    }
}

#[test]
fn a_choice_lasts_only_while_both_versions_it_was_made_for_do() {
    let mut choices = Choices::default();
    let seen = [conflict("alpha", "a1", "b1")];
    choices.follow(&seen);
    choices.chosen.insert("alpha".into(), Side::Local);
    choices.follow(&seen);
    assert_eq!(choices.of("alpha"), Some(Side::Local));
    assert!(choices.complete(&seen));
    // The remote brought a newer alpha the user has not seen.
    choices.follow(&[conflict("alpha", "a1", "b2")]);
    assert_eq!(choices.of("alpha"), None);
    choices.chosen.insert("alpha".into(), Side::Remote);
    // This device's alpha changed again after the choice.
    choices.follow(&[conflict("alpha", "a2", "b2")]);
    assert_eq!(choices.of("alpha"), None);
}
