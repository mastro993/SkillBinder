use super::*;
use skillbinder_proto::{AppError, RemoteConfig};

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

fn shown(sync: &SyncSnapshot) -> (&'static str, Tint, String, String, Option<SyncAction>) {
    let Some(Status {
        glyph,
        tint,
        title,
        line,
        main,
    }) = status(sync, now())
    else {
        panic!("{:?} shows the setup column", sync.state);
    };
    (glyph, tint, title, line, main)
}

#[test]
fn each_state_shows_its_glyph_title_line_and_one_next_action() {
    let mut push = sync(SyncState::NeedsPush, 0);
    push.local_skills = 3;
    let mut pull = sync(SyncState::NeedsPull, 0);
    pull.incoming_skills = 1;
    let mut both = sync(SyncState::NeedsSync, 0);
    (both.local_skills, both.incoming_skills) = (3, 2);
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
                Some(SyncAction::Push),
            ),
        ),
        (
            pull,
            (
                "download",
                Tint::Accent,
                s("1 skill changed on the remote"),
                s("Not pulled yet"),
                Some(SyncAction::Pull),
            ),
        ),
        (
            both,
            (
                "git-merge",
                Tint::Accent,
                s("Changes on both sides"),
                s("3 here, 2 from the remote"),
                Some(SyncAction::Sync),
            ),
        ),
        (
            unreachable,
            (
                "alert-circle",
                Tint::Danger,
                s("Can't reach the remote"),
                s("Last checked 2 hours ago · Could not resolve host."),
                Some(SyncAction::Refresh),
            ),
        ),
    ];
    for (sync, expected) in cases {
        assert_eq!(shown(&sync), expected, "{:?}", sync.state);
    }
    assert_eq!(status(&SyncSnapshot::default(), now()), None);
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
        assert_eq!(main_label(action), label);
        assert_eq!(running(action), running_label);
        assert_eq!(done(action, remote).as_deref(), toast);
    }
}
