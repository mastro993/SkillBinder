use super::{rows, snapshot, switching};
use crate::gate::{ChangeState, Gate};
use crate::tray::menu::TrayMenu;
use skillbinder_proto::{
    AppSnapshot, AppUpdateState, AppVersion, CannotInstall, CheckFailure, InstallState, SyncAction,
};

const IDS: [&str; 4] = [
    "app-update",
    "check-app-update",
    "restart",
    "download-app-update",
];

fn version() -> AppVersion {
    AppVersion {
        major: 0,
        minor: 2,
        patch: 0,
        beta: None,
    }
}

fn notes_url() -> String {
    "https://example.invalid/notes".into()
}

fn at(state: AppUpdateState) -> AppSnapshot {
    let mut snapshot = snapshot();
    snapshot.app_update.state = state;
    snapshot
}

fn ready() -> AppSnapshot {
    at(AppUpdateState::Ready {
        version: version(),
        notes_url: notes_url(),
    })
}

/// The app update group's rows, as [`rows`] shows them.
fn group(snapshot: &AppSnapshot, gate: &Gate) -> Vec<String> {
    let menu = TrayMenu::new(snapshot, gate);
    menu.items
        .iter()
        .zip(rows(&menu))
        .filter(|(item, _)| IDS.contains(&item.id().as_str()))
        .map(|(_, row)| row)
        .collect()
}

#[test]
fn each_app_update_state_shows_its_label_and_whether_it_acts() {
    let idle = |last_failure| AppUpdateState::Idle { last_failure };
    let mut checked = at(idle(None));
    checked.app_update.last_checked = Some(1_800_000_000);
    let cases = [
        (
            at(AppUpdateState::Off),
            vec!["# App updates are off in this build"],
        ),
        (
            checked,
            vec!["# SkillBinder 0.1.0 is up to date", "Check for app updates"],
        ),
        (at(idle(None)), vec!["Check for app updates"]),
        (
            at(idle(Some(CheckFailure::Offline))),
            vec!["Couldn't check for app updates · Try again"],
        ),
        (
            at(idle(Some(CheckFailure::Failed))),
            vec!["Couldn't check for app updates · Try again"],
        ),
        (
            at(AppUpdateState::Checking),
            vec!["# Checking for app updates…"],
        ),
        (
            at(AppUpdateState::Downloading {
                version: version(),
                percent: Some(42),
            }),
            vec!["# Downloading SkillBinder 0.2.0… 42%"],
        ),
        (
            at(AppUpdateState::Downloading {
                version: version(),
                percent: None,
            }),
            vec!["# Downloading SkillBinder 0.2.0…"],
        ),
        (ready(), vec!["Restart to install SkillBinder 0.2.0"]),
        (
            at(AppUpdateState::DownloadOnly {
                version: version(),
                notes_url: notes_url(),
                reason: CannotInstall::OutsideApplications,
            }),
            vec!["Download SkillBinder 0.2.0…"],
        ),
    ];
    for (snapshot, expected) in cases {
        assert_eq!(
            group(&snapshot, &Gate::default()),
            expected,
            "{:?}",
            snapshot.app_update.state
        );
    }
}

#[test]
fn restart_is_disabled_while_one_of_its_blockers_runs() {
    let off = vec!["Restart to install SkillBinder 0.2.0 (off)"];
    let mut updating = ready();
    updating.updating = Some((0, 2));
    let mut syncing = ready();
    syncing.syncing = Some(SyncAction::Push);
    let mut installing = ready();
    installing
        .explore
        .installs
        .insert("catalog/skill".into(), InstallState::Installing);
    for snapshot in [updating, syncing, installing] {
        assert_eq!(group(&snapshot, &Gate::default()), off);
    }
    let busy = Gate {
        busy: true,
        ..Gate::default()
    };
    assert_eq!(group(&ready(), &busy), off);
    assert_eq!(group(&ready(), &switching(ChangeState::Running)), off);
}
