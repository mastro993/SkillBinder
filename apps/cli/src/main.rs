//! The `skillbinder` command (ADR 0022). It never opens the library itself: it sends one request
//! to the running app over the command socket, starting the app in the background when none
//! answers, and prints the reply as text, or with `--json` as exactly one JSON document.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod connect;
mod render;
mod start;

use clap::error::ErrorKind;
use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use skillbinder_proto::command_line::{CommandError, ErrorCode, Request};
use std::path::PathBuf;
use std::process::ExitCode;

/// The grammar. Built rather than derived: the derive allows Clippy's style lints, which the
/// workspace forbids allowing.
fn grammar() -> Command {
    // The packaged file is `skillbinder-cli`, on the PATH as `skillbinder`.
    Command::new("skillbinder")
        .bin_name("skillbinder")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Run SkillBinder from a terminal. It talks to the running app, and starts it when it isn't.")
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print exactly one JSON document on stdout"),
        )
        // An isolated root's app, for tests against a native-test build.
        .arg(
            Arg::new("data-dir")
                .long("data-dir")
                .global(true)
                .hide(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .subcommand(
            Command::new("status")
                .about("Show the version, the active preset, the library's counts, and what is running")
                .after_help("Example: skillbinder status --json"),
        )
}

fn main() -> ExitCode {
    let matches = match grammar().try_get_matches() {
        Ok(matches) => matches,
        Err(error) => return usage(&error),
    };
    let json = matches.get_flag("json");
    let request = match request(&matches) {
        Ok(request) => request,
        Err(error) => return render::failure(&error, json),
    };
    let outcome = match matches.get_one::<PathBuf>("data-dir") {
        // An isolated root's app is a native-test build that needs the same root to start.
        Some(data) => connect::send(data, &request, true),
        None => platform_data().and_then(|data| connect::send(&data, &request, false)),
    };
    render::outcome(outcome, json)
}

fn request(matches: &ArgMatches) -> Result<Request, CommandError> {
    match matches.subcommand_name() {
        Some("status") => Ok(Request::Status),
        name => Err(CommandError::new(
            ErrorCode::Usage,
            format!("Unknown command {}", name.unwrap_or_default()),
        )),
    }
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
        return render::failure(&CommandError::new(ErrorCode::Usage, message), true);
    }
    let _ = error.print();
    ExitCode::from(u8::try_from(error.exit_code()).unwrap_or(2))
}
