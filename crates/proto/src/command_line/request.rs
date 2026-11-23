//! What the command asks: one [`Request`] per connection, and an [`Answer`] for each question the
//! app asks back. Names stay strings; the app resolves them against its own library.

use serde::{Deserialize, Serialize};

/// What the command asks of the app.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "request", rename_all = "snake_case")]
pub enum Request {
    /// `skillbinder status`.
    Status,
    /// A second launch of the app asks the running one to open its window; it gets no reply.
    Open,
    /// A command that only reads, which answers even while a change runs.
    Read {
        /// What it reads.
        query: Query,
    },
    /// A command that changes the library, refused while a change runs.
    Change {
        /// What it changes.
        change: Change,
        /// What the command may answer for the user.
        consent: Consent,
    },
}

/// A reading command.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "query", rename_all = "snake_case")]
pub enum Query {
    /// `skills`: Library's list, under one of its filters and in one folder.
    Skills {
        /// Which skills it keeps.
        filter: SkillFilter,
        /// The library folder it lists, by name.
        folder: Option<String>,
    },
    /// `skill <slug>`: a skill's page.
    Skill {
        /// The skill, by slug.
        skill: String,
    },
    /// `presets`.
    Presets,
    /// `folders`.
    Folders,
    /// `copies`: every copy waiting for the user's answer.
    Copies,
    /// `workspaces`.
    Workspaces,
    /// `agents`: every agent found, yours or not.
    Agents,
}

/// Library's filters, one at a time.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillFilter {
    /// Every skill.
    #[default]
    All,
    /// Skills on for one of your agents or in a workspace.
    Active,
    /// Skills on nowhere.
    Inactive,
    /// Skills whose source has a newer version.
    Updates,
}

/// A changing command. Skills are named by slug, presets and folders by name, and agents by
/// registry identity, name, or global skill directory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "change", rename_all = "snake_case")]
pub enum Change {
    /// `enable`: turns skills on for your agents, in the active preset.
    Enable {
        /// The skills.
        skills: Vec<String>,
        /// Only these agents; none means all your agents.
        agents: Vec<String>,
    },
    /// `disable`: turns skills off for your agents, in the active preset.
    Disable {
        /// The skills.
        skills: Vec<String>,
        /// Only these agents; none means all your agents.
        agents: Vec<String>,
    },
    /// `preset create`.
    CreatePreset {
        /// The new preset's name.
        name: String,
    },
    /// `preset rename`.
    RenamePreset {
        /// The preset.
        preset: String,
        /// Its new name.
        name: String,
    },
    /// `preset clone`.
    ClonePreset {
        /// The preset copied.
        preset: String,
    },
    /// `preset delete`, which confirms first.
    DeletePreset {
        /// The preset.
        preset: String,
    },
    /// `preset activate`.
    ActivatePreset {
        /// The preset.
        preset: String,
    },
    /// `preset add`.
    AddToPreset {
        /// The preset.
        preset: String,
        /// The skills it gains.
        skills: Vec<String>,
        /// Only these agents; none means all your agents.
        agents: Vec<String>,
    },
    /// `preset remove`.
    RemoveFromPreset {
        /// The preset.
        preset: String,
        /// The skills it loses.
        skills: Vec<String>,
    },
    /// `folder create`.
    CreateFolder {
        /// The new folder's name.
        name: String,
    },
    /// `folder rename`.
    RenameFolder {
        /// The folder.
        folder: String,
        /// Its new name.
        name: String,
    },
    /// `folder delete`, which confirms first.
    DeleteFolder {
        /// The folder.
        folder: String,
    },
    /// `folder move`.
    MoveToFolder {
        /// The skills moved.
        skills: Vec<String>,
        /// The folder they move into, or none to leave them unfiled.
        folder: Option<String>,
    },
    /// `copies replace`.
    ReplaceCopy {
        /// The copy's skill.
        skill: String,
        /// The copy's directory, when the skill waits in more than one.
        directory: Option<String>,
    },
    /// `copies keep`.
    KeepCopy {
        /// The copy's skill.
        skill: String,
        /// The copy's directory, when the skill waits in more than one.
        directory: Option<String>,
    },
    /// `copies retry`.
    RetryCopies,
    /// `workspace add`: adds a folder's projects as workspaces, as the sidebar's Add folder does.
    AddWorkspace {
        /// The folder, absolute.
        folder: String,
    },
    /// `workspace rename`.
    RenameWorkspace {
        /// The workspace.
        workspace: WorkspaceName,
        /// Its new name.
        name: String,
    },
    /// `workspace remove`, which confirms first.
    RemoveWorkspace {
        /// The workspace.
        workspace: WorkspaceName,
    },
    /// `workspace enable`: turns skills on in a workspace, as its Add skill picker does.
    EnableInWorkspace {
        /// The workspace.
        workspace: WorkspaceName,
        /// The skills.
        skills: Vec<String>,
        /// Only these agents; none means all the agents the workspace uses.
        agents: Vec<String>,
    },
    /// `workspace disable`: turns skills off in a workspace.
    DisableInWorkspace {
        /// The workspace.
        workspace: WorkspaceName,
        /// The skills.
        skills: Vec<String>,
    },
    /// `agent on`: makes an agent one of yours, applying the active preset to it.
    TurnOnAgent {
        /// The agent.
        agent: String,
    },
    /// `agent off`: leaves its copies in place, unmanaged.
    TurnOffAgent {
        /// The agent.
        agent: String,
    },
}

/// A workspace as the command names it: by its name, or by a folder inside it, such as `.`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceName {
    /// The name as typed.
    pub name: String,
    /// The folder it names, absolute with links resolved, when it names one.
    pub folder: Option<String>,
}

/// What the command may answer for the user (spec rules 6 and 8, A5): replacing copies
/// SkillBinder does not manage, and confirming a deletion, kept apart so a script that deletes
/// never replaces copies by accident.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Consent {
    /// Replacing a copy SkillBinder does not manage, or an edited managed copy.
    pub replace: Permission,
    /// Deleting a preset or a library folder.
    pub confirm: Permission,
}

/// How the command answers one kind of question.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Ask the user in the terminal.
    Ask,
    /// Yes, from `--replace` or `--yes`.
    Yes,
    /// No terminal and no flag: leave it for the user.
    #[default]
    No,
}

/// The command's answer to a [`super::Reply::Ask`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    /// The user said yes.
    pub yes: bool,
}
