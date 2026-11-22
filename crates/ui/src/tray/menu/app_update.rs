//! The app update group (spec rule 13): a line for the app update state, which acts where the
//! user can. Its labels always say "app update" or name SkillBinder, since "update" alone means a
//! skill update (A1).

use super::{TrayAction, TrayItem};
use crate::gate::Gate;
use crate::native_view::RestartBlocker;
use skillbinder_proto::{AppSnapshot, AppUpdateState};

const CHECK: &str = "Check for app updates";

/// The group for `snapshot` while `gate` holds. Restart is disabled while one of its blockers
/// runs, as in Settings.
pub(super) fn app_update(snapshot: &AppSnapshot, gate: &Gate) -> Vec<TrayItem> {
    let update = &snapshot.app_update;
    let status = |label: String| TrayItem::Label {
        id: "app-update",
        label,
    };
    let check = |label: &str| TrayItem::action(TrayAction::CheckAppUpdate, label);
    match &update.state {
        AppUpdateState::Off => vec![status("App updates are off in this build".into())],
        AppUpdateState::Idle {
            last_failure: Some(_),
        } => vec![check("Couldn't check for app updates · Try again")],
        // Only a check that found nothing claims the running version is up to date (A4).
        AppUpdateState::Idle { last_failure: None } => match update.last_checked {
            Some(_) => vec![
                status(format!("SkillBinder {} is up to date", update.current)),
                check(CHECK),
            ],
            None => vec![check(CHECK)],
        },
        AppUpdateState::Checking => vec![status("Checking for app updates…".into())],
        AppUpdateState::Downloading { version, percent } => {
            let percent = percent.map(|percent| format!(" {percent}%"));
            vec![status(format!(
                "Downloading SkillBinder {version}…{}",
                percent.unwrap_or_default()
            ))]
        }
        AppUpdateState::Ready { version, .. } => vec![TrayItem::Action {
            action: TrayAction::Restart,
            label: format!("Restart to install SkillBinder {version}"),
            enabled: RestartBlocker::of(snapshot, gate.blocked(snapshot)).is_none(),
            checked: None,
            shortcut: None,
        }],
        AppUpdateState::DownloadOnly { version, .. } => vec![TrayItem::action(
            TrayAction::DownloadAppUpdate,
            format!("Download SkillBinder {version}…"),
        )],
    }
}
