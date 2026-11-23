//! Skill updates and Sync: Library's Updates filter, Check now, Update, Update all, and
//! Reinstall, then Sync, Push, Pull, and the sync remote.

use super::{flag, name, unknown, value, values};
use clap::{Arg, ArgGroup, ArgMatches, Command};
use skillbinder_proto::command_line::{Change, CommandError, SyncKind};

pub(super) fn updates() -> Command {
    Command::new("updates")
        .about("List the skills whose source has a newer version, as of the last check")
        .arg(flag(
            "check",
            "Check every source now first, as Check now does",
        ))
        .after_help("Example: skillbinder updates --check")
}

pub(super) fn update() -> Command {
    Command::new("update")
        .about("Update skills from their sources")
        .arg(
            Arg::new("slugs")
                .value_name("SLUG")
                .num_args(1..)
                .help("The skills, by slug"),
        )
        .arg(flag(
            "all",
            "Every skill with an update, as Update all does",
        ))
        .group(
            ArgGroup::new("skills")
                .args(["slugs", "all"])
                .required(true),
        )
        .after_help("Example: skillbinder update --all")
}

pub(super) fn reinstall() -> Command {
    Command::new("reinstall")
        .about("Restore a skill changed outside SkillBinder to the content its record holds")
        .arg(name("slug", "The skill's slug"))
        .after_help("Example: skillbinder reinstall pdf")
}

pub(super) fn sync() -> [Command; 3] {
    [
        Command::new("sync")
            .about("Pull the remote's changes, then push the library's")
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

/// `update`'s skills, or Update all.
pub(super) fn update_change(args: &ArgMatches) -> Change {
    match values(args, "slugs").as_slice() {
        [] => Change::UpdateAll,
        skills => Change::Update {
            skills: skills.to_vec(),
        },
    }
}

/// `sync`, `push`, or `pull`.
pub(super) fn sync_change(command: &str) -> Change {
    let action = match command {
        "push" => SyncKind::Push,
        "pull" => SyncKind::Pull,
        _ => SyncKind::Sync,
    };
    Change::Synchronize { action }
}

pub(super) fn remote_change(matches: &ArgMatches) -> Result<Change, CommandError> {
    match matches.subcommand() {
        Some(("set", args)) => Ok(Change::SetRemote {
            url: value(args, "url"),
            branch: value(args, "branch"),
        }),
        Some(("remove", _)) => Ok(Change::RemoveRemote),
        _ => Err(unknown("remote")),
    }
}
