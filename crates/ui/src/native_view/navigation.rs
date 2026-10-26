use gpui_kit::{Context, Window};
use skillbinder_proto::*;

use super::route::Route;
use super::{NativeView, agent_page};

impl NativeView {
    /// Opens `route` and records the current one for Back.
    pub(super) fn navigate(&mut self, route: Route, window: &mut Window, cx: &mut Context<Self>) {
        if self.route != route {
            self.history.push(self.route.clone());
        }
        self.show(route, window, cx);
    }

    pub(super) fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(route) = self.history.back(self.route.clone()) {
            self.show(route, window, cx);
        }
    }

    pub(super) fn go_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(route) = self.history.forward(self.route.clone()) {
            self.show(route, window, cx);
        }
    }

    /// Shows `route` without touching history; a deleted skill falls back to Library, and a
    /// removed workspace or an agent no longer detected to Discovery.
    pub(super) fn show(&mut self, route: Route, window: &mut Window, cx: &mut Context<Self>) {
        let skill_exists = |id: &SkillId| {
            self.snapshot
                .library
                .as_ref()
                .is_some_and(|library| library.library.skills.iter().any(|s| &s.id == id))
        };
        let workspace_exists =
            |id: &WorkspaceId| self.snapshot.workspaces.iter().any(|p| &p.id == id);
        let route = match route {
            Route::Skill(id) if !skill_exists(&id) => Route::Library,
            Route::Workspace(id) if !workspace_exists(&id) => Route::Discovery,
            route => agent_page::agent_fallback(&self.snapshot.agents, &self.snapshot.home, route),
        };
        if self.route != route {
            self.leave_page();
            self.needs_update_only = false;
            self.search
                .update(cx, |input, cx| input.set_value("", window, cx));
            if let Route::Skill(id) = &route {
                self.load_skill(id.clone(), cx);
            }
        }
        self.route = route;
        self.error = None;
        cx.notify();
    }

    /// Drops the shown page's transient state and scrolls the next page to its top.
    pub(super) fn leave_page(&mut self) {
        self.selected_skills.clear();
        self.version_choices.clear();
        self.expanded_skills.clear();
        self.acknowledge_invalid = false;
        self.preview = None;
        self.update_outcome = None;
        self.skill_load = None;
        self.content_scroll.handle.set_offset(Default::default());
    }
}
