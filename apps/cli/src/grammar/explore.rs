//! Scan and import, Explore, and your repositories: Library's Import skills dialog with its
//! Ignore and Stop ignoring, Explore's search and Install, and Your repositories with Install all.

use super::{flag, has, name, replace, slugs, unknown, value, values};
use clap::{Arg, ArgGroup, ArgMatches, Command};
use skillbinder_proto::command_line::{Change, CommandError, Find};

pub(super) fn scan() -> Command {
    Command::new("scan")
        .about("Scan your agents and workspaces for skills not in the library, as Scan for skills does")
        .after_help("Example: skillbinder scan")
}

pub(super) fn import() -> Command {
    Command::new("import")
        .about("Import found skills into the library; each stays active where it was found")
        .arg(
            Arg::new("slugs")
                .value_name("SLUG[@VERSION]")
                .num_args(1..)
                .help("The skills, by slug; one with several versions as <slug>@<version>, the short digest `scan` prints"),
        )
        .arg(flag("all", "Every skill the Import skills dialog ticks"))
        .group(ArgGroup::new("skills").args(["slugs", "all"]).required(true))
        .arg(replace().help("Replace your copies the import replaces without asking"))
        .after_help("Example: skillbinder import pdf sql@3f9a2c1")
}

pub(super) fn ignore() -> [Command; 2] {
    [
        Command::new("ignore")
            .about("Ignore found skills, so scan, the Import skills dialog, and import --all leave them out")
            .arg(slugs())
            .after_help("Example: skillbinder ignore storybook tailwind-helper"),
        Command::new("unignore")
            .about("Stop ignoring skills, so they are offered for import again")
            .arg(slugs())
            .after_help("Example: skillbinder unignore storybook"),
    ]
}

pub(super) fn search() -> Command {
    Command::new("search")
        .about("Search the catalog, your repositories, and skills.sh, as Explore does")
        .arg(
            Arg::new("query")
                .value_name("QUERY")
                .required(true)
                .num_args(1..)
                .help("What to search for"),
        )
        .after_help("Example: skillbinder search pdf")
}

pub(super) fn install() -> Command {
    Command::new("install")
        .about("Install skills into the library from Explore, inactive")
        .arg(
            Arg::new("refs")
                .value_name("REF")
                .required(true)
                .num_args(1..)
                .help("A slug found once in the catalog or your repositories, <owner>/<repo>/<slug>, or a skills.sh <owner>/<repo>@<skill>"),
        )
        .after_help("Example: skillbinder install anthropics/skills/pdf")
}

pub(super) fn repo() -> Command {
    let repository = || name("url", "The repository, by its URL or <owner>/<repo>");
    Command::new("repo")
        .about("Add or remove one of your repositories, or install all its skills")
        .subcommand_required(true)
        .subcommand(
            Command::new("add")
                .about("Add a Git repository to browse and install its skills")
                .arg(name("url", "The repository's Git URL"))
                .after_help("Example: skillbinder repo add https://github.com/acme/skills"),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove a repository; the skills installed from it stay in the library")
                .arg(repository()),
        )
        .subcommand(
            Command::new("install-all")
                .about("Install each of a repository's skills the library lacks")
                .arg(repository())
                .after_help("Example: skillbinder repo install-all acme/skills"),
        )
}

/// `search`'s words, as one query.
pub(super) fn search_find(args: &ArgMatches) -> Find {
    Find::Search {
        query: values(args, "query").join(" "),
    }
}

pub(super) fn import_change(args: &ArgMatches) -> Change {
    Change::Import {
        skills: values(args, "slugs"),
        all: has(args, "all"),
    }
}

pub(super) fn repo_change(matches: &ArgMatches) -> Result<Change, CommandError> {
    let Some((command, args)) = matches.subcommand() else {
        return Err(unknown("repo"));
    };
    let repository = value(args, "url");
    Ok(match command {
        "add" => Change::AddRepository { url: repository },
        "remove" => Change::RemoveRepository { repository },
        "install-all" => Change::InstallAll { repository },
        other => return Err(unknown(other)),
    })
}

/// `ignore` or `unignore`.
pub(super) fn ignore_change(command: &str, args: &ArgMatches) -> Change {
    Change::Ignore {
        skills: values(args, "slugs"),
        stop: command == "unignore",
    }
}

/// `install`'s references, unchanged: the app resolves them against Explore.
pub(super) fn install_change(args: &ArgMatches) -> Change {
    Change::Install {
        skills: values(args, "refs"),
    }
}
