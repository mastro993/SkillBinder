use super::*;
use skillbinder_proto::{ActivationStep, CopyState, SkillId};

fn progress(done: usize, total: usize) -> ActivationProgress {
    ActivationProgress {
        done,
        total,
        step: None,
    }
}

fn copy(directory: &str, slug: &str, state: CopyState) -> ManagedCopy {
    ManagedCopy {
        skill: SkillId(slug.to_owned()),
        directory: directory.to_owned(),
        managed: true,
        state,
    }
}

/// A strip for `activity` whose change landed at `now`, never shown.
fn strip_landed(activity: Activity, landing: Landing, now: Instant) -> Strip {
    let mut strip = Strip::new(activity, now);
    strip.land(landing, now);
    strip
}

fn switching() -> Activity {
    Activity::Switching {
        preset: "Frontend".to_owned(),
    }
}

#[test]
fn a_change_that_lands_before_the_reveal_ends_at_once_with_its_toast() {
    let now = Instant::now();
    let mut strip = Strip::new(switching(), now);
    assert_eq!(strip.shown(), Shown::Blocker);
    let landed = now + Duration::from_millis(120);
    assert_eq!(strip.land(Landing::default(), landed), landed);
    assert_eq!(strip.shown(), Shown::Blocker, "it never shows");
    assert!(!strip.washed(), "no wash fades out");
    assert_eq!(
        strip.finish(landed, &[]),
        Finish::Toast("Switched to Frontend".to_owned())
    );
}

#[test]
fn a_shown_strip_stays_at_least_500_ms() {
    let now = Instant::now();
    let revealed = now + REVEAL_AFTER;
    assert_eq!(ends_at(None, now), now);
    assert_eq!(ends_at(Some(revealed), revealed), revealed + SHOWN_AT_LEAST);
    let late = revealed + Duration::from_secs(2);
    assert_eq!(ends_at(Some(revealed), late), late);

    let mut strip = Strip::new(switching(), now);
    strip.reveal(revealed);
    assert!(strip.washed(), "its wash fades out once it ends");
    let landed = revealed + Duration::from_millis(100);
    assert_eq!(
        strip.land(Landing::default(), landed),
        revealed + SHOWN_AT_LEAST
    );
    assert_eq!(strip.ends(), Some(revealed + SHOWN_AT_LEAST));
    assert_eq!(
        strip.shown(),
        Shown::Working(100),
        "it holds at 100% meanwhile"
    );
}

#[test]
fn the_shown_percent_never_goes_down() {
    let now = Instant::now();
    let mut strip = Strip::new(switching(), now);
    strip.reveal(now + REVEAL_AFTER);
    strip.follow(Some(&progress(9, 20)));
    assert_eq!(strip.shown(), Shown::Working(45));
    // A refresh publishing no progress, or a smaller total planned by a later pass, keeps 45%.
    strip.follow(None);
    strip.follow(Some(&progress(9, 40)));
    assert_eq!(strip.shown(), Shown::Working(45));
    strip.follow(Some(&progress(19, 20)));
    assert_eq!(strip.shown(), Shown::Working(95));
}

#[test]
fn an_older_waiting_copy_of_another_skill_ends_the_change_in_the_done_pill() {
    let now = Instant::now();
    let mut strip = strip_landed(switching(), Landing::default(), now);
    // The switch wrote its copy; a conflict it never touched waits from before.
    let copies = [
        copy("/h/.claude/skills", "tailwind-helper", CopyState::Applied),
        copy("/h/.cursor/skills", "release-notes", CopyState::Conflict),
    ];
    assert_eq!(strip.finish(now, &copies), Finish::Done);
    assert_eq!(
        strip.shown(),
        Shown::Done(1),
        "it shows at once though it never had"
    );
}

#[test]
fn the_done_pill_counts_the_copies_its_review_lists() {
    let now = Instant::now();
    let reason = || "No access.".to_owned();
    let copies = [
        copy("/h/.claude/skills", "applied", CopyState::Applied),
        copy("/h/.claude/skills", "kept", CopyState::Skipped),
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
        copy(
            "/h/.claude/skills",
            "unremoved",
            CopyState::NotRemoved(reason()),
        ),
    ];
    let mut strip = strip_landed(switching(), Landing::default(), now);
    assert_eq!(strip.finish(now, &copies), Finish::Done);
    let review = Review::of(&copies);
    assert_eq!(review.len(), 4);
    assert_eq!(strip.shown(), Shown::Done(review.len()));
}

