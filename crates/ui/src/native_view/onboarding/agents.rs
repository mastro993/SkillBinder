//! The Choose your agents step (design A1 to A3, P3): the lead and one tile per Agents row of every
//! agent found, Universal first, all ticked until the choice is saved; Continue saves it, ticked
//! or not, and opens the import. With no agent found it says SkillBinder will ask when it finds
//! one.

#[cfg(test)]
mod tests;
mod tile;

use super::super::library_copies::{AgentRow, Reader, found_rows};
use super::super::{Effect, NativeView};
use super::StepView;
use gpui_kit::component::{ActiveTheme, h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};
use skillbinder_design::Button;
use skillbinder_design::tokens::space;
use skillbinder_proto::AgentOption;
use std::collections::BTreeSet;

const TITLE: &str = "Choose your agents";
const LEAD: &str = "SkillBinder adds and removes skills only in the agents you choose. You can \
                    change this later in Settings.";
const NONE_FOUND: &str =
    "No agents found on this computer. SkillBinder will ask when it finds one.";
/// Tiles per row; Universal spans them all (design A2, P6).
const COLUMNS: usize = 3;

/// The rows a saved choice leaves unticked: those none of whose agents is yours. Before the
/// choice is saved every row is ticked.
fn unticked(rows: &[AgentRow], agents: &[AgentOption], saved: bool) -> BTreeSet<String> {
    if !saved {
        return BTreeSet::new();
    }
    rows.iter()
        .filter(|row| {
            !agents
                .iter()
                .any(|agent| agent.chosen && row.ids.contains(&agent.id))
        })
        .map(|row| row.dir.clone())
        .collect()
}

/// What Continue saves: every row's global skill directory shown, and those ticked.
fn choice(rows: &[AgentRow], unticked: &BTreeSet<String>) -> (BTreeSet<String>, BTreeSet<String>) {
    let shown: BTreeSet<String> = rows.iter().map(|row| row.dir.clone()).collect();
    let chosen = shown.difference(unticked).cloned().collect();
    (shown, chosen)
}

impl NativeView {
    /// The step; `saved` once the choice was saved and the step gone back to, when its tiles
    /// show that choice.
    pub(super) fn agents_step(&mut self, saved: bool, cx: &mut Context<Self>) -> StepView {
        let rows = found_rows(&self.snapshot.agents, &self.snapshot.home);
        let unticked = self
            .unticked_agents
            .get_or_insert_with(|| unticked(&rows, &self.snapshot.agents, saved))
            .clone();
        let muted = cx.theme().muted_foreground;
        let lead = |text: &'static str| div().text_color(muted).child(text).into_any_element();
        let body = if rows.is_empty() {
            vec![lead(NONE_FOUND)]
        } else {
            vec![lead(LEAD), self.agent_grid(&rows, &unticked, cx)]
        };
        let (shown, chosen) = choice(&rows, &unticked);
        let continue_button = Button::new("onboarding-next")
            .label("Continue")
            .primary()
            .inactive(self.blocked())
            .on_click(cx.listener(move |view, _, _, cx| {
                let client = view.client.clone();
                let (shown, chosen) = (shown.clone(), chosen.clone());
                view.run(
                    async move {
                        client.choose_agents(shown, chosen).await?;
                        Ok(Effect::AgentsChosen)
                    },
                    cx,
                );
            }));
        StepView {
            title: TITLE,
            wide: true,
            body,
            leading: None,
            actions: vec![continue_button.into_any_element()],
        }
    }

    /// [`COLUMNS`] columns of tiles, Universal spanning them all. The frame scrolls the step when
    /// they do not fit, so the grid itself never scrolls (design A4).
    fn agent_grid(
        &self,
        rows: &[AgentRow],
        unticked: &BTreeSet<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let home = self.snapshot.home.clone();
        let mut tile = |row: &AgentRow| {
            let chosen = !unticked.contains(&row.dir);
            self.agent_tile(row, chosen, &home, cx)
        };
        let (wide, others): (Vec<&AgentRow>, Vec<&AgentRow>) = rows
            .iter()
            .partition(|row| matches!(row.reader, Reader::Universal(_)));
        let mut lines: Vec<AnyElement> = wide.into_iter().map(&mut tile).collect();
        for chunk in others.chunks(COLUMNS) {
            let line = h_flex()
                .gap(px(space::MD))
                .items_stretch()
                .children(
                    chunk
                        .iter()
                        .map(|row| div().flex_1().min_w_0().flex().child(tile(row))),
                )
                .children((chunk.len()..COLUMNS).map(|_| div().flex_1()));
            lines.push(line.into_any_element());
        }
        v_flex()
            .id("onboarding-agents")
            .gap(px(space::MD))
            .children(lines)
            .into_any_element()
    }

    /// Ticks or unticks the row reading `dir`.
    fn toggle_agent(&mut self, dir: &str, cx: &mut Context<Self>) {
        if self.blocked() {
            return;
        }
        let unticked = self.unticked_agents.get_or_insert_default();
        if !unticked.remove(dir) {
            unticked.insert(dir.to_owned());
        }
        cx.notify();
    }
}
