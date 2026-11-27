//! The trail (spec rule 16): each public `Engine` method that starts an action runs it under an
//! `op` of its own. Every line the action writes, on the worker or on the threads its work moves
//! to, carries that `op`, and its last line says how it ended and logs its error once (ADR 0032).

use super::Engine;
use crate::logging::{Area, Level};
use skillbinder_proto::{AppResult, ErrorCategory};
use std::{fmt::Display, future::Future, sync::atomic::Ordering, time::Instant};

/// Which lines an action leaves besides its failure.
#[derive(Clone, Copy)]
enum Trail {
    /// A change: started and finished at `INFO`.
    Change,
    /// A read, such as the snapshot, a preview, or a listing: started and finished at `DEBUG`.
    Detail,
    /// A background job, whose own summary stands for both (spec rule 17).
    Job,
}

/// What an action acts on, the fields its started line names, such as `slug` or `preset`.
pub(crate) type Subject = Vec<(&'static str, String)>;

/// `items` as one list field's value, such as `slugs=review,tidy`.
pub(crate) fn list<T: Display>(items: impl IntoIterator<Item = T>) -> String {
    items
        .into_iter()
        .map(|item| item.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

macro_rules! actions {
    ($($action:ident => $area:ident, $trail:ident, $name:literal;)*) => {
        /// An operation a public `Engine` method starts, which names its lines.
        #[derive(Clone, Copy)]
        pub(crate) enum Action {
            $($action,)*
        }

        impl Action {
            fn area(self) -> Area {
                match self {
                    $(Self::$action => Area::$area,)*
                }
            }

            fn trail(self) -> Trail {
                match self {
                    $(Self::$action => Trail::$trail,)*
                }
            }

            /// Its started, finished, failed, and stopped-at-quit messages.
            fn messages(self) -> [&'static str; 4] {
                match self {
                    $(Self::$action => [
                        concat!($name, " started"),
                        concat!($name, " finished"),
                        concat!($name, " failed"),
                        concat!($name, " stopped at quit"),
                    ],)*
                }
            }
        }
    };
}

actions! {
    Snapshot => Library, Detail, "snapshot";
    GrantDirectory => Workspaces, Detail, "directory grant";
    Scan => Scan, Job, "scan";
    PrepareFoundImport => Import, Detail, "found import prepare";
    PrepareImportVersion => Import, Detail, "import version prepare";
    SetIgnored => Import, Change, "ignore set";
    PreviewSkill => Library, Detail, "skill preview";
    PreviewDelete => Presets, Detail, "preset delete preview";
    SavePreferences => App, Change, "preferences save";
    Bootstrap => App, Detail, "bootstrap";
    SkipGit => App, Change, "git skip";
    ChooseAgents => Agents, Change, "agents choose";
    SettleCommandLine => CommandLine, Change, "command step settle";
    SettleConnectAgents => Mcp, Change, "agents step settle";
    InstallGit => App, Change, "git install";
    FinishOnboarding => App, Change, "onboarding finish";
    Search => Explore, Detail, "search";
    Resolve => Explore, Detail, "folder resolve";
    Install => Install, Change, "install";
    InstallAll => Install, Change, "install all";
    MarkSeen => Library, Change, "new mark clear";
    CommandLineStatus => CommandLine, Detail, "command status";
    InstallCommandLine => CommandLine, Change, "command install";
    UninstallCommandLine => CommandLine, Change, "command uninstall";
    Preview => Explore, Detail, "preview";
    ConnectRemote => Sync, Change, "remote connect";
    Refresh => Sync, Detail, "refresh";
    Pull => Sync, Change, "pull";
    Push => Sync, Change, "push";
    Sync => Sync, Change, "sync";
    ResolveSync => Sync, Change, "sync resolve";
    Disconnect => Sync, Change, "disconnect";
    RenameDevice => Sync, Change, "device rename";
    History => Sync, Detail, "history";
    RestoreSkill => Sync, Change, "restore";
    DiscardChange => Sync, Change, "discard";
    RefreshCatalog => Explore, Job, "catalog refresh";
    ListRepository => Explore, Detail, "repository list";
    AddRepository => Explore, Change, "repository add";
    RemoveRepository => Explore, Change, "repository remove";
    RestoreRepository => Explore, Change, "repository restore";
    PruneSourceCaches => Explore, Detail, "source caches prune";
    RefreshRepositories => Explore, Job, "repositories refresh";
    McpClients => Mcp, Detail, "agents read";
    ConnectMcp => Mcp, Change, "agent connect";
    DisconnectMcp => Mcp, Change, "agent disconnect";
    CheckAppUpdate => AppUpdate, Job, "check";
    SetUpdateChannel => AppUpdate, Change, "channel set";
    SetUpdateAutoCheck => AppUpdate, Change, "automatic check set";
    MoveToApplications => AppUpdate, Change, "move to applications";
    AnswerApplicationsMenu => AppUpdate, Change, "applications menu answer";
    CreatePreset => Presets, Change, "preset create";
    RenamePreset => Presets, Change, "preset rename";
    ClonePreset => Presets, Change, "preset clone";
    ActivatePreset => Presets, Change, "preset activate";
    AddToPreset => Presets, Change, "preset skills add";
    SetTargets => Presets, Change, "preset targets set";
    RemoveFromPreset => Presets, Change, "preset skills remove";
    ReorderPresets => Presets, Change, "presets reorder";
    SetPresetColor => Presets, Change, "preset color set";
    SetPresetIcon => Presets, Change, "preset icon set";
    DeletePreset => Presets, Change, "preset delete";
    CreateFolder => Library, Change, "folder create";
    RenameFolder => Library, Change, "folder rename";
    DeleteFolder => Library, Change, "folder delete";
    ReorderFolders => Library, Change, "folders reorder";
    MoveToFolder => Library, Change, "skill move";
    AddSkills => Copies, Change, "skills add";
    ApplyImport => Import, Change, "import";
    SetAgentsChosen => Agents, Change, "agents set";
    ResolveCopy => Copies, Change, "copy resolve";
    RetryCopies => Copies, Change, "copies retry";
    CheckUpdates => Update, Job, "check";
    UpdateSkills => Update, Change, "update";
    ReviewUpdate => Update, Detail, "review";
    UpdateReviewed => Update, Change, "reviewed update";
    ReinstallSkill => Update, Change, "reinstall";
    AddWorkspaces => Workspaces, Change, "workspaces add";
    RenameWorkspace => Workspaces, Change, "workspace rename";
    SetWorkspaceColor => Workspaces, Change, "workspace color set";
    SetWorkspaceIcon => Workspaces, Change, "workspace icon set";
    SetWorkspaceImage => Workspaces, Change, "workspace image set";
    ReorderWorkspaces => Workspaces, Change, "workspaces reorder";
    SetWorkspaceSkill => Workspaces, Change, "workspace skill set";
    RemoveWorkspace => Workspaces, Change, "workspace remove";
}

impl Engine {
    /// Runs `body` as `action` on `subject` under the session's next `op`. `body` is called at
    /// once with an engine whose lines, and those of each `call` it makes, carry that `op`. A change writes its
    /// started and finished lines at `INFO`, a read at `DEBUG`, and a job none; any of them that
    /// fails writes its error once, as `DEBUG` when the app is quitting (spec rule 15).
    pub(crate) fn action<T, F>(
        &self,
        action: Action,
        subject: Subject,
        body: impl FnOnce(Engine) -> F,
    ) -> impl Future<Output = AppResult<T>> + Send + 'static
    where
        T: Send + 'static,
        F: Future<Output = AppResult<T>> + Send + 'static,
    {
        let op = self.owner.ops.fetch_add(1, Ordering::Relaxed) + 1;
        let engine = Self {
            owner: self.owner.clone(),
            log: self.log.for_op(op),
        };
        let log = engine.log.clone();
        let (area, [started, finished, failed, stopped]) = (action.area(), action.messages());
        let level = match action.trail() {
            Trail::Change => Some(Level::Info),
            Trail::Detail => Some(Level::Debug),
            Trail::Job => None,
        };
        if let Some(level) = level {
            let mut line = log.event(level, area, started);
            for (name, value) in subject {
                line = line.field(name, value);
            }
        }
        let begun = Instant::now();
        let work = body(engine);
        async move {
            let result = work.await;
            match &result {
                Ok(_) => {
                    if let Some(level) = level {
                        log.event(level, area, finished).took(begun);
                    }
                }
                Err(error) if error.category == ErrorCategory::Cancelled => {
                    log.event(Level::Debug, area, stopped).took(begun);
                }
                Err(error) => {
                    log.event(Level::Error, area, failed)
                        .took(begun)
                        .error(error);
                }
            }
            result
        }
    }
}
