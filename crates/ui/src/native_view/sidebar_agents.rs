//! The sidebar Agents section: Universal first, then the other detected agents alphabetically,
//! each opening its own page.

use super::agent_skills::{
    FoundSkill, UNIVERSAL, agent_name, detected_agents, found_where, read_globally, split_universal,
};
use super::controls::{agent_icon, app_icon};
use super::sidebar_row::{activate, count, empty_row, row};
use super::{NativeView, Route};
use gpui_kit::component::Icon;
use gpui_kit::{
    AnyElement, Context, ElementId, InteractiveElement, IntoElement, ParentElement, Styled, div,
    prelude::FluentBuilder, px,
};
use skillbinder_proto::{AgentOption, CandidateLocation, SidebarSection};
use skillbinder_theme::sidebar::ROW_GAP;

impl NativeView {
    pub(super) fn render_agents(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let home = &self.snapshot.home;
        let (universal, others) = split_universal(detected_agents(&self.snapshot.agents), home);
        let universal_row = (!universal.is_empty()).then(|| {
            self.render_agent_row(
                "agents-universal",
                app_icon("layers"),
                UNIVERSAL.to_owned(),
                Route::Universal,
                self.found_for(&universal).len(),
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
        let empty = rows.is_empty().then(|| empty_row("No agents", cx));
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
        activate(
            row(id, icon, None, name.clone(), selected, cx),
            name,
            cx,
            move |view, window, cx| view.navigate(route.clone(), window, cx),
        )
        .child(count(skills, cx))
        .into_any_element()
    }

    /// The skill folders in the agent's global skill directory.
    pub(super) fn agent_skills(&self, agent: &AgentOption) -> Vec<FoundSkill> {
        self.found_for(&[agent])
    }

    /// The skill folders the last scan found in the global skill directory of any of `agents`.
    pub(super) fn found_for(&self, agents: &[&AgentOption]) -> Vec<FoundSkill> {
        self.found_where(|location| read_globally(location, agents))
    }

    /// The skill folders the last scan found at the locations `holds` accepts.
    pub(super) fn found_where(
        &self,
        holds: impl Fn(&CandidateLocation) -> bool,
    ) -> Vec<FoundSkill> {
        let candidates = self
            .snapshot
            .scan
            .as_ref()
            .map_or(&[][..], |scan| &scan.candidates);
        found_where(candidates, holds)
    }
}
