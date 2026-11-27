//! The command line: built rather than derived, since the derive allows Clippy's style lints,
//! which the workspace forbids allowing, and read into one [`Request`].

use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command, value_parser};
use skillbinder_proto::command_line::{
    Change, CommandError, Consent, ErrorCode, Find, Permission, Query, Request, SkillFilter,
};
use std::path::PathBuf;

pub(crate) fn grammar() -> Command {
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
        .subcommand(
            Command::new("skills")
                .about("List the library's skills, where each is on, and its marks")
                .arg(flag("active", "Only skills on for your agents or in a workspace"))
                .arg(flag("inactive", "Only skills on nowhere"))
                .arg(flag("updates", "Only skills whose source has a newer version"))
                .group(ArgGroup::new("filter").args(["active", "inactive", "updates"]))
                .arg(Arg::new("folder").long("folder").value_name("F").help("Only skills in this library folder"))
                .after_help("Example: skillbinder skills --active --folder Writing"),
        )
        .subcommand(
            Command::new("skill")
                .about("Show a skill's page: its description, source, presets, and copies")
                .arg(name("slug", "The skill's slug"))
                .after_help("Example: skillbinder skill pdf"),
        )
        .subcommand(
            Command::new("enable")
                .about("Turn skills on for your agents, in the active preset")
                .arg(slugs())
                .arg(agents())
                .arg(replace())
                .after_help("Example: skillbinder enable pdf --agent claude-code"),
        )
        .subcommand(
            Command::new("disable")
                .about("Turn skills off for your agents, in the active preset")
                .arg(slugs())
                .arg(agents())
                .after_help("Example: skillbinder disable pdf"),
        )
        .subcommand(Command::new("presets").about("List the presets, marking the active one"))
        .subcommand(preset())
        .subcommand(Command::new("folders").about("List the library folders"))
        .subcommand(folder())
        .subcommand(copies())
        .subcommand(Command::new("workspaces").about("List the workspaces, their agents, and the skills on in each"))
        .subcommand(places::workspace())
        .subcommand(
            Command::new("agents")
                .about("List the agents found on this computer, marking yours"),
        )
        .subcommand(places::agent())
        .subcommand(explore::scan())
        .subcommand(explore::import())
        .subcommands(explore::ignore())
        .subcommand(explore::search())
        .subcommand(explore::install())
        .subcommand(Command::new("repos").about("List your repositories and how many of their skills aren't in the library"))
        .subcommand(explore::repo())
        .subcommand(updates::updates())
        .subcommand(updates::update())
        .subcommand(updates::reinstall())
        .subcommands(sync::sync())
        .subcommand(sync::remote())
        .subcommand(sync::device())
        .subcommand(
            Command::new("mcp")
                .about("Serve SkillBinder's tools to an agent over MCP, on stdin and stdout")
                .after_help("Example: claude mcp add --scope user skillbinder -- skillbinder mcp"),
        )
}

fn preset() -> Command {
    Command::new("preset")
        .about("Create, rename, clone, delete, activate, or edit a preset")
        .subcommand_required(true)
        .subcommand(Command::new("create").about("Create a preset").arg(name("name", "The new preset's name")))
        .subcommand(
            Command::new("rename")
                .about("Rename a preset")
                .arg(name("name", "The preset"))
                .arg(name("new-name", "Its new name")),
        )
        .subcommand(
            Command::new("clone")
                .about("Copy a preset and its skills into a new one")
                .arg(name("name", "The preset")),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a preset; its skills stay in the library. Deleting the active one activates Default")
                .arg(name("name", "The preset"))
                .arg(yes()),
        )
        .subcommand(
            Command::new("activate")
                .about("Make a preset active, writing its skills for your agents")
                .arg(name("name", "The preset"))
                .arg(replace())
                .after_help("Example: skillbinder preset activate Work --replace"),
        )
        .subcommand(
            Command::new("add")
                .about("Add skills to a preset, active or not")
                .arg(name("name", "The preset"))
                .arg(slugs())
                .arg(agents())
                .arg(replace()),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove skills from a preset; they stay in the library")
                .arg(name("name", "The preset"))
                .arg(slugs()),
        )
}

