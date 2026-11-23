//! The `skillbinder` command (ADR 0022). It never opens the library itself: it sends one request
//! to the running app over the command socket, starting the app in the background when none
//! answers, and prints the reply as text, or with `--json` as exactly one JSON document.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod connect;
mod console;
mod grammar;
mod render;
mod start;

use clap::error::ErrorKind;
use console::Console;
use skillbinder_proto::command_line::{CommandError, ErrorCode};
use std::io::IsTerminal as _;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let matches = match grammar::grammar().try_get_matches() {
        Ok(matches) => matches,
        Err(error) => return usage(&error),
    };
    let json = matches.get_flag("json");
    // Questions need someone to answer them: no prompts without a terminal (D3).
    let interactive = std::io::stdin().is_terminal() && std::io::stderr().is_terminal();
    let request = match grammar::request(&matches, interactive) {
        Ok(request) => request,
        Err(error) => return render::failure(&error.into(), json),
    };
    let mut console = Console::new(std::io::stderr().is_terminal());
    let outcome = match matches.get_one::<PathBuf>("data-dir") {
        // An isolated root's app is a native-test build that needs the same root to start.
        Some(data) => connect::send(data, &request, true, &mut console),
        None => platform_data()
            .map_err(Into::into)
            .and_then(|data| connect::send(&data, &request, false, &mut console)),
    };
    console.end_progress();
    render::outcome(outcome, json)
}

/// The application data folder the app's engine uses.
fn platform_data() -> Result<PathBuf, CommandError> {
    directories::BaseDirs::new()
        .map(|dirs| dirs.data_local_dir().join("SkillBinder"))
        .ok_or_else(|| {
            CommandError::new(
                ErrorCode::Unavailable,
                "SkillBinder's application data can't be found: there is no home folder.",
            )
        })
}

/// Help and the version print as text; a wrong command line prints as one JSON document with
/// `--json`, and exits 2 either way.
fn usage(error: &clap::Error) -> ExitCode {
    let shown = matches!(
        error.kind(),
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    );
    if !shown && std::env::args_os().any(|argument| argument == "--json") {
        let message = error
            .kind()
            .as_str()
            .unwrap_or("The command line is not valid");
        let error = CommandError::new(ErrorCode::Usage, message);
        return render::failure(&error.into(), true);
    }
    let _ = error.print();
    ExitCode::from(u8::try_from(error.exit_code()).unwrap_or(2))
}
