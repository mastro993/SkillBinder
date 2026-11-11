//! The header breadcrumb: the current page's title under its sidebar section.

use super::{NativeView, Route};
use gpui_kit::component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_kit::{Context, IntoElement, Styled};

/// One breadcrumb step; `target` is the route it opens, and the last step opens nothing.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Crumb {
    pub label: String,
    pub target: Option<Route>,
}

impl Crumb {
    fn text(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            target: None,
        }
    }
}

/// The steps leading to `route`, ending in its `title`. A section without a page of its own is
/// plain text.
pub(super) fn crumbs(route: &Route, title: String) -> Vec<Crumb> {
    let section = match route {
        Route::Skill(_) => Some(Crumb {
            label: "Library".to_owned(),
            target: Some(Route::Library),
        }),
        // A workspace's, an agent's, and Universal's page draw their own head instead.
        Route::Workspace(_)
        | Route::Agent(_)
        | Route::Universal
        | Route::Discovery
        | Route::Library
        | Route::Sync
        | Route::Settings => None,
    };
    section.into_iter().chain([Crumb::text(title)]).collect()
}

/// The current page's own name, as the sidebar shows it. Library and the scoped pages draw their
/// own title instead of the breadcrumb.
fn title(route: &Route) -> String {
    match route {
        Route::Discovery => "Discovery".to_owned(),
        Route::Skill(id) => id.to_string(),
        Route::Sync => "Sync".to_owned(),
        Route::Settings => "Settings".to_owned(),
        Route::Library | Route::Workspace(_) | Route::Agent(_) | Route::Universal => String::new(),
    }
}

impl NativeView {
    pub(super) fn render_breadcrumb(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();
        let steps = crumbs(&self.route, title(&self.route));
        Breadcrumb::new()
            .min_w_0()
            .children(steps.into_iter().map(|crumb| {
                let item = BreadcrumbItem::new(crumb.label);
                let Some(route) = crumb.target else {
                    return item;
                };
                let weak = weak.clone();
                item.on_click(move |_, window, cx| {
                    let _ = weak.update(cx, |view, cx| view.navigate(route.clone(), window, cx));
                })
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_proto::{SkillId, WorkspaceId};

    fn labels(route: Route) -> Vec<String> {
        crumbs(&route, "Page".to_owned())
            .into_iter()
            .map(|crumb| crumb.label)
            .collect()
    }

    #[test]
    fn pages_without_a_section_are_their_title_alone() {
        for route in [
            Route::Discovery,
            Route::Library,
            Route::Workspace(WorkspaceId::new()),
            Route::Agent("codex".to_owned()),
            Route::Universal,
            Route::Sync,
            Route::Settings,
        ] {
            assert_eq!(labels(route), ["Page"]);
        }
    }

    #[test]
    fn a_skill_page_follows_library_and_links_back() {
        let skill = crumbs(&Route::Skill(SkillId("pdf".into())), "pdf".to_owned());
        assert_eq!(skill[0].target, Some(Route::Library));
        assert_eq!(skill[1].label, "pdf");
    }
}
