use super::*;
use skillbinder_proto::{AppError, Conflict, RemoteConfig};

/// `count` changed skills, outside SkillBinder.
fn changes(count: usize) -> Vec<ChangedSkill> {
    (0..count)
        .map(|n| ChangedSkill {
            slug: format!("skill-{n}"),
            kind: ChangeKind::Changed,
            reason: Some(Reason::Outside),
        })
        .collect()
}

/// `count` skills incoming from `devices`.
fn incoming(devices: &[&str], count: usize) -> Option<Incoming> {
    Some(Incoming {
        devices: devices.iter().map(|d| (*d).to_owned()).collect(),
        at: None,
        skills: changes(count),
    })
}

const NOW: i64 = 1_800_000_000;

fn now() -> DateTime<Utc> {
    DateTime::from_timestamp(NOW, 0).unwrap_or_default()
}

/// A connected library in `state`, checked `ago` seconds before [`NOW`].
fn sync(state: SyncState, ago: u64) -> SyncSnapshot {
    SyncSnapshot {
        remote: Some(RemoteConfig {
            url: "git@github.com:ada/skills.git".to_owned(),
            branch: "main".to_owned(),
        }),
        state,
        refreshed_at: Some(NOW.unsigned_abs() - ago),
        ..SyncSnapshot::default()
    }
}

fn shown(sync: &SyncSnapshot) -> (&'static str, Tint, String, String, Option<Next>) {
    shown_chosen(sync, false)
}

fn shown_chosen(
    sync: &SyncSnapshot,
    chosen: bool,
) -> (&'static str, Tint, String, String, Option<Next>) {
    let Some(Status {
        glyph,
        tint,
        title,
        line,
        main,
    }) = status(sync, chosen, now())
    else {
        panic!("{:?} shows the setup column", sync.state);
    };
    (glyph, tint, title, line, main)
}

#[test]
fn each_state_shows_its_glyph_title_line_and_one_next_action() {
    let mut push = sync(SyncState::NeedsPush, 0);
    push.local = changes(3);
    let mut pull = sync(SyncState::NeedsPull, 0);
    pull.incoming = incoming(&["MacBook Air"], 1);
    let mut both = sync(SyncState::NeedsSync, 0);
    (both.local, both.incoming) = (changes(3), incoming(&["MacBook Air", "Studio"], 2));
    let mut unnamed = sync(SyncState::NeedsPull, 0);
    unnamed.incoming = incoming(&[], 2);
    let mut unreachable = sync(SyncState::Unreachable, 7_200);
    unreachable.last_error = Some(AppError::validation("Could not resolve host."));
    let s = str::to_owned;
    let cases = [
        (
            sync(SyncState::Synced, 300),
            (
                "check-circle",
                Tint::Success,
                s("Up to date"),
                s("Checked 5 minutes ago"),
                None,
            ),
        ),
        (
            push,
            (
                "upload",
                Tint::Accent,
                s("3 skills changed on this device"),
                s("Not pushed yet"),
                Some(Next::Run(SyncAction::Push)),
            ),
        ),
        (
            pull,
            (
                "download",
                Tint::Accent,
                s("1 skill changed on MacBook Air"),
                s("Not pulled yet"),
                Some(Next::Run(SyncAction::Pull)),
            ),
        ),
        (
            unnamed,
            (
                "download",
                Tint::Accent,
                s("2 skills changed on the remote"),
                s("Not pulled yet"),
                Some(Next::Run(SyncAction::Pull)),
            ),
        ),
        (
            both,
            (
                "git-merge",
                Tint::Accent,
                s("Changes on both sides"),
                s("3 here, 2 from MacBook Air and 1 more · none on the same skill"),
                Some(Next::Run(SyncAction::Sync)),
            ),
        ),
        (
            unreachable,
            (
                "alert-circle",
                Tint::Danger,
                s("Can't reach the remote"),
                s("Last checked 2 hours ago · Could not resolve host."),
                Some(Next::Run(SyncAction::Refresh)),
            ),
        ),
    ];
    for (sync, expected) in cases {
        assert_eq!(shown(&sync), expected, "{:?}", sync.state);
    }
    assert_eq!(status(&SyncSnapshot::default(), false, now()), None);
}

