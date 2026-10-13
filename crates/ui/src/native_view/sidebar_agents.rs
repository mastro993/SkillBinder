//! The sidebar Agents section: installed agents, alphabetically, each opening its own page.

use std::path::Path;

use super::agent_skills::{InstalledSkill, installed_agents, installed_skills};
use super::controls::agent_icon;
use super::sidebar_row::{activate, count, row, spinner};
use super::{NativeView, Route};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use skillbinder_proto::{AgentOption, DeploymentTarget, SidebarSection};
use skillbinder_theme::sidebar::{LABEL_SIZE, ROW_GAP, ROW_HEIGHT, ROW_PADDING};

impl NativeView {
    pub(super) fn render_agents(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let agents = installed_agents(&self.snapshot.agents, &self.snapshot.global_agents);
        let rows: Vec<AnyElement> = agents
            .iter()
            .map(|agent| self.render_agent_row(agent, cx))
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

    fn render_agent_row(&self, agent: &AgentOption, cx: &mut Context<Self>) -> AnyElement {
        let route = Route::Agent(agent.id.clone());
        let selected = self.route == route;
        let trailing = if self.snapshot.rescanning.contains(&DeploymentTarget::Global) {
            spinner(cx)
        } else {
            count(self.agent_skills(agent).len(), cx)
        };
        let id = agent.id.clone();
        activate(
            row(
                format!("agent-{}", agent.id),
                agent_icon(&agent.id),
                None,
                agent.display_name.clone(),
                selected,
                cx,
            ),
            agent.display_name.clone(),
            cx,
            move |view, window, cx| view.open_agent(id.clone(), window, cx),
        )
        .child(trailing)
        .into_any_element()
    }

    /// Opens an agent's page and rescans Global, which holds its skills, in the background.
    fn open_agent(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.navigate(Route::Agent(id), window, cx);
        self.client.spawn_rescan(DeploymentTarget::Global);
    }

    /// The skill folders in the agent's global skill directory.
    pub(super) fn agent_skills(&self, agent: &AgentOption) -> Vec<InstalledSkill> {
        let Some(dir) = &agent.global_skills_dir else {
            return Vec::new();
        };
        installed_skills(
            Path::new(dir),
            &self.snapshot.deployments,
            &self.snapshot.importables,
            |skill| self.skill_slug(skill),
        )
    }
}
