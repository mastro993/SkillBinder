//! The sidebar Agents section: Universal first, then the other installed agents alphabetically,
//! each opening its own page.

use std::path::Path;

use super::agent_skills::{
    InstalledSkill, UNIVERSAL, agent_name, installed_agents, installed_skills, split_universal,
    universal_dir,
};
use super::controls::{agent_icon, app_icon};
use super::sidebar_row::{activate, count, row, spinner};
use super::{NativeView, Route};
use gpui_kit::component::{ActiveTheme, Icon};
use gpui_kit::{
    AnyElement, Context, ElementId, InteractiveElement, IntoElement, ParentElement, Styled, Window,
    div, prelude::FluentBuilder, px,
};
use skillbinder_proto::{AgentOption, DeploymentTarget, SidebarSection};
use skillbinder_theme::sidebar::{LABEL_SIZE, ROW_GAP, ROW_HEIGHT, ROW_PADDING};

impl NativeView {
    pub(super) fn render_agents(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let home = &self.snapshot.home;
        let (universal, others) = split_universal(
            installed_agents(&self.snapshot.agents, &self.snapshot.global_agents),
            home,
        );
        let universal_row = (!universal.is_empty()).then(|| {
            let dir = universal_dir(home);
            self.render_agent_row(
                "agents-universal",
                app_icon("layers"),
                UNIVERSAL.to_owned(),
                Route::Universal,
                self.installed_in(&dir).len(),
                cx,
            )
        });
        let rows: Vec<AnyElement> = universal_row
            .into_iter()
            .chain(others.iter().map(|agent| {
                self.render_agent_row(
                    format!("agent-{}", agent.id),
                    agent_icon(&agent.id),
                    agent_name(agent, home),
                    Route::Agent(agent.id.clone()),
                    self.agent_skills(agent).len(),
                    cx,
                )
            }))
            .collect();
        let items = rows.len().max(1);
        let empty = rows.is_empty().then(|| {
            div()
                .flex()
                .items_center()
                .h(px(ROW_HEIGHT))
                .px(px(ROW_PADDING))
                .text_size(px(LABEL_SIZE))
                .text_color(cx.theme().muted_foreground.opacity(0.65))
                .child("No agents")
        });
        div()
            .flex()
            .flex_col()
            .child(self.section_header(SidebarSection::Agents, "Agents", None, cx))
            .children(
                self.section_rows(
                    SidebarSection::Agents,
                    items,
                    div()
                        .id("sidebar-agents")
                        .flex()
                        .flex_col()
                        .gap(px(ROW_GAP))
                        .children(rows)
                        .when_some(empty, |list, empty| list.child(empty)),
                ),
            )
    }

    fn render_agent_row(
        &self,
        id: impl Into<ElementId>,
        icon: Icon,
        name: String,
        route: Route,
        skills: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let selected = self.route == route;
        let trailing = if self.snapshot.rescanning.contains(&DeploymentTarget::Global) {
            spinner(cx)
        } else {
            count(skills, cx)
        };
        activate(
            row(id, icon, None, name.clone(), selected, cx),
            name,
            cx,
            move |view, window, cx| view.open_agent(route.clone(), window, cx),
        )
        .child(trailing)
        .into_any_element()
    }

    /// Opens an agent's or Universal's page and rescans Global, which holds their skills, in the
    /// background.
    fn open_agent(&mut self, route: Route, window: &mut Window, cx: &mut Context<Self>) {
        self.navigate(route, window, cx);
        self.client.spawn_rescan(DeploymentTarget::Global);
    }

    /// The skill folders in the agent's global skill directory.
    pub(super) fn agent_skills(&self, agent: &AgentOption) -> Vec<InstalledSkill> {
        agent
            .global_skills_dir
            .as_deref()
            .map_or_else(Vec::new, |dir| self.installed_in(Path::new(dir)))
    }

    /// The skill folders in a global skill directory.
    pub(super) fn installed_in(&self, dir: &Path) -> Vec<InstalledSkill> {
        installed_skills(
            dir,
            &self.snapshot.deployments,
            &self.snapshot.importables,
            |skill| self.skill_slug(skill),
        )
    }
}
