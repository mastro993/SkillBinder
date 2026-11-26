//! The summary card (O2, I12): the avatars of the Agents rows whose global skill directories hold
//! the found skills, up to four, and "N skills found in <agents>", in the Agents section's order.
//! Ignored skills are left out; when every found skill is ignored, it reads "No new skills found
//! in <agents> · K ignored", naming the agents that hold the ignored ones.

use super::super::super::NativeView;
use super::super::super::controls::reader_logo;
use super::super::super::import_dialog::versions::found_skills;
use super::super::super::library_copies::{AgentRow, agent_rows};
use gpui_kit::{AnyElement, IntoElement};
use skillbinder_design::{AvatarStack, CardList, ListRow, Logo, Text, format_count};
use skillbinder_proto::{Candidate, ScanSnapshot};
use std::collections::{BTreeMap, BTreeSet};

/// How many found skills not in library are offered and ignored, and the Agents rows that hold
/// the offered ones, or the ignored ones when none is offered.
pub(super) struct Summary<'r> {
    count: usize,
    ignored: usize,
    agents: Vec<&'r AgentRow>,
}

/// The summary of `scan` over the Agents `rows` with the `ignored` keys, counting a skill with
/// several versions once.
pub(super) fn summary<'r>(
    scan: &ScanSnapshot,
    rows: &'r [AgentRow],
    ignored: &BTreeMap<String, String>,
) -> Summary<'r> {
    let (skipped, offered): (Vec<&Candidate>, Vec<&Candidate>) = scan
        .candidates
        .iter()
        .filter(|candidate| candidate.in_library().is_none())
        .partition(|candidate| ignored.contains_key(&candidate.key));
    let named = if offered.is_empty() {
        &skipped
    } else {
        &offered
    };
    let readers: BTreeSet<&str> = named
        .iter()
        .flat_map(|candidate| &candidate.locations)
        .flat_map(|location| &location.global_agents)
        .map(String::as_str)
        .collect();
    Summary {
        count: found_skills(&offered).len(),
        ignored: found_skills(&skipped).len(),
        agents: rows
            .iter()
            .filter(|row| row.ids.iter().any(|id| readers.contains(id.as_str())))
            .collect(),
    }
}

impl Summary<'_> {
    /// Whether any found skill is offered; with none, every found skill is ignored (I12).
    pub(super) fn offers(&self) -> bool {
        self.count > 0
    }

    /// "N skills found in <agents>", or "No new skills found in <agents> · K ignored" when every
    /// found skill is ignored; in your agents when no Agents row holds them, such as skills
    /// reached only through a link.
    pub(super) fn text(&self) -> String {
        let names: Vec<&str> = self.agents.iter().map(|row| row.name.as_str()).collect();
        let place = if names.is_empty() {
            "your agents".to_owned()
        } else {
            names.join(", ")
        };
        match self.count {
            0 => format!(
                "No new skills found in {place} · {} ignored",
                format_count(self.ignored)
            ),
            1 => format!("1 skill found in {place}"),
            count => format!("{} skills found in {place}", format_count(count)),
        }
    }
}

impl NativeView {
    /// The summary of `scan` over the Agents rows, leaving the ignored skills out.
    pub(super) fn found_summary<T>(
        &self,
        scan: &ScanSnapshot,
        show: impl FnOnce(&Summary<'_>) -> T,
    ) -> T {
        let rows = agent_rows(&self.snapshot.agents, &self.snapshot.home);
        show(&summary(scan, &rows, &self.snapshot.ignored))
    }

    /// The summary card of `scan`, one row.
    pub(super) fn summary_card(&self, scan: &ScanSnapshot) -> AnyElement {
        self.found_summary(scan, |summary| {
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
        })
    }
}
