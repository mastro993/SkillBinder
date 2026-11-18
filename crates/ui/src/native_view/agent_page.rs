//! One of your agents' page, or Universal's: only the library skills the active preset holds for
//! the agents reading that global skill directory (ADR 0007), as Library rows in one card under the
//! search shared with Library, each with Delete. The header draws the page's head, the active
//! preset's chip, and Add skill; every change on the page edits that preset. With no skill it shows
//! its empty state.

mod head;
mod rows;

pub(super) use rows::AgentPageRow;

use super::agent_skills::{agent_name, split_universal, your_agents};
use super::controls::reader_logo;
use super::library_copies::{AgentRow, placements};
use super::library_items::{RowEnd, SkillItem};
use super::skill_picker::{Place, Resolved};
use super::snapshot_query::{agent_skills, has_update};
use super::{NativeView, Route};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{AnyElement, App, Context, IntoElement, ParentElement, Styled, div};
use skillbinder_design::{EmptyState, IconSize};
use skillbinder_proto::{AgentOption, AppSnapshot, CopyState, PresetId, Skill};

impl NativeView {
    pub(super) fn render_agent(&self, id: &str, cx: &mut Context<Self>) -> AnyElement {
        self.render_agent_page(Place::Agent(id.to_owned()), cx)
    }

    pub(super) fn render_universal(&self, cx: &mut Context<Self>) -> AnyElement {
        self.render_agent_page(Place::Universal, cx)
    }

    /// The page of `place`, an agent's or Universal's.
    fn render_agent_page(&self, place: Place, cx: &mut Context<Self>) -> AnyElement {
        let Some(Resolved::Agents { row, ids }) = Resolved::of(&self.snapshot, &place) else {
            return div().into_any_element();
        };
        let preset = self.active_preset_id();
        let held = page_skills(&self.snapshot, preset.as_ref(), &ids);
        let page = self.scoped_page().children(self.render_update_outcome(cx));
        if held.is_empty() {
            return page
                .child(self.empty_agent(&place, &row, cx))
                .into_any_element();
        }
        self.scoped_list(
            page,
            held,
            |view, shown, cx| view.agent_list(&place, &row, shown, cx),
            cx,
        )
    }

    /// The rows of `skills` in one card.
    fn agent_list(
        &self,
        place: &Place,
        row: &AgentRow,
        skills: &[&Skill],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(library) = &self.snapshot.library else {
            return div().into_any_element();
        };
        // The review glyph counts the copies that need review in every scope.
        let placed = placements(&library.copies, &[]);
        let items: Vec<SkillItem<'_>> = skills
            .iter()
            .map(|skill| SkillItem {
                skill,
                details: library.details.get(&skill.id),
                folder: None,
                update: has_update(&self.snapshot, &skill.id),
                drifted: skill.drift.is_some(),
                placement: placed.get(&skill.id),
                end: RowEnd::Agent(AgentPageRow {
                    place,
                    name: &row.name,
                    edited: library.copies.iter().any(|copy| {
                        copy.skill == skill.id
                            && copy.directory == row.resolved
                            && matches!(copy.state, CopyState::Edited { .. })
                    }),
                }),
            })
            .collect();
        self.skill_list(&items, false, cx)
    }

    /// The page's logo, "No skills on for <name>", and Add skill.
    fn empty_agent(&self, place: &Place, row: &AgentRow, cx: &mut Context<Self>) -> AnyElement {
        let add = self.add_skill_button(place.clone(), "add-skill-empty", cx);
        EmptyState::new("agent-empty", format!("No skills on for {}", row.name))
            .media(logo(row, IconSize::Lg, cx))
            .action(add)
            .into_any_element()
    }
}

/// The library skills the page of your agents `ids` lists: those the active `preset` holds
/// for any of them, by name. The sidebar counts the same list.
pub(super) fn page_skills<'a>(
    snapshot: &'a AppSnapshot,
    preset: Option<&PresetId>,
    ids: &[String],
) -> Vec<&'a Skill> {
    preset.map_or_else(Vec::new, |preset| agent_skills(snapshot, preset, ids))
}

/// `row`'s logo at `size`, bare: Universal's `layers` glyph, an agent's color art where it has
/// one, else its monochrome logo or the square terminal glyph, in the text color.
fn logo(row: &AgentRow, size: IconSize, cx: &App) -> AnyElement {
    div()
        .flex()
        .text_color(cx.theme().foreground)
        .child(reader_logo(&row.reader).element(size, false))
        .into_any_element()
}

/// Your agents reading `resolved`, other than `except`, in the Agents section's order.
fn readers<'a>(
    agents: &'a [AgentOption],
    resolved: &str,
    except: Option<&str>,
) -> Vec<&'a AgentOption> {
    your_agents(agents)
        .into_iter()
        .filter(|other| Some(other.id.as_str()) != except)
        .filter(|other| {
            other
                .resolved_skills_dir
                .as_ref()
                .or(other.global_skills_dir.as_ref())
                .is_some_and(|dir| dir == resolved)
        })
        .collect()
}

/// Where an agent's or Universal's page goes once `agents` no longer lists it: a universal
/// agent's page becomes Universal's, and the page of an agent that is no longer yours, or
/// Universal's with no universal agent left, opens Library, as a removed workspace's does.
pub(super) fn agent_fallback(agents: &[AgentOption], home: &str, route: Route) -> Route {
    let (universal, others) = split_universal(your_agents(agents), home);
    match route {
        Route::Agent(id) if universal.iter().any(|agent| agent.id == id) => Route::Universal,
        Route::Agent(id) if !others.iter().any(|agent| agent.id == id) => Route::Library,
        Route::Universal if universal.is_empty() => Route::Library,
        route => route,
    }
}

/// `agents` as the Agents section names them.
fn names(agents: &[&AgentOption], home: &str) -> Vec<String> {
    agents.iter().map(|agent| agent_name(agent, home)).collect()
}

#[cfg(test)]
mod tests;
