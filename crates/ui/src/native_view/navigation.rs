use gpui_kit::{Context, Window};
use skillbinder_proto::*;

use super::NativeView;
use super::route::{self, Route};

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

    /// Shows `route` without touching history; a deleted skill's page opens Library, and other
    /// pages follow [`route::fallback`].
    pub(super) fn show(&mut self, route: Route, window: &mut Window, cx: &mut Context<Self>) {
        let skill_exists = |id: &SkillId| {
            self.snapshot
                .library
                .as_ref()
                .is_some_and(|library| library.library.skills.iter().any(|s| &s.id == id))
        };
        let route = match route {
            Route::Skill(id) if !skill_exists(&id) => Route::Library,
            route => route::fallback(&self.snapshot, route),
        };
        if self.route != route {
            self.leave_page();
            self.search
                .update(cx, |input, cx| input.set_value("", window, cx));
            self.explore.clear(window, cx);
            if let Route::Skill(id) = &route {
                self.load_skill(id.clone(), cx);
            }
            if route == Route::Explore {
                self.client.explore_opened();
            }
        }
        self.route = route;
        self.error = None;
        cx.notify();
    }

    /// Drops the shown page's transient state and scrolls the next page to its top.
    pub(super) fn leave_page(&mut self) {
        self.preview = None;
        self.update_outcome = None;
        self.skill_load = None;
        self.place_popover = None;
        self.library_reveal = None;
        self.content_scroll.handle.set_offset(Default::default());
    }
}
