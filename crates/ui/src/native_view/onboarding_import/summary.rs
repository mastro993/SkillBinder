//! The summary card (O2): the avatars of the Agents rows whose global skill directories hold the
//! found skills, up to four, and "N skills found in <agents>", in the Agents section's order.

use super::super::NativeView;
use super::super::controls::Logo;
use super::super::import_dialog::versions::found_skills;
use super::super::library_avatars::{Ring, stack};
use super::super::library_copies::{AgentRow, agent_rows};
use super::super::sidebar_row::format_count;
use gpui_kit::component::{ActiveTheme, h_flex};
use gpui_kit::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Styled, Text,
    div, px,
};
use skillbinder_proto::{Candidate, ScanSnapshot};
use std::collections::BTreeSet;

/// How many found skills are not in library, and the Agents rows that hold them.
pub(super) struct Summary<'r> {
    count: usize,
    agents: Vec<&'r AgentRow>,
}

/// The summary of `scan` over the Agents `rows`, counting a skill with several versions once.
pub(super) fn summary<'r>(scan: &ScanSnapshot, rows: &'r [AgentRow]) -> Summary<'r> {
    let listed: Vec<&Candidate> = scan
        .candidates
        .iter()
        .filter(|candidate| candidate.in_library().is_none())
        .collect();
    let readers: BTreeSet<&str> = listed
        .iter()
        .flat_map(|candidate| &candidate.locations)
        .flat_map(|location| &location.global_agents)
        .map(String::as_str)
        .collect();
    Summary {
        count: found_skills(&listed).len(),
        agents: rows
            .iter()
            .filter(|row| row.ids.iter().any(|id| readers.contains(id.as_str())))
            .collect(),
    }
}

impl Summary<'_> {
    /// "N skills found in <agents>", or in your agents when no Agents row holds them, such as
    /// skills reached only through a link.
    pub(super) fn text(&self) -> String {
        let skills = match self.count {
            1 => "1 skill".to_owned(),
            count => format!("{} skills", format_count(count)),
        };
        let names: Vec<&str> = self.agents.iter().map(|row| row.name.as_str()).collect();
        let place = if names.is_empty() {
            "your agents".to_owned()
        } else {
            names.join(", ")
        };
        format!("{skills} found in {place}")
    }
}

impl NativeView {
    /// The bordered summary card of `scan`.
    pub(super) fn summary_card(&self, scan: &ScanSnapshot, cx: &mut Context<Self>) -> AnyElement {
        let rows = agent_rows(&self.snapshot.agents, &self.snapshot.home);
        let summary = summary(scan, &rows);
        let logos: Vec<Logo> = summary
            .agents
            .iter()
            .map(|row| Logo::reader(&row.reader))
            .collect();
        let theme = cx.theme();
        let ring = Ring {
            rest: theme.background,
            hovered: None,
        };
        h_flex()
            .id("onboarding-summary")
            .items_center()
            .gap(px(10.))
            .p(px(12.))
            .border_1()
            .border_color(theme.border)
            .rounded(px(10.))
            .bg(theme.background)
            .child(stack(&logos, ring, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .font_weight(FontWeight::MEDIUM)
                    .child(Text::new("summary".into(), summary.text().into())),
            )
            .into_any_element()
    }
}
