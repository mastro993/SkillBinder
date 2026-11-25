//! The `skillbinder` command (ADR 0022). It never opens the library itself: it sends one request
//! to the running app over the command socket, starting the app in the background when none
//! answers, and prints the reply as text, or with `--json` as exactly one JSON document.
#![allow(clippy::print_stdout, clippy::print_stderr)]

mod connect;
mod console;
mod grammar;
mod mcp;
mod render;
mod start;

use clap::error::ErrorKind;
use console::Console;
use skillbinder_proto::command_line::{CommandError, ErrorCode};
use std::io::IsTerminal as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let matches = match grammar::grammar().try_get_matches() {
        Ok(matches) => matches,
        Err(error) => return usage(&error),
    };
    let data = match matches.get_one::<PathBuf>("data-dir") {
        // An isolated root's app is a native-test build that needs the same root to start.
        Some(data) => Ok((isolated_data(data), true)),
        None => platform_data().map(|data| (data, false)),
    };
    if matches.subcommand_name() == Some("mcp") {
        return match data {
            Ok((data, isolated)) => mcp::serve(data, isolated),
            Err(error) => render::failure(&error.into(), false),
        };
    }
    let json = matches.get_flag("json");
    // Questions need someone to answer them: no prompts without a terminal (D3).
    let interactive = std::io::stdin().is_terminal() && std::io::stderr().is_terminal();
    let request = match grammar::request(&matches, interactive) {
        Ok(request) => request,
        Err(error) => return render::failure(&error.into(), json),
    };
    let mut console = Console::new(std::io::stderr().is_terminal());
    let outcome = data
        .map_err(Into::into)
        .and_then(|(data, isolated)| connect::send(&data, &request, isolated, &mut console));
    console.end_progress();
    render::outcome(outcome, json)
}

/// An isolated root resolved as the native-test app resolves it, since the endpoint derives from
/// the path: `.skillbinder-dev/default/data` reaches the app launched on its absolute path.
fn isolated_data(data: &Path) -> PathBuf {
    std::fs::canonicalize(data)
        .or_else(|_| std::path::absolute(data))
        .unwrap_or_else(|_| data.to_owned())
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

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_proto::command_line::Endpoint;

    #[test]
    fn a_relative_isolated_root_reaches_the_app_on_its_resolved_path() -> std::io::Result<()> {
        let resolved = std::fs::canonicalize(std::env::current_dir()?)?;
        let private = Path::new("private");
        assert_eq!(
            Endpoint::of(&isolated_data(Path::new(".")), private),
            Endpoint::of(&resolved, private)
        );
        Ok(())
    }
}
