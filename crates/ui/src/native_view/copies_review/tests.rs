use super::*;

fn copy(directory: &str, slug: &str, state: CopyState) -> ManagedCopy {
    ManagedCopy {
        skill: SkillId(slug.to_owned()),
        directory: directory.to_owned(),
        managed: true,
        state,
    }
}

fn item(directory: &str, slug: &str) -> Item {
    (directory.to_owned(), SkillId(slug.to_owned()))
}

#[test]
fn the_review_lists_exactly_the_copies_that_need_attention() {
    let reason = || "No access.".to_owned();
    let copies = [
        copy("/h/.claude/skills", "applied", CopyState::Applied),
        copy("/h/.claude/skills", "skipped", CopyState::Skipped),
        copy("/h/.cursor/skills", "conflict", CopyState::Conflict),
        copy(
            "/h/.claude/skills",
            "edited",
            CopyState::Edited { active: false },
        ),
        copy(
            "/h/.claude/skills",
            "unwritten",
            CopyState::NotApplied(reason()),
        ),
        copy("/h/.claude/skills", "kept", CopyState::NotRemoved(reason())),
    ];
    let review = Review::of(&copies);
    assert_eq!(
        review.items,
        [
            item("/h/.cursor/skills", "conflict"),
            item("/h/.claude/skills", "edited"),
            item("/h/.claude/skills", "unwritten"),
            item("/h/.claude/skills", "kept"),
        ]
    );
    let expected: Vec<_> = copies
        .iter()
        .filter(|copy| copy.state.needs_attention())
        .map(|copy| (copy.directory.clone(), copy.skill.clone()))
        .collect();
    assert_eq!(review.items, expected);
}

#[test]
fn each_problem_offers_its_answers_and_settles_with_what_was_done() {
    let conflict = copy("/h/.cursor/skills", "tailwind-helper", CopyState::Conflict);
    let edited = copy(
        "/h/.claude/skills",
        "release-notes",
        CopyState::Edited { active: false },
    );
    let unwritten = copy(
        "/h/.windsurf/skills",
        "pdf-tools",
        CopyState::NotApplied("No access.".to_owned()),
    );
    let mut review = Review::of(&[conflict.clone(), edited.clone(), unwritten.clone()]);
    let rows = |review: &Review, copies: &[ManagedCopy]| -> Vec<Row> {
        review
            .items
            .iter()
            .map(|item| review.row(item, copies))
            .collect()
    };
    let asked = [conflict.clone(), edited.clone(), unwritten.clone()];
    let choices: Vec<_> = rows(&review, &asked)
        .into_iter()
        .map(|row| match row {
            Row::Ask(choice, _) => (choice.reason, choice.yes, choice.no),
            Row::Settled(word) => panic!("settled early: {word}"),
        })
        .collect();
    assert_eq!(
        choices,
        [
            ("Different copy there", Answer::Replace, Answer::Keep),
            ("You changed it here", Answer::Remove, Answer::Keep),
            ("Can't write the folder", Answer::Retry, Answer::Skip),
        ]
    );

    // Replace landed, Keep removed the edited copy's record, and Skip settles at once.
    review.answers.insert(
        item(&conflict.directory, "tailwind-helper"),
        Answer::Replace,
    );
    review
        .answers
        .insert(item(&edited.directory, "release-notes"), Answer::Keep);
    review
        .answers
        .insert(item(&unwritten.directory, "pdf-tools"), Answer::Skip);
    let now = [
        copy("/h/.cursor/skills", "tailwind-helper", CopyState::Applied),
        unwritten,
    ];
    assert_eq!(
        rows(&review, &now),
        [
            Row::Settled("Replaced"),
            Row::Settled("Kept"),
            Row::Settled("Skipped"),
        ]
    );
}

#[test]
fn an_answer_that_did_not_take_asks_again() {
    let conflict = copy("/h/.cursor/skills", "tailwind-helper", CopyState::Conflict);
    let mut review = Review::of(std::slice::from_ref(&conflict));
    review.answers.insert(
        item(&conflict.directory, "tailwind-helper"),
        Answer::Replace,
    );
    assert!(matches!(
        review.row(&review.items[0], std::slice::from_ref(&conflict)),
        Row::Ask(..)
    ));
    // A copy settled some other way reads as done.
    let fresh = Review::of(std::slice::from_ref(&conflict));
    assert_eq!(fresh.row(&fresh.items[0], &[]), Row::Settled("Done"));
}
