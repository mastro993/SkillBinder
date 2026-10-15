//! The header breadcrumb: the current page's title under its sidebar section.

use super::agent_skills::{UNIVERSAL, agent_name};
use super::{LibraryFilter, NativeView, Route};
use gpui_kit::component::breadcrumb::{Breadcrumb, BreadcrumbItem};
use gpui_kit::{Context, IntoElement, Styled};
use skillbinder_proto::AppSnapshot;

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
        Route::Library(LibraryFilter::Preset(_)) | Route::Skill(_) => Some(Crumb {
            label: "Library".to_owned(),
            target: Some(Route::Library(LibraryFilter::All)),
        }),
        Route::Workspace(_) => Some(Crumb::text("Workspaces")),
        Route::Agent(_) | Route::Universal => Some(Crumb::text("Agents")),
        Route::Discovery | Route::Library(LibraryFilter::All) | Route::Sync | Route::Settings => {
            None
        }
    };
    section.into_iter().chain([Crumb::text(title)]).collect()
}

/// The current page's own name, as the sidebar shows it.
fn title(route: &Route, snapshot: &AppSnapshot) -> String {
    match route {
        Route::Discovery => "Discovery".to_owned(),
        Route::Library(LibraryFilter::All) => "Library".to_owned(),
        Route::Library(LibraryFilter::Preset(id)) => snapshot
            .library
            .as_ref()
            .and_then(|library| library.library.presets.iter().find(|f| &f.id == id))
            .map(|preset| preset.name.clone())
            .unwrap_or_default(),
        Route::Skill(id) => snapshot
            .library
            .as_ref()
            .and_then(|library| library.library.skills.iter().find(|skill| &skill.id == id))
            .map(|skill| {
                skill
                    .display_name
                    .clone()
                    .unwrap_or_else(|| skill.slug.clone())
            })
            .unwrap_or_default(),
        Route::Workspace(id) => snapshot
            .workspaces
            .iter()
            .find(|workspace| &workspace.id == id)
            .map(|workspace| workspace.name.clone())
            .unwrap_or_default(),
        Route::Agent(id) => snapshot
            .agents
            .iter()
            .find(|agent| &agent.id == id)
            .map_or_else(|| id.clone(), |agent| agent_name(agent, &snapshot.home)),
        Route::Universal => UNIVERSAL.to_owned(),
        Route::Sync => "Sync".to_owned(),
        Route::Settings => "Settings".to_owned(),
    }
}

impl NativeView {
    pub(super) fn render_breadcrumb(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let weak = cx.entity().downgrade();
        let steps = crumbs(&self.route, title(&self.route, &self.snapshot));
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
    use skillbinder_proto::{PresetId, SkillId, WorkspaceId};

    fn labels(route: Route) -> Vec<String> {
        crumbs(&route, "Page".to_owned())
            .into_iter()
            .map(|crumb| crumb.label)
            .collect()
    }

    #[test]
    fn top_level_pages_are_their_title_alone() {
        for route in [
            Route::Discovery,
            Route::Library(LibraryFilter::All),
            Route::Sync,
            Route::Settings,
        ] {
            assert_eq!(labels(route), ["Page"]);
        }
    }

    #[test]
    fn nested_pages_follow_their_section_and_only_library_links_back() {
        assert_eq!(
            labels(Route::Workspace(WorkspaceId::new())),
            ["Workspaces", "Page"]
        );
        assert_eq!(labels(Route::Agent("codex".to_owned())), ["Agents", "Page"]);
        assert_eq!(labels(Route::Universal), ["Agents", "Page"]);
        let preset = crumbs(
            &Route::Library(LibraryFilter::Preset(PresetId::new())),
            "Research".to_owned(),
        );
        assert_eq!(preset[0].target, Some(Route::Library(LibraryFilter::All)));
        let skill = crumbs(&Route::Skill(SkillId::new()), "pdf".to_owned());
        assert_eq!(skill[0].target, Some(Route::Library(LibraryFilter::All)));
        assert_eq!(preset[1], Crumb::text("Research"));
        assert_eq!(
            crumbs(&Route::Workspace(WorkspaceId::new()), String::new())[0].target,
            None
        );
    }
}
