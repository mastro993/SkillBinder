//! The Choose your agents step (design A1 to A3, P3): the lead and one tile per Agents row of every
//! agent found, Universal first, all ticked until the choice is saved; Continue saves it, ticked
//! or not, and opens the import. With no agent found it says SkillBinder will ask when it finds
//! one.

#[cfg(test)]
mod tests;
mod tile;

use super::super::controls::{Inactive, text_button};
use super::super::library_copies::{AgentRow, Reader, found_rows};
use super::super::{Effect, NativeView};
use super::StepView;
use gpui_kit::component::{
    ActiveTheme, Size, button::ButtonVariants, h_flex, scroll::ScrollableElement, v_flex,
};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};
use skillbinder_proto::AgentOption;
use std::collections::BTreeSet;

const TITLE: &str = "Choose your agents";
const LEAD: &str = "SkillBinder adds and removes skills only in the agents you choose. You can \
                    change this later in Settings.";
const NONE_FOUND: &str =
    "No agents found on this computer. SkillBinder will ask when it finds one.";
/// The grid's tallest, past which it scrolls so the buttons stay in view.
const GRID_HEIGHT: f32 = 312.;

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
        let continue_button = text_button("onboarding-next", "Continue", Size::Medium)
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
            body,
            leading: None,
            actions: vec![continue_button.into_any_element()],
        }
    }

    /// Two columns of tiles, Universal spanning both, scrolling past [`GRID_HEIGHT`].
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
        let (wide, pairs): (Vec<&AgentRow>, Vec<&AgentRow>) = rows
            .iter()
            .partition(|row| matches!(row.reader, Reader::Universal(_)));
        let mut lines: Vec<AnyElement> = wide.into_iter().map(&mut tile).collect();
        for pair in pairs.chunks(2) {
            let mut line = h_flex().gap(px(8.)).items_stretch();
            for row in pair {
                line = line.child(div().flex_1().min_w_0().flex().child(tile(row)));
            }
            if pair.len() == 1 {
                line = line.child(div().flex_1());
            }
            lines.push(line.into_any_element());
        }
        div()
            .id("onboarding-agents")
            .max_h(px(GRID_HEIGHT))
            .min_h_0()
            .child(v_flex().gap(px(8.)).children(lines))
            .overflow_y_scrollbar()
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
