//! Navigable routes and the in-memory back/forward history between them.

use super::agent_page::agent_fallback;
use skillbinder_proto::{AppSnapshot, SkillId, WorkspaceId};

/// A navigable page.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum Route {
    /// The catalog and search, the sidebar's first row; the app still opens on Library.
    Explore,
    #[default]
    Library,
    /// A library skill's page, with its files: `skill/{id}`.
    Skill(SkillId),
    /// A workspace's page: `workspace/{id}`.
    Workspace(WorkspaceId),
    /// One of your agents' page, by registry id: `agent/{id}`.
    Agent(String),
    /// The page of the universal agents together, which read `~/.agents/skills`:
    /// `agents/universal`.
    Universal,
    Sync,
    Settings,
}

impl Route {
    /// A workspace's, an agent's, or Universal's page, which lays out its own column.
    pub(super) fn is_scoped(&self) -> bool {
        matches!(self, Self::Workspace(_) | Self::Agent(_) | Self::Universal)
    }
}

/// The page `snapshot` shows for `route`: a removed workspace's page opens Library, and an
/// agent's or Universal's page follows [`agent_fallback`].
pub(super) fn fallback(snapshot: &AppSnapshot, route: Route) -> Route {
    match route {
        Route::Workspace(id) if !snapshot.workspaces.iter().any(|w| w.id == id) => Route::Library,
        route => agent_fallback(&snapshot.agents, &snapshot.home, route),
    }
}

/// Routes behind and ahead of the current one.
#[derive(Default)]
pub(super) struct History {
    back: Vec<Route>,
    forward: Vec<Route>,
}

impl History {
    /// Records leaving `from` for a new destination, which discards the forward routes.
    pub(super) fn push(&mut self, from: Route) {
        self.back.push(from);
        self.forward.clear();
    }

    pub(super) fn back(&mut self, current: Route) -> Option<Route> {
        let route = self.back.pop()?;
        self.forward.push(current);
        Some(route)
    }

    pub(super) fn forward(&mut self, current: Route) -> Option<Route> {
        let route = self.forward.pop()?;
        self.back.push(current);
        Some(route)
    }

    pub(super) fn can_go_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub(super) fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::super::snapshot_query::fixture::Fixture;
    use super::{History, Route, fallback};
    use skillbinder_proto::WorkspaceId;

    #[test]
    fn the_app_starts_on_library() {
        assert_eq!(Route::default(), Route::Library);
    }

    #[test]
    fn pages_whose_workspace_or_agents_are_gone_open_library() {
        let snapshot = Fixture::new().workspace("app", &[".agents/skills"]).build();
        let workspace = |name: &str| Route::Workspace(WorkspaceId(name.to_owned()));
        assert_eq!(fallback(&snapshot, workspace("app")), workspace("app"));
        assert_eq!(fallback(&snapshot, workspace("gone")), Route::Library);
        assert_eq!(
            fallback(&snapshot, Route::Agent("cursor".to_owned())),
            Route::Library
        );
        assert_eq!(fallback(&snapshot, Route::Universal), Route::Library);
        assert_eq!(fallback(&snapshot, Route::Sync), Route::Sync);
    }

    #[test]
    fn back_and_forward_walk_the_visited_routes() {
        let mut history = History::default();
        let library = Route::Library;
        history.push(Route::Sync);
        history.push(library.clone());
        assert_eq!(history.back(Route::Settings), Some(library.clone()));
        assert_eq!(history.back(library.clone()), Some(Route::Sync));
        assert_eq!(history.back(Route::Sync), None);
        assert_eq!(history.forward(Route::Sync), Some(library.clone()));
        assert_eq!(history.forward(library), Some(Route::Settings));
        assert!(!history.can_go_forward());
    }

    #[test]
    fn workspace_pages_join_the_history() {
        let mut history = History::default();
        let workspace = Route::Workspace(WorkspaceId::new());
        history.push(Route::Library);
        history.push(Route::Universal);
        assert_eq!(history.back(workspace.clone()), Some(Route::Universal));
        assert_eq!(history.forward(Route::Universal), Some(workspace));
    }

    #[test]
    fn navigating_after_going_back_drops_the_forward_routes() {
        let mut history = History::default();
        history.push(Route::Library);
        history.back(Route::Sync);
        assert!(history.can_go_forward());
        history.push(Route::Library);
        assert!(!history.can_go_forward());
        assert!(history.can_go_back());
    }
}
