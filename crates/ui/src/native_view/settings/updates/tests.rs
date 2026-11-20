use super::{Action, DOWNLOAD_LINE, Status, channel_line, status};
use skillbinder_proto::{AppUpdateSnapshot, AppUpdateState, Channel, CheckFailure};

/// 2027-01-15T08:00:00Z.
const NOW: u64 = 1_800_000_000;

fn update(current: &str, channel: Channel, state: AppUpdateState) -> AppUpdateSnapshot {
    AppUpdateSnapshot {
        current: current
            .parse()
            .unwrap_or_else(|_| panic!("{current} parses")),
        channel,
        auto_check: true,
        state,
        last_checked: None,
    }
}

fn line(update: &AppUpdateSnapshot) -> (String, Action) {
    let Status { line, action, .. } = status(update, NOW);
    (line, action)
}

#[test]
fn every_state_reads_its_row_of_rule_18() {
    let check_now = Action::CheckNow { enabled: true };
    let idle = |last_failure| AppUpdateState::Idle { last_failure };
    let at = |state| update("0.1.0", Channel::Stable, state);
    assert_eq!(
        line(&at(AppUpdateState::Off)),
        ("Updates are off in this build.".to_owned(), Action::None)
    );
    assert_eq!(
        line(&at(idle(None))),
        ("Not checked yet".to_owned(), check_now.clone())
    );
    let checked = AppUpdateSnapshot {
        last_checked: Some(NOW - 300),
        ..at(idle(None))
    };
    assert_eq!(
        line(&checked),
        (
            "Up to date · checked 5 minutes ago".to_owned(),
            check_now.clone()
        )
    );
    let stale = AppUpdateSnapshot {
        auto_check: false,
        last_checked: Some(NOW - 3 * 86_400),
        ..at(idle(None))
    };
    assert_eq!(line(&stale).0, "Up to date · checked 3 days ago");
    let checking = status(&at(AppUpdateState::Checking), NOW);
    assert_eq!(checking.line, "Checking…");
    assert!(checking.spinner);
    assert_eq!(checking.action, Action::CheckNow { enabled: false });
    assert_eq!(
        line(&at(idle(Some(CheckFailure::Offline)))),
        (
            "Couldn't check for updates. Check your connection.".to_owned(),
            check_now.clone()
        )
    );
    assert_eq!(
        line(&at(idle(Some(CheckFailure::Failed)))),
        ("Couldn't check for updates.".to_owned(), check_now)
    );
    let offered = status(
        &at(AppUpdateState::DownloadOnly {
            version: "0.2.0".parse().unwrap_or_else(|_| panic!("0.2.0 parses")),
            notes_url: "https://example.invalid/v0.2.0".to_owned(),
        }),
        NOW,
    );
    assert_eq!(offered.line, "SkillBinder 0.2.0 is available.");
    assert_eq!(offered.detail, Some(DOWNLOAD_LINE));
    assert_eq!(
        offered.action,
        Action::Download {
            notes_url: "https://example.invalid/v0.2.0".to_owned()
        }
    );
}

#[test]
fn a_beta_build_on_stable_says_when_it_moves() {
    let idle = AppUpdateState::Idle { last_failure: None };
    assert_eq!(
        channel_line(&update("0.1.1-beta.12", Channel::Stable, idle.clone())).as_deref(),
        Some("You'll move to Stable at its first release after 0.1.1-beta.12.")
    );
    assert_eq!(
        channel_line(&update("0.1.1-beta.12", Channel::Beta, idle.clone())),
        None
    );
    assert_eq!(channel_line(&update("0.1.0", Channel::Stable, idle)), None);
}