fn side(device: Option<&str>, at: Option<u64>) -> ConflictSide {
    ConflictSide {
        kind: ChangeKind::Changed,
        reason: Some(Reason::Updated),
        device: device.map(str::to_owned),
        at,
    }
}

#[test]
fn skills_changed_on_both_sides_ask_for_a_choice_then_sync() {
    let mut conflict = sync(SyncState::NeedsResolve, 0);
    conflict.conflicts = ["review", "notes"]
        .map(|slug| Conflict {
            slug: slug.to_owned(),
            local: side(None, None),
            remote: side(Some("MacBook Air"), None),
        })
        .to_vec();
    let s = str::to_owned;
    assert_eq!(
        shown_chosen(&conflict, false),
        (
            "git-merge-conflict",
            Tint::Warning,
            s("2 skills changed on both sides"),
            s("Choose which version to keep"),
            Some(Next::Resolve),
        )
    );
    assert_eq!(main_label(Next::Resolve), "Resolve…");
    let (.., line, main) = shown_chosen(&conflict, true);
    assert_eq!(
        (line.as_str(), main),
        (
            "Choices made · Sync to finish",
            Some(Next::Run(SyncAction::Sync))
        )
    );
}

#[test]
fn resolve_names_each_side_with_why_and_when() {
    assert_eq!(
        resolve_intro("MacBook Air", 1),
        "This device and MacBook Air both changed this skill. Choose the version to keep. The \
         other is still in History."
    );
    assert!(resolve_intro("MacBook Air", 2).contains("both changed these skills."));
    let at = NOW.unsigned_abs() - 7_200;
    assert_eq!(
        side_card(Side::Local, &side(None, None), "Studio", now()),
        (
            s("This device (Studio)"),
            s("Updated from its source · Not pushed")
        )
    );
    assert_eq!(
        side_card(
            Side::Remote,
            &side(Some("MacBook Air"), Some(at)),
            "Studio",
            now()
        ),
        (s("MacBook Air"), s("Updated from its source · 2 hours ago"))
    );
    assert_eq!(
        merge_on_their_own(1),
        "1 other skill changed on only one side and merges on its own."
    );
    assert_eq!(
        merge_on_their_own(3),
        "3 other skills changed on only one side and merge on their own."
    );
    assert_eq!(chosen(Side::Local), "Keeping this device's");
    assert_eq!(chosen(Side::Remote), "Taking this one");
    assert_eq!(kept(Some("Laptop"), "review"), "Kept Laptop's review");
    assert_eq!(kept(None, "review"), "Kept the remote's review");
}

fn s(text: &str) -> String {
    text.to_owned()
}

#[test]
fn last_checked_reads_as_relative_time() {
    let line = |ago, refreshed: bool| {
        let mut synced = sync(SyncState::Synced, ago);
        if !refreshed {
            synced.refreshed_at = None;
        }
        shown(&synced).3
    };
    assert_eq!(line(10, true), "Checked just now");
    assert_eq!(line(60, true), "Checked 1 minute ago");
    assert_eq!(line(86_400 * 2, true), "Checked 2 days ago");
    assert_eq!(line(0, false), "Not checked yet");
    let mut never = sync(SyncState::Unreachable, 0);
    never.refreshed_at = None;
    assert_eq!(shown(&never).3, "The remote did not answer.");
}

#[test]
fn actions_read_as_they_run_and_end_with_a_toast_naming_the_remote() {
    let remote = "github.com/ada/skills";
    for (action, label, running_label, toast) in [
        (
            SyncAction::Push,
            "Push",
            "Pushing…",
            Some("Pushed to github.com/ada/skills"),
        ),
        (
            SyncAction::Pull,
            "Pull",
            "Pulling…",
            Some("Pulled from github.com/ada/skills"),
        ),
        (
            SyncAction::Sync,
            "Sync",
            "Syncing…",
            Some("Synced with github.com/ada/skills"),
        ),
        (SyncAction::Refresh, "Try again", "Checking…", None),
    ] {
        assert_eq!(main_label(Next::Run(action)), label);
        assert_eq!(running(action), running_label);
        assert_eq!(done(action, remote).as_deref(), toast);
    }
}