#[test]
fn a_change_that_leaves_no_copy_waiting_ends_with_its_toast() {
    let now = Instant::now();
    let mut strip = strip_landed(switching(), Landing::default(), now);
    // A pass could not write a copy that a later pass of the same change wrote, and a copy the
    // user kept waits for nothing.
    let copies = [
        copy("/h/.claude/skills", "applied", CopyState::Applied),
        copy("/h/.cursor/skills", "kept", CopyState::Skipped),
    ];
    assert_eq!(
        strip.finish(now, &copies),
        Finish::Toast("Switched to Frontend".to_owned())
    );
    assert_eq!(strip.shown(), Shown::Blocker);
}

#[test]
fn the_done_pill_follows_the_copies_that_wait_until_none_does() {
    let now = Instant::now();
    let conflict = copy("/h/.cursor/skills", "conflict", CopyState::Conflict);
    let edited = copy(
        "/h/.claude/skills",
        "edited",
        CopyState::Edited { active: true },
    );
    let mut strip = Strip::new(switching(), now);
    strip.recount(std::slice::from_ref(&conflict));
    assert_eq!(
        strip.shown(),
        Shown::Blocker,
        "a running strip counts nothing"
    );
    strip.land(Landing::default(), now);
    assert_eq!(
        strip.finish(now, std::slice::from_ref(&conflict)),
        Finish::Done
    );
    // A scan after the change found an edited copy before Review was clicked.
    strip.recount(&[conflict.clone(), edited.clone()]);
    assert_eq!(strip.shown(), Shown::Done(2));
    strip.recount(&[edited]);
    assert_eq!(strip.shown(), Shown::Done(1));
    strip.recount(&[]);
    assert_eq!(strip.shown(), Shown::Done(1), "Review then opens nothing");
}

#[test]
fn a_refused_change_closes_without_a_toast() {
    let now = Instant::now();
    let mut strip = Strip::new(switching(), now);
    let revealed = now + REVEAL_AFTER;
    strip.reveal(revealed);
    strip.follow(Some(&progress(1, 4)));
    assert_eq!(strip.fail(revealed), revealed + SHOWN_AT_LEAST);
    assert_eq!(strip.shown(), Shown::Working(25));
    assert_eq!(strip.finish(revealed + SHOWN_AT_LEAST, &[]), Finish::Closed);
}

#[test]
fn labels_and_toasts_name_each_change() {
    let deleting = Activity::DeletingActive {
        preset: "Frontend".to_owned(),
    };
    let importing = Activity::Importing { skills: 6 };
    let imported = |count| Landing { imported: count };
    assert_eq!(switching().label(), "Switching…");
    assert_eq!(deleting.label(), "Switching…");
    assert_eq!(importing.label(), "Importing…");
    assert_eq!(switching().toast(&imported(0)), "Switched to Frontend");
    assert_eq!(
        deleting.toast(&imported(0)),
        "Deleted Frontend · Switched to Default"
    );
    assert_eq!(importing.toast(&imported(6)), "Imported 6 skills");
    assert_eq!(importing.toast(&imported(1)), "Imported 1 skill");
    let adding = |slugs: &[&str]| Activity::Adding {
        skills: slugs.iter().map(|slug| (*slug).to_owned()).collect(),
    };
    assert_eq!(adding(&["pdf"]).label(), "Adding…");
    assert_eq!(adding(&["pdf"]).title(), "Adding pdf");
    assert_eq!(adding(&["pdf"]).toast(&imported(0)), "Added pdf");
    assert_eq!(
        adding(&["pdf", "a", "b"]).toast(&imported(0)),
        "Added 3 skills"
    );
    assert_eq!(done_label(1), "Done · 1 needs attention");
    assert_eq!(done_label(2), "Done · 2 need attention");
}

#[test]
fn the_tooltip_names_the_change_and_its_step() {
    let now = Instant::now();
    let mut strip = Strip::new(switching(), now);
    let place = |directory: &str| directory.trim_start_matches("/h/").to_owned();
    assert_eq!(strip.tooltip(place), "Switching to Frontend");
    let step = |slug: &str, directory: &str, removing| ActivationStep {
        skill: SkillId(slug.to_owned()),
        directory: directory.to_owned(),
        removing,
    };
    strip.follow(Some(&ActivationProgress {
        step: Some(step("tailwind-helper", "/h/Cursor", false)),
        ..progress(2, 20)
    }));
    assert_eq!(
        strip.tooltip(place),
        "Switching to Frontend\nCopying tailwind-helper → Cursor"
    );
    strip.follow(Some(&ActivationProgress {
        step: Some(step("release-notes", "/h/Claude Code", true)),
        ..progress(3, 20)
    }));
    assert_eq!(
        strip.tooltip(place),
        "Switching to Frontend\nRemoving release-notes from Claude Code"
    );
    assert_eq!(
        Activity::Importing { skills: 6 }.title(),
        "Importing 6 skills"
    );
}
