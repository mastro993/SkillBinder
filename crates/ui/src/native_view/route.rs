//! Navigable routes and the in-memory back/forward history between them.

use skillbinder_proto::FolderId;

/// Which skills the Library route lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum LibraryFilter {
    All,
    Unfiled,
    Folder(FolderId),
}

/// A sidebar destination; Library carries its folder as a sub-route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Route {
    Discovery,
    Library(LibraryFilter),
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

    #[test]
    fn back_and_forward_walk_the_visited_routes() {
        let mut history = History::default();
        let unfiled = Route::Library(LibraryFilter::Unfiled);
        history.push(Route::Discovery);
        history.push(unfiled.clone());
        assert_eq!(history.back(Route::Settings), Some(unfiled.clone()));
        assert_eq!(history.back(unfiled.clone()), Some(Route::Discovery));
        assert_eq!(history.back(Route::Discovery), None);
        assert_eq!(history.forward(Route::Discovery), Some(unfiled.clone()));
        assert_eq!(history.forward(unfiled), Some(Route::Settings));
        assert!(!history.can_go_forward());
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
