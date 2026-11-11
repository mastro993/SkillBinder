//! The sidebar Agents section: Universal first, then the other detected agents alphabetically,
//! each opening its own page.

use super::agent_page::page_skills;
use super::controls::{agent_icon, app_icon};
use super::library_copies::{AgentRow, Reader, agent_rows};
use super::library_targets::sharing;
use super::sidebar_row::{RowIcon, activate, count, row};
use super::{NativeView, Route};
use gpui_kit::component::Icon;
use gpui_kit::{
    AnyElement, Context, ElementId, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};
use skillbinder_proto::{AppSnapshot, PresetId, SidebarSection};
use skillbinder_theme::sidebar::ROW_GAP;

impl NativeView {
    pub(super) fn render_agents(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let preset = self.active_preset_id();
        let rows: Vec<AnyElement> = counted_rows(&self.snapshot, preset.as_ref())
            .into_iter()
            .map(|(agent, skills)| {
                let (id, icon, route): (ElementId, Icon, Route) = match &agent.reader {
                    Reader::Universal(_) => (
                        "agents-universal".into(),
                        app_icon("layers"),
                        Route::Universal,
                    ),
                    Reader::Agent(id) => (
                        format!("agent-{id}").into(),
                        agent_icon(id),
                        Route::Agent(id.clone()),
                    ),
                };
                self.render_agent_row(id, icon, agent.name, route, skills, cx)
            })
            .collect();
        // The section is hidden while no agent is detected.
        if rows.is_empty() {
            return None;
        }
        let items = rows.len();
        let section = div()
            .flex()
            .flex_col()
            .child(self.section_header(SidebarSection::Agents, "Agents", None, false, cx))
            .children(
                self.section_rows(
                    SidebarSection::Agents,
                    items,
                    div()
                        .id("sidebar-agents")
                        .flex()
                        .flex_col()
                        .gap(px(ROW_GAP))
                        .children(rows),
                ),
            );
        Some(section)
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
            row(
                id,
                RowIcon::Glyph(Box::new(icon)),
                None,
                name.clone(),
                selected,
                cx,
            ),
            name,
            cx,
            move |view, window, cx| view.navigate(route.clone(), window, cx),
        )
        .child(count(skills, cx))
        .into_any_element()
    }
}

/// The Agents rows, each with the number of skills its page lists through the active `preset`.
fn counted_rows(snapshot: &AppSnapshot, preset: Option<&PresetId>) -> Vec<(AgentRow, usize)> {
    let rows = agent_rows(&snapshot.agents, &snapshot.home);
    rows.iter()
        .map(|row| {
            let skills = page_skills(snapshot, preset, &sharing(row, &rows)).len();
            (row.clone(), skills)
        })
        .collect()
}

#[cfg(test)]
mod tests;
