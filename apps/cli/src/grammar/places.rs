//! Workspaces and your agents: the sidebar's workspaces, a workspace's skills, and Settings ›
//! Agents. A workspace is named by its name or by a folder inside it, such as `.`, which the
//! command resolves here since the app runs elsewhere.

use super::{absolute, agents, flag, name, replace, slugs, unknown, value, values};
use clap::{ArgMatches, Command};
use skillbinder_proto::command_line::{Change, CommandError, WorkspaceName};
use std::path::Path;

pub(super) fn workspace() -> Command {
    let workspace = || {
        name(
            "workspace",
            "The workspace, by name or by a folder inside it, such as .",
        )
    };
    Command::new("workspace")
        .about("Add, rename, or remove a workspace, or turn skills on or off in one")
        .subcommand_required(true)
        .subcommand(
            Command::new("add")
                .about("Add a folder's projects as workspaces, as the sidebar's Add folder does")
                .arg(name("path", "The folder"))
                .arg(replace())
                .after_help("Example: skillbinder workspace add ."),
        )
        .subcommand(
            Command::new("rename")
                .about("Rename a workspace")
                .arg(workspace())
                .arg(name("new-name", "Its new name")),
        )
        .subcommand(
            Command::new("remove")
                .about("Remove a workspace from SkillBinder; its files and copies stay, unmanaged")
                .arg(workspace())
                .arg(flag("yes", "Remove without asking")),
        )
        .subcommand(
            Command::new("enable")
                .about(
                    "Turn skills on in a workspace, for all the agents it uses or the named ones",
                )
                .arg(workspace())
                .arg(slugs())
                .arg(agents().help(
                    "Only this agent, by id, name, or workspace skill directory such as \
                     .claude/skills; repeatable",
                ))
                .arg(replace())
                .after_help("Example: skillbinder workspace enable . pdf --agent claude-code"),
        )
        .subcommand(
            Command::new("disable")
                .about("Turn skills off in a workspace, removing their copies there")
                .arg(workspace())
                .arg(slugs())
                .after_help("Example: skillbinder workspace disable . pdf"),
        )
}

pub(super) fn agent() -> Command {
    let agent = || name("agent", "The agent, by id, name, or global skill directory");
    Command::new("agent")
        .about("Turn an agent found on this computer on or off")
        .subcommand_required(true)
        .subcommand(
            Command::new("on")
                .about("Make it one of your agents, writing the active preset's skills for it")
                .arg(agent())
                .arg(replace())
                .after_help("Example: skillbinder agent on cursor"),
        )
        .subcommand(
            Command::new("off")
                .about("Stop managing its skills; its copies stay in place")
                .arg(agent()),
        )
}

pub(super) fn workspace_change(matches: &ArgMatches) -> Result<Change, CommandError> {
    let Some((command, args)) = matches.subcommand() else {
        return Err(unknown("workspace"));
    };
    if command == "add" {
        return Ok(Change::AddWorkspace {
            folder: folder(&value(args, "path")).unwrap_or_else(|| absolute(value(args, "path"))),
        });
    }
    let workspace = workspace_name(value(args, "workspace"));
    Ok(match command {
        "rename" => Change::RenameWorkspace {
            workspace,
            name: value(args, "new-name"),
        },
        "remove" => Change::RemoveWorkspace { workspace },
        "enable" => Change::EnableInWorkspace {
            workspace,
            skills: values(args, "slugs"),
            agents: values(args, "agent"),
        },
        "disable" => Change::DisableInWorkspace {
            workspace,
            skills: values(args, "slugs"),
        },
        other => return Err(unknown(other)),
    })
}

pub(super) fn agent_change(matches: &ArgMatches) -> Result<Change, CommandError> {
    let Some((command, args)) = matches.subcommand() else {
        return Err(unknown("agent"));
    };
    let agent = value(args, "agent");
    Ok(match command {
        "on" => Change::TurnOnAgent { agent },
        "off" => Change::TurnOffAgent { agent },
        other => return Err(unknown(other)),
    })
}

/// A workspace as typed, with the folder it names when it names one.
fn workspace_name(name: String) -> WorkspaceName {
    WorkspaceName {
        folder: folder(&name),
        name,
    }
}

/// The folder `path` names, absolute with links resolved as the app stores workspace folders, or
/// `None` when it names none.
pub(crate) fn folder(path: &str) -> Option<String> {
    let folder = dunce::canonicalize(Path::new(path)).ok()?;
    folder.is_dir().then(|| folder.display().to_string())
}
