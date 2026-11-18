//! The summary card (O2): the avatars of the Agents rows whose global skill directories hold the
//! found skills, up to four, and "N skills found in <agents>", in the Agents section's order.

use super::super::super::NativeView;
use super::super::super::controls::reader_logo;
use super::super::super::import_dialog::versions::found_skills;
use super::super::super::library_copies::{AgentRow, agent_rows};
use gpui_kit::{AnyElement, IntoElement};
use skillbinder_design::{AvatarStack, CardList, ListRow, Logo, Text, format_count};
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
    /// The summary card of `scan`, one row.
    pub(super) fn summary_card(&self, scan: &ScanSnapshot) -> AnyElement {
        let rows = agent_rows(&self.snapshot.agents, &self.snapshot.home);
        let summary = summary(scan, &rows);
        let logos: Vec<Logo> = summary
            .agents
            .iter()
            .map(|row| reader_logo(&row.reader))
            .collect();
        let row = ListRow::new("onboarding-summary-row")
            .leading(AvatarStack::new(logos))
            .title(Text::labeled("summary", summary.text()));
        CardList::new("onboarding-summary")
            .row(row)
            .into_any_element()
    }
}
