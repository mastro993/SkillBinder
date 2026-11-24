use super::super::offers::Install;
use super::{landed, line};
use skillbinder_design::Status;
use skillbinder_proto::{CommandLineState, CommandLineStatus};
use std::path::PathBuf;

#[test]
fn the_line_follows_install_then_the_commands_state() {
    let failed = Install::Failed("Couldn't install `skillbinder`: denied".to_owned());
    assert_eq!(
        line(&failed, Some(CommandLineState::NotInstalled)),
        Some((
            "Couldn't install `skillbinder`: denied".to_owned(),
            Some(Status::Danger)
        ))
    );
    let (running, tone) = line(&Install::Running, None).unwrap_or_default();
    assert!(running == "Installing…" || running.contains("macOS prompt"));
    assert_eq!(tone, None);
    let (ready, tone) = line(&Install::Idle, Some(CommandLineState::Installed)).unwrap_or_default();
    assert!(ready.starts_with("skillbinder is ready"));
    assert_eq!(tone, Some(Status::Success));
    assert_eq!(
        line(&Install::Idle, Some(CommandLineState::NotOnPath)),
        Some(("Add ~/.local/bin to your PATH:".to_owned(), None))
    );
    // Nothing to say yet: the step says where Install puts the command instead.
    assert_eq!(
        line(&Install::Idle, Some(CommandLineState::NotInstalled)),
        None
    );
    assert_eq!(line(&Install::Idle, None), None);
}

#[test]
fn only_a_command_in_place_has_landed() {
    let status = |state| CommandLineStatus {
        state,
        path: PathBuf::from("/usr/local/bin/skillbinder"),
    };
    assert!(landed(&status(CommandLineState::Installed)));
    assert!(landed(&status(CommandLineState::NotOnPath)));
    assert!(!landed(&status(CommandLineState::NotInstalled)));
    assert!(!landed(&status(CommandLineState::OtherApp)));
}
