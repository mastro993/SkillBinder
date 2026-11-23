use super::super::catalog::{Group, Section, SettingRow, search};
use super::{Control, INVITE, KEYWORDS, row};
use skillbinder_proto::{CannotInstallCommand, CommandLineState, CommandLineStatus};
use std::path::PathBuf;

fn line(state: CommandLineState) -> (String, Control) {
    row(&CommandLineStatus {
        state,
        path: PathBuf::from("/usr/local/bin/skillbinder"),
    })
}

#[test]
fn each_state_has_the_spec_tables_line_and_control() {
    let cases = [
        (CommandLineState::NotInstalled, INVITE, Control::Install),
        (
            CommandLineState::Installed,
            "skillbinder in /usr/local/bin",
            Control::Uninstall,
        ),
        (
            CommandLineState::Homebrew,
            "skillbinder in /usr/local/bin",
            Control::Homebrew,
        ),
        (
            CommandLineState::OtherApp,
            "skillbinder opens another copy of SkillBinder.",
            Control::Repair,
        ),
        (
            CommandLineState::NotOnPath,
            "Add ~/.local/bin to your PATH:",
            Control::Uninstall,
        ),
        (
            CommandLineState::InTheWay,
            "Another skillbinder is in /usr/local/bin.",
            Control::None,
        ),
        (
            CommandLineState::CannotInstall(CannotInstallCommand::OutsideApplications),
            "Move SkillBinder to Applications to install it.",
            Control::None,
        ),
        (
            CommandLineState::CannotInstall(CannotInstallCommand::NotSetUp),
            "Install SkillBinder with its setup program to add it.",
            Control::None,
        ),
    ];
    for (state, text, control) in cases {
        assert_eq!(line(state), (text.to_owned(), control), "{state:?}");
    }
}

#[test]
fn search_finds_the_row_by_terminal_cli_command_and_path() {
    for query in ["terminal", "CLI", "command", "PATH"] {
        let rows = vec![
            SettingRow::new("settings-command-line-row", "Command line tool")
                .detail(INVITE)
                .keywords(KEYWORDS),
        ];
        let found = search(Section::General, vec![Group::new(rows)], query);
        assert_eq!(found.len(), 1, "{query}");
    }
}
