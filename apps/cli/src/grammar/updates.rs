//! Skill updates: Library's Updates filter, Check now, Update, Update all, and Reinstall.

use super::{flag, name, values};
use clap::{Arg, ArgGroup, ArgMatches, Command};
use skillbinder_proto::command_line::{Change, Find};

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
        .arg(
            flag(
                "diff",
                "Print what updating the skills would change, as a patch, and change nothing",
            )
            .conflicts_with("all"),
        )
        .group(
            ArgGroup::new("skills")
                .args(["slugs", "all"])
                .required(true),
        )
        .after_help("Example: skillbinder update pdf --diff")
}

pub(super) fn reinstall() -> Command {
    Command::new("reinstall")
        .about("Restore a skill changed outside SkillBinder to the content its record holds")
        .arg(name("slug", "The skill's slug"))
        .after_help("Example: skillbinder reinstall pdf")
}

/// `update --diff`'s skills.
pub(super) fn diff_find(args: &ArgMatches) -> Find {
    Find::Diff {
        skills: values(args, "slugs"),
    }
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
