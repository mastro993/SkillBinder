//! Sync: Sync, Push, and Pull, what waits on each side, History, Restore, Discard, Resolve…, the
//! sync remote, and this device's name.

use super::{name, unknown, value, values};
use clap::{Arg, ArgMatches, Command, value_parser};
use skillbinder_proto::Side;
use skillbinder_proto::command_line::{
    Change, CommandError, Consent, ErrorCode, Find, Query, Request, SyncCommand, SyncKind,
};
use std::collections::BTreeMap;

pub(super) fn sync() -> [Command; 3] {
    [
        Command::new("sync")
            .about("Pull the remote's changes, then push the library's")
            .subcommand(
                Command::new("status")
                    .about("List the skills not synced yet on each side, and those changed on both"),
            )
            .subcommand(
                Command::new("history")
                    .about("List the library's changes, newest first, with the device that made each")
                    .arg(
                        Arg::new("limit")
                            .long("limit")
                            .value_name("N")
                            .value_parser(value_parser!(u64).range(1..))
                            .help("The most changes listed; 50 unless named"),
                    )
                    .arg(
                        Arg::new("before")
                            .long("before")
                            .value_name("C")
                            .help("Only changes older than this commit"),
                    )
                    .after_help("Example: skillbinder sync history --limit 10"),
            )
            .subcommand(
                Command::new("restore")
                    .about("Put a skill back to its version in a change History lists")
                    .arg(name("slug", "The skill's slug"))
                    .arg(
                        Arg::new("to")
                            .long("to")
                            .value_name("COMMIT")
                            .required(true)
                            .help("The change's commit, as `sync history` lists it"),
                    )
                    .after_help("Example: skillbinder sync restore pdf --to 3f2a9c1b"),
            )
            .subcommand(
                Command::new("discard")
                    .about("Undo this device's change to a skill not pushed yet")
                    .arg(name("slug", "The skill's slug")),
            )
            .subcommand(
                Command::new("resolve")
                    .about("Sync, keeping this device's version (mine) or the remote's (theirs) of each skill changed on both sides")
                    .arg(
                        Arg::new("choices")
                            .value_name("SLUG=mine|theirs")
                            .required(true)
                            .num_args(1..)
                            .help("The version kept of each skill changed on both sides"),
                    )
                    .after_help("Example: skillbinder sync resolve pdf=mine sql=theirs"),
            )
            .after_help("Example: skillbinder sync"),
        Command::new("push").about("Commit the library's changes and push them to the remote"),
        Command::new("pull").about("Bring in the remote's changes"),
    ]
}

pub(super) fn remote() -> Command {
    Command::new("remote")
        .about("Set or remove the Git remote the library syncs with")
        .subcommand_required(true)
        .subcommand(
            Command::new("set")
                .about("Connect the library to a Git remote, replacing the one set")
                .arg(name("url", "The remote's Git URL"))
                .arg(
                    Arg::new("branch")
                        .long("branch")
                        .value_name("B")
                        .default_value("main")
                        .help("The remote's branch"),
                )
                .after_help("Example: skillbinder remote set git@github.com:me/skills.git"),
        )
        .subcommand(
            Command::new("remove").about("Disconnect the remote; the library stays as it is"),
        )
}

pub(super) fn device() -> Command {
    Command::new("device")
        .about("Rename this device, as the changes it pushes name it")
        .subcommand_required(true)
        .subcommand(
            Command::new("rename")
                .about("Rename this device; changes pushed from now on carry the new name")
                .arg(name("name", "The new name, quoted when it has spaces"))
                .after_help("Example: skillbinder device rename \"Work laptop\""),
        )
}

/// `sync` and its subcommands, `push`, or `pull`.
pub(super) fn sync_request(
    command: &str,
    args: &ArgMatches,
    consent: Consent,
) -> Result<Request, CommandError> {
    let change = |sync| {
        Ok(Request::Change {
            change: Change::Sync { sync },
            consent,
        })
    };
    let synchronize = |action| change(SyncCommand::Run { action });
    match (command, args.subcommand()) {
        ("push", _) => synchronize(SyncKind::Push),
        ("pull", _) => synchronize(SyncKind::Pull),
        (_, None) => synchronize(SyncKind::Sync),
        (_, Some(("status", _))) => Ok(Request::Read {
            query: Query::SyncStatus,
        }),
        (_, Some(("history", args))) => Ok(Request::Find {
            find: Find::History {
                before: args.get_one::<String>("before").cloned(),
                limit: args
                    .get_one::<u64>("limit")
                    .map(|limit| usize::try_from(*limit).unwrap_or(usize::MAX)),
            },
        }),
        (_, Some(("restore", args))) => change(SyncCommand::Restore {
            skill: value(args, "slug"),
            to: value(args, "to"),
        }),
        (_, Some(("discard", args))) => change(SyncCommand::Discard {
            skill: value(args, "slug"),
        }),
        (_, Some(("resolve", args))) => change(SyncCommand::Resolve {
            choices: choices(&values(args, "choices"))?,
        }),
        (_, Some((other, _))) => Err(unknown(other)),
    }
}

/// Each `<slug>=mine|theirs`, mine being this device's version and theirs the remote's.
fn choices(written: &[String]) -> Result<BTreeMap<String, Side>, CommandError> {
    let mut choices = BTreeMap::new();
    for choice in written {
        let (slug, side) = choice.rsplit_once('=').ok_or_else(|| {
            CommandError::new(
                ErrorCode::Usage,
                format!("Write {choice} as {choice}=mine or {choice}=theirs."),
            )
        })?;
        let side = match side.trim().to_lowercase().as_str() {
            "mine" => Side::Local,
            "theirs" => Side::Remote,
            _ => {
                return Err(CommandError::new(
                    ErrorCode::Usage,
                    format!("Choose mine or theirs for {slug}, as {slug}=mine."),
                ));
            }
        };
        let slug = slug.trim().to_owned();
        if choices
            .insert(slug.clone(), side)
            .is_some_and(|was| was != side)
        {
            return Err(CommandError::new(
                ErrorCode::Usage,
                format!("Choose one version of {slug}."),
            ));
        }
    }
    Ok(choices)
}

/// `remote set` or `remote remove`.
pub(super) fn remote_change(matches: &ArgMatches) -> Result<Change, CommandError> {
    let sync = match matches.subcommand() {
        Some(("set", args)) => SyncCommand::SetRemote {
            url: value(args, "url"),
            branch: value(args, "branch"),
        },
        Some(("remove", _)) => SyncCommand::RemoveRemote,
        _ => return Err(unknown("remote")),
    };
    Ok(Change::Sync { sync })
}

/// `device rename`.
pub(super) fn device_change(matches: &ArgMatches) -> Result<Change, CommandError> {
    match matches.subcommand() {
        Some(("rename", args)) => Ok(Change::Sync {
            sync: SyncCommand::RenameDevice {
                name: value(args, "name"),
            },
        }),
        _ => Err(unknown("device")),
    }
}

#[cfg(test)]
mod tests;
