//! A detected agent's page, or Universal's: only the library skills the active preset holds for
//! the agents reading that global skill directory (ADR 0007), as Library rows in one card under the
//! search shared with Library, each with Delete. The header draws the page's head, the active
//! preset's chip, and Add skill; every change on the page edits that preset. With no skill it shows
//! its empty state.

mod head;
mod rows;

pub(super) use rows::AgentPageRow;

use super::agent_skills::{agent_name, detected_agents, split_universal};
use super::controls::Logo;
use super::library_copies::{AgentRow, placements};
use super::library_filters::empty_state;
use super::library_items::{RowEnd, SkillItem};
use super::page_head::EMPTY_TOP;
use super::skill_picker::{Place, Resolved};
use super::snapshot_query::{agent_skills, has_update};
use super::{NativeView, Route};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{AnyElement, App, Context, IntoElement, ParentElement, Styled, div, px};
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
        empty_state(
            logo(row, 20., cx),
            format!("No skills on for {}", row.name),
            cx,
        )
        .pt(px(EMPTY_TOP))
        .child(div().mt(px(8.)).child(add))
        .into_any_element()
    }
}

/// The library skills the page of the detected agents `ids` lists: those the active `preset` holds
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
fn logo(row: &AgentRow, size: f32, cx: &App) -> AnyElement {
    div()
        .flex()
        .text_color(cx.theme().foreground)
        .child(Logo::reader(&row.reader).element(size, false))
        .into_any_element()
}

/// The detected agents reading `resolved`, other than `except`, in the Agents section's order.
fn readers<'a>(
    agents: &'a [AgentOption],
    resolved: &str,
    except: Option<&str>,
) -> Vec<&'a AgentOption> {
    detected_agents(agents)
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
/// agent's page becomes Universal's, and the page of an agent that is no longer detected opens
/// Discovery, as a removed workspace's does.
pub(super) fn agent_fallback(agents: &[AgentOption], home: &str, route: Route) -> Route {
    let (universal, others) = split_universal(detected_agents(agents), home);
    match route {
        Route::Agent(id) if universal.iter().any(|agent| agent.id == id) => Route::Universal,
        Route::Agent(id) if !others.iter().any(|agent| agent.id == id) => Route::Discovery,
        Route::Universal if universal.is_empty() => Route::Discovery,
        route => route,
    }
}

/// `agents` as the Agents section names them.
fn names(agents: &[&AgentOption], home: &str) -> Vec<String> {
    agents.iter().map(|agent| agent_name(agent, home)).collect()
}

#[cfg(test)]
mod tests;
