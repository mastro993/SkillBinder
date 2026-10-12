//! Navigable routes and the in-memory back/forward history between them.

use skillbinder_proto::{DeploymentTarget, FolderId};

/// Which skills the Library route lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum LibraryFilter {
    All,
    Folder(FolderId),
}

/// A sidebar destination; Library carries its folder as a sub-route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Route {
    Discovery,
    Library(LibraryFilter),
    /// The page of Global or a workspace: `workspace/{id}`, or Global's own route.
    Target(DeploymentTarget),
    Sync,
    Settings,
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
    use super::{History, LibraryFilter, Route};
    use skillbinder_proto::{DeploymentTarget, WorkspaceId};

    #[test]
    fn back_and_forward_walk_the_visited_routes() {
        let mut history = History::default();
        let library = Route::Library(LibraryFilter::All);
        history.push(Route::Discovery);
        history.push(library.clone());
        assert_eq!(history.back(Route::Settings), Some(library.clone()));
        assert_eq!(history.back(library.clone()), Some(Route::Discovery));
        assert_eq!(history.back(Route::Discovery), None);
        assert_eq!(history.forward(Route::Discovery), Some(library.clone()));
        assert_eq!(history.forward(library), Some(Route::Settings));
        assert!(!history.can_go_forward());
    }

    #[test]
    fn target_pages_join_the_history() {
        let mut history = History::default();
        let workspace = Route::Target(DeploymentTarget::Workspace(WorkspaceId::new()));
        history.push(Route::Discovery);
        history.push(Route::Target(DeploymentTarget::Global));
        assert_eq!(
            history.back(workspace.clone()),
            Some(Route::Target(DeploymentTarget::Global))
        );
        assert_eq!(
            history.forward(Route::Target(DeploymentTarget::Global)),
            Some(workspace)
        );
    }

    #[test]
    fn navigating_after_going_back_drops_the_forward_routes() {
        let mut history = History::default();
        history.push(Route::Discovery);
        history.back(Route::Sync);
        assert!(history.can_go_forward());
        history.push(Route::Discovery);
        assert!(!history.can_go_forward());
        assert!(history.can_go_back());
    }
}