fn folder() -> Command {
    Command::new("folder")
        .about("Create, rename, or delete a library folder, or file skills in one")
        .subcommand_required(true)
        .subcommand(
            Command::new("create")
                .about("Create a library folder")
                .arg(name("name", "The new folder's name")),
        )
        .subcommand(
            Command::new("rename")
                .about("Rename a library folder")
                .arg(name("name", "The folder"))
                .arg(name("new-name", "Its new name")),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete a library folder; its skills stay in the library, unfiled")
                .arg(name("name", "The folder"))
                .arg(yes()),
        )
        .subcommand(
            Command::new("move")
                .about("File skills in a library folder, or in none")
                .arg(slugs())
                .arg(
                    Arg::new("to")
                        .long("to")
                        .value_name("F")
                        .help("The folder they move into"),
                )
                .arg(flag("none", "Leave them in no folder"))
                .group(
                    ArgGroup::new("destination")
                        .args(["to", "none"])
                        .required(true),
                )
                .after_help("Example: skillbinder folder move pdf docx --to Documents"),
        )
}

fn copies() -> Command {
    let copy = |command: &'static str, about: &'static str| {
        Command::new(command)
            .about(about)
            .arg(name("slug", "The copy's skill"))
            .arg(
                Arg::new("in")
                    .long("in")
                    .value_name("DIR")
                    .help("The skill directory holding it, when it waits in more than one"),
            )
    };
    Command::new("copies")
        .about("List the copies waiting for your answer, or answer for one")
        .subcommand(copy(
            "replace",
            "Replace your copy with the library's, which SkillBinder then manages",
        ))
        .subcommand(copy(
            "keep",
            "Keep your copy as it is; SkillBinder stops managing it",
        ))
        .subcommand(
            Command::new("retry").about("Try again every copy that couldn't be written or removed"),
        )
}

fn flag(id: &'static str, help: &'static str) -> Arg {
    Arg::new(id).long(id).action(ArgAction::SetTrue).help(help)
}

fn name(id: &'static str, help: &'static str) -> Arg {
    Arg::new(id).required(true).help(help)
}

fn slugs() -> Arg {
    Arg::new("slugs")
        .value_name("SLUG")
        .required(true)
        .num_args(1..)
        .help("The skills, by slug")
}

fn agents() -> Arg {
    Arg::new("agent")
        .long("agent")
        .value_name("A")
        .action(ArgAction::Append)
        .help("Only this agent, by id, name, or global skill directory; repeatable")
}

fn replace() -> Arg {
    flag(
        "replace",
        "Replace your own copies of these skills in their skill directories without asking",
    )
}

fn yes() -> Arg {
    flag("yes", "Delete without asking")
}

/// The request the command line asks for; `interactive` when the user can answer questions.
pub(crate) fn request(matches: &ArgMatches, interactive: bool) -> Result<Request, CommandError> {
    let Some((command, args)) = matches.subcommand() else {
        return Err(unknown(""));
    };
    let read = |query| Ok(Request::Read { query });
    // `--replace` and `--yes` belong to the innermost command, such as `preset activate`.
    let mut leaf = args;
    while let Some((_, inner)) = leaf.subcommand() {
        leaf = inner;
    }
    let permission = |flag: &str| match (has(leaf, flag), interactive) {
        (true, _) => Permission::Yes,
        (false, true) => Permission::Ask,
        (false, false) => Permission::No,
    };
    let consent = Consent {
        replace: permission("replace"),
        confirm: permission("yes"),
    };
    let change = |change| Ok(Request::Change { change, consent });
    match command {
        "status" => Ok(Request::Status),
        "skills" => read(Query::Skills {
            filter: if has(args, "active") {
                SkillFilter::Active
            } else if has(args, "inactive") {
                SkillFilter::Inactive
            } else if has(args, "updates") {
                SkillFilter::Updates
            } else {
                SkillFilter::All
            },
            folder: text(args, "folder"),
        }),
        "skill" => read(Query::Skill {
            skill: value(args, "slug"),
        }),
        "presets" => read(Query::Presets),
        "folders" => read(Query::Folders),
        "workspaces" => read(Query::Workspaces),
        "agents" => read(Query::Agents),
        "repos" => read(Query::Repositories),
        "scan" => Ok(Request::Find { find: Find::Scan }),
        "search" => Ok(Request::Find {
            find: explore::search_find(args),
        }),
        "import" => change(explore::import_change(args)),
        "ignore" | "unignore" => change(explore::ignore_change(command, args)),
        "install" => change(explore::install_change(args)),
        "repo" => explore::repo_change(args).and_then(change),
        "updates" if has(args, "check") => Ok(Request::Find { find: Find::Check }),
        "updates" => read(Query::Updates),
        "update" if has(args, "diff") => Ok(Request::Find {
            find: updates::diff_find(args),
        }),
        "update" => change(updates::update_change(args)),
        "reinstall" => change(Change::Reinstall {
            skill: value(args, "slug"),
        }),
        "sync" | "push" | "pull" => sync::sync_request(command, args, consent),
        "remote" => sync::remote_change(args).and_then(change),
        "device" => sync::device_change(args).and_then(change),
        "workspace" => places::workspace_change(args).and_then(change),
        "agent" => places::agent_change(args).and_then(change),
        "enable" => change(Change::Enable {
            skills: values(args, "slugs"),
            agents: values(args, "agent"),
        }),
        "disable" => change(Change::Disable {
            skills: values(args, "slugs"),
            agents: values(args, "agent"),
        }),
        "preset" => preset_change(args).and_then(change),
        "folder" => folder_change(args).and_then(change),
        "copies" => match args.subcommand() {
            None => read(Query::Copies),
            Some(("retry", _)) => change(Change::RetryCopies),
            Some((answer, copy)) => {
                let (skill, directory) = (value(copy, "slug"), text(copy, "in").map(absolute));
                let answered = match answer {
                    "replace" => Change::ReplaceCopy { skill, directory },
                    "keep" => Change::KeepCopy { skill, directory },
                    other => return Err(unknown(other)),
                };
                change(answered)
            }
        },
        other => Err(unknown(other)),
    }
}

