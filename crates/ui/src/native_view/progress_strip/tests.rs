use super::*;
use skillbinder_proto::{ActivationStep, SkillId};

fn progress(done: usize, total: usize, attention: usize) -> ActivationProgress {
    ActivationProgress {
        done,
        total,
        attention,
        step: None,
    }
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
    assert_eq!(strip.land(progress(20, 20, 0).into(), landed), landed);
    assert_eq!(strip.shown(), Shown::Blocker, "it never shows");
    assert_eq!(
        strip.finish(landed),
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
    let landed = revealed + Duration::from_millis(100);
    assert_eq!(
        strip.land(progress(3, 3, 0).into(), landed),
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
    strip.follow(Some(&progress(9, 20, 0)));
    assert_eq!(strip.shown(), Shown::Working(45));
    // A refresh publishing no progress, or a smaller total planned by a later pass, keeps 45%.
    strip.follow(None);
    strip.follow(Some(&progress(9, 40, 0)));
    assert_eq!(strip.shown(), Shown::Working(45));
    strip.follow(Some(&progress(19, 20, 0)));
    assert_eq!(strip.shown(), Shown::Working(95));
}

#[test]
fn copies_that_need_attention_turn_the_strip_into_the_done_pill() {
    let now = Instant::now();
    let mut strip = Strip::new(switching(), now);
    let landed = now + Duration::from_millis(50);
    strip.land(progress(20, 20, 2).into(), landed);
    assert_eq!(strip.finish(landed), Finish::Done);
    assert_eq!(
        strip.shown(),
        Shown::Done(2),
        "it shows at once though it never had"
    );
    assert_eq!(done_label(1), "Done · 1 needs attention");
    assert_eq!(done_label(2), "Done · 2 need attention");
}

#[test]
fn a_refused_change_closes_without_a_toast() {
    let now = Instant::now();
    let mut strip = Strip::new(switching(), now);
    let revealed = now + REVEAL_AFTER;
    strip.reveal(revealed);
    strip.follow(Some(&progress(1, 4, 0)));
    assert_eq!(strip.fail(revealed), revealed + SHOWN_AT_LEAST);
    assert_eq!(strip.shown(), Shown::Working(25));
    assert_eq!(strip.finish(revealed + SHOWN_AT_LEAST), Finish::Closed);
}

#[test]
fn labels_and_toasts_name_each_change() {
    let deleting = Activity::DeletingActive {
        preset: "Frontend".to_owned(),
    };
    let importing = Activity::Importing { skills: 6 };
    let imported = |count| Landing {
        progress: progress(6, 6, 0),
        imported: count,
    };
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
        ..progress(2, 20, 0)
    }));
    assert_eq!(
        strip.tooltip(place),
        "Switching to Frontend\nCopying tailwind-helper → Cursor"
    );
    strip.follow(Some(&ActivationProgress {
        step: Some(step("release-notes", "/h/Claude Code", true)),
        ..progress(3, 20, 0)
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
