//! Settings › Agents (design S1, S2, P4): one row per Agents row of every agent found, chosen or
//! not, Universal first. Each row has the logo, the name, the folder or Universal's agents, and a
//! trailing Kit switch. Turning one on applies the active preset to it under the progress strip,
//! which keeps every switch disabled until it ends; turning one off saves at once, and its row
//! reads "Its skills stay in place." in place of its folder until the app quits. An agent never
//! on keeps its folder line.

pub(super) mod mcp;
#[cfg(test)]
mod tests;

use super::super::controls::{agent_detail, reader_logo};
use super::super::library_copies::{AgentRow, Reader, found_rows, shown_dir};
use super::super::progress_strip::{Activity, Landing};
use super::super::{Effect, NativeView};
use super::catalog::{Group, SettingRow};
use gpui_kit::component::{Disableable, switch::Switch};
use gpui_kit::{Context, IntoElement, Styled, px};
use skillbinder_design::tokens::space;
use skillbinder_design::{AvatarStack, Notice};
use std::collections::BTreeSet;

const TURNED_OFF: &str = "Its skills stay in place.";
const NONE_FOUND: &str = "No agents found on this computer.";

/// What a row reads under its name: its folder, or Universal's agents, unless its agents were
/// turned off this session while managed.
fn detail(row: &AgentRow, on: bool, turned_off: &BTreeSet<String>, home: &str) -> String {
    if !on && turned_off.contains(&row.dir) {
        return TURNED_OFF.to_owned();
    }
    match &row.reader {
        Reader::Universal(names) => names.join(", "),
        Reader::Agent(_) => shown_dir(&row.dir, home),
    }
}

/// The element id stem of a row. Not `settings-agent-universal`: that is the registry's own
/// Universal agent.
fn row_id(row: &AgentRow) -> String {
    match &row.reader {
        Reader::Universal(_) => "settings-agents-universal".to_owned(),
        Reader::Agent(id) => format!("settings-agent-{id}"),
    }
}

impl NativeView {
    /// One row per found agent, or the line saying no agent was found, then the MCP server.
    pub(super) fn agents_groups(&self, cx: &mut Context<Self>) -> Vec<Group> {
        let rows = found_rows(&self.snapshot.agents, &self.snapshot.home);
        let rows = rows
            .iter()
            .map(|row| self.agent_switch_row(row, cx))
            .collect();
        let switches = Group {
            empty: Some(
                Notice::new("settings-agents-none", NONE_FOUND)
                    .p(px(space::LG))
                    .into_any_element(),
            ),
            ..Group::new(rows)
        };
        std::iter::once(switches)
            .chain(self.mcp_group(cx))
            .collect()
    }

    /// One row: logo, name, folder line, and the switch.
    fn agent_switch_row(&self, row: &AgentRow, cx: &mut Context<Self>) -> SettingRow {
        let on = row.chosen(&self.snapshot.agents);
        let detail = detail(row, on, &self.agents_turned_off, &self.snapshot.home);
        let mono = matches!(row.reader, Reader::Agent(_)) && detail != TURNED_OFF;
        let id = row_id(row);
        let switched = row.clone();
        let toggle = Switch::new(format!("{id}-switch"))
            .checked(on)
            .disabled(self.blocked())
            .accessibility_label(row.name.clone())
            .on_click(cx.listener(move |view, on: &bool, _, cx| {
                if *on {
                    view.turn_on_agents(vec![switched.clone()], cx);
                } else {
                    view.turn_off_agent(&switched, cx);
                }
            }));
        SettingRow::new(id.clone(), row.name.clone())
            .leading(AvatarStack::new([reader_logo(&row.reader)]))
            .subtitle(
                detail.clone(),
                agent_detail(format!("{id}-detail"), detail, mono, cx),
            )
            .keywords(&["agent", "skill folder"])
            .control(toggle)
    }

    /// Turns on the agents of `rows` under the progress strip, which ends in the toast naming
    /// them; the strip keeps every switch disabled, and the notice down, until it ends.
    pub(in crate::native_view) fn turn_on_agents(
        &mut self,
        rows: Vec<AgentRow>,
        cx: &mut Context<Self>,
    ) {
        if self.blocked() {
            return;
        }
        let dirs: BTreeSet<String> = rows.iter().map(|row| row.dir.clone()).collect();
        for dir in &dirs {
            self.agents_turned_off.remove(dir);
        }
        let agents = rows.into_iter().map(|row| row.name).collect();
        let client = self.client.clone();
        self.block(
            Activity::TurningOn { agents },
            async move {
                client.set_agents_chosen(dirs, true).await?;
                Ok(Landing::default())
            },
            cx,
        );
    }

    /// Turns the agents of `row` off at once, leaving their copies in place.
    fn turn_off_agent(&mut self, row: &AgentRow, cx: &mut Context<Self>) {
        if self.blocked() {
            return;
        }
        self.agents_turned_off.insert(row.dir.clone());
        let client = self.client.clone();
        let dirs = [row.dir.clone()].into();
        self.run(
            async move {
                client.set_agents_chosen(dirs, false).await?;
                Ok(Effect::None)
            },
            cx,
        );
    }
}