fn preset_change(matches: &ArgMatches) -> Result<Change, CommandError> {
    let Some((command, args)) = matches.subcommand() else {
        return Err(unknown("preset"));
    };
    let preset = value(args, "name");
    Ok(match command {
        "create" => Change::CreatePreset { name: preset },
        "rename" => Change::RenamePreset {
            preset,
            name: value(args, "new-name"),
        },
        "clone" => Change::ClonePreset { preset },
        "delete" => Change::DeletePreset { preset },
        "activate" => Change::ActivatePreset { preset },
        "add" => Change::AddToPreset {
            preset,
            skills: values(args, "slugs"),
            agents: values(args, "agent"),
        },
        "remove" => Change::RemoveFromPreset {
            preset,
            skills: values(args, "slugs"),
        },
        other => return Err(unknown(other)),
    })
}

fn folder_change(matches: &ArgMatches) -> Result<Change, CommandError> {
    let Some((command, args)) = matches.subcommand() else {
        return Err(unknown("folder"));
    };
    Ok(match command {
        "create" => Change::CreateFolder {
            name: value(args, "name"),
        },
        "rename" => Change::RenameFolder {
            folder: value(args, "name"),
            name: value(args, "new-name"),
        },
        "delete" => Change::DeleteFolder {
            folder: value(args, "name"),
        },
        "move" => Change::MoveToFolder {
            skills: values(args, "slugs"),
            folder: text(args, "to"),
        },
        other => return Err(unknown(other)),
    })
}

fn has(args: &ArgMatches, flag: &str) -> bool {
    args.try_get_one::<bool>(flag).ok().flatten() == Some(&true)
}

fn text(args: &ArgMatches, id: &str) -> Option<String> {
    args.get_one::<String>(id).cloned()
}

fn value(args: &ArgMatches, id: &str) -> String {
    text(args, id).unwrap_or_default()
}

fn values(args: &ArgMatches, id: &str) -> Vec<String> {
    args.get_many::<String>(id)
        .map(|values| values.cloned().collect())
        .unwrap_or_default()
}

/// A directory as given, made absolute against the current folder when it is relative and not
/// written from the home folder, since the app runs elsewhere.
fn absolute(directory: String) -> String {
    let path = PathBuf::from(&directory);
    if path.is_absolute() || directory.starts_with('~') {
        return directory;
    }
    std::env::current_dir()
        .map(|current| current.join(path).display().to_string())
        .unwrap_or(directory)
}

fn unknown(command: &str) -> CommandError {
    CommandError::new(ErrorCode::Usage, format!("Unknown command {command}"))
}

mod explore;
mod places;
pub(crate) use places::folder as workspace_folder;
mod sync;
#[cfg(test)]
mod tests;
mod updates;
