use super::{Action, Actions, GitFacts, GitSetup, Status, Waiting};
use skillbinder_proto::GitInstall;

fn facts(install: &GitInstall) -> GitFacts<'_> {
    GitFacts {
        found: None,
        outdated: None,
        install,
        failed: false,
    }
}

#[test]
fn install_starts_the_installer_again_while_download_and_copy_wait_only_when_idle() {
    let cases = [
        (None, Action::Install, Some(Waiting::Installer)),
        (
            Some(Waiting::Installer),
            Action::Install,
            Some(Waiting::Installer),
        ),
        (
            Some(Waiting::Check),
            Action::Install,
            Some(Waiting::Installer),
        ),
        (None, Action::Download, Some(Waiting::Check)),
        (None, Action::Copy, Some(Waiting::Check)),
        (
            Some(Waiting::Installer),
            Action::Download,
            Some(Waiting::Installer),
        ),
        (
            Some(Waiting::Installer),
            Action::Copy,
            Some(Waiting::Installer),
        ),
    ];
    for (waiting, action, expected) in cases {
        let mut setup = GitSetup { waiting };
        setup.act(action);
        assert_eq!(setup.waiting, expected, "{waiting:?} then {action:?}");
    }
}

#[test]
fn the_status_line_follows_what_the_app_waits_on_and_what_it_found() {
    let apple = GitInstall::AppleInstaller;
    let winget = GitInstall::Winget;
    let failed = GitFacts {
        failed: true,
        ..facts(&winget)
    };
    let outdated = GitFacts {
        outdated: Some("2.30.1"),
        ..facts(&apple)
    };
    let found = GitFacts {
        found: Some("2.47.1"),
        ..outdated
    };
    let cases = [
        (facts(&apple), None, None),
        (
            facts(&apple),
            Some(Waiting::Installer),
            Some(Status::Waiting("Finish in Apple's installer.")),
        ),
        (
            facts(&winget),
            Some(Waiting::Installer),
            Some(Status::Waiting("Installing Git…")),
        ),
        (
            facts(&apple),
            Some(Waiting::Check),
            Some(Status::Waiting("Checking for Git…")),
        ),
        // winget's failure ends its wait; Download Git after it waits on the check.
        (failed, Some(Waiting::Installer), Some(Status::Failed)),
        (failed, None, Some(Status::Failed)),
        (
            failed,
            Some(Waiting::Check),
            Some(Status::Waiting("Checking for Git…")),
        ),
        (outdated, None, Some(Status::TooOld("2.30.1".into()))),
        (
            outdated,
            Some(Waiting::Installer),
            Some(Status::Waiting("Finish in Apple's installer.")),
        ),
        (
            found,
            Some(Waiting::Installer),
            Some(Status::Ready("2.47.1".into())),
        ),
    ];
    for (facts, waiting, expected) in cases {
        assert_eq!(facts.status(waiting), expected, "{facts:?} {waiting:?}");
    }
}

#[test]
fn the_status_lines_read_as_designed() {
    let cases = [
        (Status::Waiting("Installing Git…"), "Installing Git…"),
        (Status::Failed, "Couldn't install with winget."),
        (Status::Ready("2.47.1".into()), "Git 2.47.1 is ready"),
        (
            Status::TooOld("2.30.1".into()),
            "Git 2.30.1 is too old. SkillBinder needs 2.39 or newer.",
        ),
    ];
    for (status, text) in cases {
        assert_eq!(status.text(), text);
    }
}

#[test]
fn a_git_too_old_titles_the_step_update_git() {
    let apple = GitInstall::AppleInstaller;
    assert_eq!(facts(&apple).title(), "Install Git");
    let outdated = GitFacts {
        outdated: Some("2.30.1"),
        ..facts(&apple)
    };
    assert_eq!(outdated.title(), "Update Git");
}

#[test]
fn each_offer_shows_its_actions() {
    let command = GitInstall::Command("sudo apt install git".into());
    let install = Actions {
        install: true,
        command: None,
        download_primary: false,
    };
    let download = Actions {
        install: false,
        command: None,
        download_primary: true,
    };
    let cases = [
        (GitInstall::AppleInstaller, false, install),
        (GitInstall::Winget, false, install),
        (GitInstall::Winget, true, download),
        (GitInstall::DownloadOnly, false, download),
    ];
    for (offer, failed, expected) in cases {
        let facts = GitFacts {
            failed,
            ..facts(&offer)
        };
        assert_eq!(facts.actions(), expected, "{offer:?} failed {failed}");
    }
    assert_eq!(
        facts(&command).actions(),
        Actions {
            install: false,
            command: Some("sudo apt install git"),
            download_primary: false,
        }
    );
}
