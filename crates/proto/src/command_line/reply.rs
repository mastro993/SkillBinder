//! What the app sends back for a request: progress and questions while it runs, then one outcome
//! holding the command's typed output.

use super::{CommandError, FoundOutput, RepositoryRow, SearchOutput, UpdatesOutput};
use crate::{AppError, SkillSource, SyncState};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A line the app sends back for a request.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reply {
    /// How far the change has got.
    Progress(Progress),
    /// A question for the user; the command answers with one [`super::Answer`] line.
    Ask(Question),
    /// The request ended; it is the last line.
    Outcome(Result<Output, Failure>),
}

/// How far a change that writes copies has got.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    /// What runs, such as `Switching to Work`.
    pub activity: String,
    /// Copies written or removed so far.
    pub done: usize,
    /// Copies the change writes or removes.
    pub total: usize,
}

/// A yes-or-no question, in the app's own words.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Question {
    /// The question, without the `[y/N]` the command adds.
    pub prompt: String,
}

/// A command that failed, with what it changed before it stopped, such as copies left waiting.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Failure {
    /// Why it failed.
    pub error: CommandError,
    /// What changed, when part of it landed.
    pub result: Option<Box<Output>>,
}

impl From<CommandError> for Failure {
    fn from(error: CommandError) -> Self {
        Self {
            error,
            result: None,
        }
    }
}

impl From<AppError> for Failure {
    fn from(error: AppError) -> Self {
        CommandError::from(error).into()
    }
}

/// A command's result.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Output {
    /// `status`.
    Status(StatusOutput),
    /// `skills`.
    Skills(Vec<SkillRow>),
    /// `skill`.
    Skill(Box<SkillOutput>),
    /// `presets`.
    Presets(Vec<PresetRow>),
    /// `folders`.
    Folders(Vec<FolderRow>),
    /// `copies`.
    Copies(Vec<CopyRow>),
    /// `workspaces`.
    Workspaces(Vec<WorkspaceRow>),
    /// `agents`.
    Agents(Vec<AgentRow>),
    /// `scan`.
    Found(FoundOutput),
    /// `search`.
    Search(SearchOutput),
    /// `repos`.
    Repositories(Vec<RepositoryRow>),
    /// `updates`.
    Updates(UpdatesOutput),
    /// Every changing command.
    Changed(Changed),
}

/// What `status` reports: the same counts Library and the menu show.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StatusOutput {
    /// The app's version.
    pub version: String,
    /// Whether onboarding finished; every other command waits for it.
    pub onboarding_finished: bool,
    /// The active preset's name.
    pub active_preset: Option<String>,
    /// Library skills.
    pub skills: usize,
    /// Library skills that are Active.
    pub active: usize,
    /// Library skills whose source has a newer version.
    pub updates: usize,
    /// Copies waiting for the user's answer.
    pub attention: usize,
    /// Where the library stands with its sync remote.
    pub sync: SyncState,
    /// The work that refuses changes while it runs, such as `Switching to Work`.
    pub running: Option<String>,
}

/// A library skill as Library lists it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SkillRow {
    /// The skill's slug.
    pub slug: String,
    /// Its description.
    pub description: String,
    /// On for one of your agents or in a workspace.
    pub active: bool,
    /// Your agents the active preset holds it for, by the Agents section's names.
    pub agents: Vec<String>,
    /// The workspaces holding it, by name.
    pub workspaces: Vec<String>,
    /// Its library folder, by name.
    pub folder: Option<String>,
    /// Its source has a newer version.
    pub update: bool,
    /// Its content differs from its record.
    pub drifted: bool,
    /// It was installed and not opened or activated yet.
    pub new: bool,
}

/// A skill's page.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SkillOutput {
    /// What Library lists for it.
    #[serde(flatten)]
    pub skill: SkillRow,
    /// Where it was published from.
    pub source: Option<SkillSource>,
    /// The Git tree id of the content its record holds.
    pub recorded: String,
    /// The presets holding it, by name.
    pub presets: Vec<String>,
    /// Its copies in your agents' and the workspaces' skill directories.
    pub copies: Vec<CopyRow>,
}

/// A preset as the sidebar lists it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PresetRow {
    /// Its name.
    pub name: String,
    /// It is the active preset.
    pub active: bool,
    /// The skills it holds.
    pub skills: usize,
}

/// A library folder as Library's tabs list it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FolderRow {
    /// Its name.
    pub name: String,
    /// The skills filed in it.
    pub skills: usize,
}

/// A workspace as the sidebar lists it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceRow {
    /// Its name.
    pub name: String,
    /// Its folder, with the home folder shortened to `~`.
    pub folder: String,
    /// Its folder was missing at the last scan.
    pub missing: bool,
    /// Its agents: the workspace skill directories it uses, such as `.claude/skills`.
    pub agents: Vec<String>,
    /// The library skills on there, by slug.
    pub skills: Vec<String>,
}

/// An agent found on this computer as Settings › Agents lists it. Agents sharing one global skill
/// directory, such as the universal agents, are one row.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AgentRow {
    /// The row's name.
    pub name: String,
    /// Its agents' registry identities.
    pub ids: Vec<String>,
    /// Their global skill directory, with the home folder shortened to `~`.
    pub directory: String,
    /// They are yours: SkillBinder applies the active preset to them.
    pub on: bool,
}

/// A skill's copy in a skill directory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CopyRow {
    /// The skill's slug.
    pub skill: String,
    /// The skill directory, with the home folder shortened to `~`.
    pub directory: String,
    /// How it stands.
    pub state: CopyStatus,
    /// Why it could not be written or removed.
    pub reason: Option<String>,
}

/// How a copy stands.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CopyStatus {
    /// The managed copy is in place.
    Applied,
    /// The user kept their own copy.
    Kept,
    /// A copy SkillBinder does not manage holds the slug; replacing it waits for the user.
    Conflict,
    /// The managed copy was changed there; replacing it waits for the user.
    Edited,
    /// The managed copy was changed there, and the skill is no longer active there; removing it
    /// waits for the user.
    EditedInactive,
    /// The copy could not be written.
    NotApplied,
    /// The copy could not be removed.
    NotRemoved,
}

/// What a changing command did.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Changed {
    /// What changed, as one sentence.
    pub message: String,
    /// The preset that became active, when the change switched presets.
    pub activated: Option<String>,
    /// Skills the switch added for your agents.
    pub added: Vec<String>,
    /// Skills the switch removed from your agents.
    pub removed: Vec<String>,
    /// Copies left waiting for the user's answer.
    pub waiting: Vec<CopyRow>,
    /// Skills an import or install put in the library, by slug.
    pub installed: Vec<String>,
    /// Skills an update brought to their source's version, by slug.
    pub updated: Vec<String>,
}

impl Changed {
    /// A change that only says what it did.
    pub fn said(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            ..Self::default()
        }
    }
}