#[test]
fn each_skill_says_how_and_why_it_changed() {
    let why_of = |kind, reason| {
        why(&ChangedSkill {
            slug: "review".to_owned(),
            kind,
            reason,
        })
    };
    let from = Some("Claude Code".to_owned());
    for (kind, reason, expected) in [
        (
            ChangeKind::Added,
            Some(Reason::Imported { from }),
            Some("Imported from Claude Code"),
        ),
        (
            ChangeKind::Added,
            Some(Reason::Imported { from: None }),
            Some("Imported"),
        ),
        (
            ChangeKind::Added,
            Some(Reason::Installed),
            Some("Installed from Explore"),
        ),
        (
            ChangeKind::Changed,
            Some(Reason::Updated),
            Some("Updated from its source"),
        ),
        (
            ChangeKind::Changed,
            Some(Reason::Reinstalled),
            Some("Reinstalled"),
        ),
        (
            ChangeKind::Changed,
            Some(Reason::Record),
            Some("Record added"),
        ),
        (
            ChangeKind::Changed,
            Some(Reason::Outside),
            Some("Changed outside SkillBinder"),
        ),
        (
            ChangeKind::Removed,
            Some(Reason::Outside),
            Some("Removed outside SkillBinder"),
        ),
        (ChangeKind::Removed, None, None),
    ] {
        assert_eq!(why_of(kind, reason).as_deref(), expected);
    }
    let today = chrono::Local::now();
    let restored = Reason::Restored {
        at: today.timestamp().unsigned_abs(),
    };
    assert_eq!(
        why_of(ChangeKind::Changed, Some(restored)),
        Some(format!(
            "Restored to the version of {}",
            today.format("%b %-d")
        ))
    );
    assert_eq!(kind(ChangeKind::Added), ("+", "Added", Tint::Success));
    assert_eq!(kind(ChangeKind::Changed), ("~", "Changed", Tint::Accent));
    assert_eq!(kind(ChangeKind::Removed), ("−", "Removed", Tint::Danger));
}

#[test]
fn the_remote_s_group_says_when_its_newest_change_was_made() {
    let mut pulled = incoming(&["MacBook Air"], 1).unwrap_or_default();
    assert_eq!(not_pulled(&pulled, now()), "Not pulled");
    pulled.at = Some(NOW.unsigned_abs() - 10);
    assert_eq!(not_pulled(&pulled, now()), "Just now · Not pulled");
    pulled.at = Some(NOW.unsigned_abs() - 7_200);
    assert_eq!(not_pulled(&pulled, now()), "2 hours ago · Not pulled");
}

#[test]
fn a_restored_version_names_its_day_with_the_year_only_when_it_differs() {
    let day = |at: i64| version_day(at.unsigned_abs(), &now());
    // 2027-01-15, the year of NOW, then a day of the year before.
    assert_eq!(day(NOW), "Jan 15");
    assert_eq!(day(NOW - 86_400 * 365), "Jan 15, 2026");
}

#[test]
fn discard_and_restore_ask_what_they_do() {
    let skill = |kind| ChangedSkill {
        slug: "review".to_owned(),
        kind,
        reason: None,
    };
    let (title, added) = discard_question(&skill(ChangeKind::Added));
    assert_eq!(title, "Discard your change to review?");
    assert!(added.ends_with("Copies your agents have stay where they are."));
    let (_, removed) = discard_question(&skill(ChangeKind::Removed));
    assert!(removed.contains("in no preset"));
    assert_eq!(
        discard_tip(ChangeKind::Changed),
        "Go back to the last synced version"
    );
    let (title, _, confirm) = restore_question("review", false, "Wed, Nov 12", "09:15");
    assert_eq!(
        (title.as_str(), confirm),
        ("Restore review to Wed, Nov 12, 09:15?", "Restore")
    );
    let (title, _, confirm) = restore_question("review", true, "Today", "09:15");
    assert_eq!(
        (title.as_str(), confirm),
        ("Bring back review?", "Bring back")
    );
}
