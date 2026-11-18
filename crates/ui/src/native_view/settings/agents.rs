//! Settings › Agents (design S1, S2, P4): a bordered list with one row per Agents row of every
//! agent found, chosen or not, Universal first. Each row has the logo, the name, the folder or
//! Universal's agents, and a trailing Kit switch. Turning one on applies the active preset to it
//! under the progress strip, which keeps every switch disabled until it ends; turning one off saves
//! at once, and its row reads "Its skills stay in place." in place of its folder until the app
//! quits. An agent never on keeps its folder line.

#[cfg(test)]
mod tests;

use super::super::controls::{Logo, tooltip};
use super::super::library_avatars::{Ring, avatar};
use super::super::library_copies::{AgentRow, Reader, found_rows, shown_dir};
use super::super::progress_strip::{Activity, Landing};
use super::super::{Effect, NativeView};
use gpui_kit::component::{ActiveTheme, Disableable, h_flex, switch::Switch, v_flex};
use gpui_kit::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Text, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::AgentOption;
use std::collections::BTreeSet;

const TURNED_OFF: &str = "Its skills stay in place.";
const NONE_FOUND: &str = "No agents found on this computer.";

/// Whether the agents of `row` are yours: they share one global skill directory, so all or none.
fn chosen(row: &AgentRow, agents: &[AgentOption]) -> bool {
    agents
        .iter()
        .any(|agent| agent.chosen && row.ids.contains(&agent.id))
}

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
    /// The Agents list, or the line saying no agent was found, inside the list frame.
    pub(super) fn agents_section(&self, cx: &mut Context<Self>) -> AnyElement {
        let rows = found_rows(&self.snapshot.agents, &self.snapshot.home);
        let theme = cx.theme();
        let list = v_flex()
            .id("settings-agents")
            .border_1()
            .border_color(theme.border)
            .rounded(theme.radius_lg)
            .overflow_hidden();
        if rows.is_empty() {
            let muted = theme.muted_foreground;
            return list
                .child(div().p(px(12.)).text_color(muted).child(NONE_FOUND))
                .into_any_element();
        }
        let lines: Vec<AnyElement> = rows
            .iter()
            .enumerate()
            .map(|(index, row)| self.agent_switch_row(row, index > 0, cx))
            .collect();
        list.children(lines).into_any_element()
    }

    /// One row: logo, name, folder line, and the switch; `divided` draws the line above it.
    fn agent_switch_row(
        &self,
        row: &AgentRow,
        divided: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let on = chosen(row, &self.snapshot.agents);
        let detail = detail(row, on, &self.agents_turned_off, &self.snapshot.home);
        let mono = matches!(row.reader, Reader::Agent(_)) && detail != TURNED_OFF;
        let theme = cx.theme();
        let ring = Ring {
            rest: theme.background,
            hovered: None,
        };
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
        h_flex()
            .gap(px(12.))
            .px(px(12.))
            .py(px(10.))
            .when(divided, |line| line.border_t_1().border_color(theme.border))
            .child(
                div()
                    .flex_none()
                    .child(avatar(Logo::reader(&row.reader), ring, cx)),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(2.))
                    .child(
                        div()
                            .truncate()
                            .font_weight(FontWeight::MEDIUM)
                            .child(Text::new(
                                format!("{id}-name").into(),
                                row.name.clone().into(),
                            )),
                    )
                    .child(
                        div()
                            .id(format!("{id}-detail"))
                            .truncate()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .when(mono, |text| {
                                text.font_family(theme.mono_font_family.clone())
                            })
                            .child(detail.clone())
                            .when(!mono, |text| text.tooltip(tooltip(detail))),
                    ),
            )
            .child(toggle)
            .into_any_element()
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
