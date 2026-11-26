//! Which agents the active preset holds a library skill for, and the targets after choosing an
//! agent row. Universal is one row, since its agents share one directory.

use std::collections::BTreeSet;

use super::NativeView;
use super::library_copies::AgentRow;
use gpui_kit::Context;
use skillbinder_proto::{OrganizationChange, SkillId, Targets};

impl NativeView {
    /// The agents the active preset holds `skill` for, while it holds it.
    pub(super) fn skill_targets(&self, skill: &SkillId) -> Option<&Targets> {
        let library = self.snapshot.library.as_ref()?;
        let active = self.active_preset(library)?;
        library
            .library
            .skills
            .iter()
            .find(|held| &held.id == skill)?
            .presets
            .get(&active.id)
    }

    /// Holds `skill` in the active preset for `targets`; no agents removes it from the preset.
    pub(super) fn set_targets(&mut self, skill: SkillId, targets: Targets, cx: &mut Context<Self>) {
        let Some(id) = self.active_preset_id() else {
            return;
        };
        self.change_organization(OrganizationChange::SetTargets { id, skill, targets }, cx);
    }
}

/// The agents of `row` and of every other row reading the same directory, such as an agent whose
/// global skill directory links to the universal one: the target is the directory, so they are
/// chosen together.
pub(super) fn sharing(row: &AgentRow, rows: &[AgentRow]) -> Vec<String> {
    rows.iter()
        .filter(|other| other.resolved == row.resolved)
        .flat_map(|other| other.ids.iter().cloned())
        .collect()
}

/// Whether `targets` hold the skill for any of `ids`.
pub(crate) fn held(targets: Option<&Targets>, ids: &[String]) -> bool {
    targets.is_some_and(|targets| ids.iter().any(|id| targets.includes(id)))
}

/// The targets after choosing a row reading for `ids`: they leave when the skill is held for them
/// and join otherwise. Leaving all agents keeps every other one of your agents as a specific list,
/// which never gains agents; `every` lists your agents.
pub(super) fn toggled(targets: Option<&Targets>, ids: &[String], every: &[String]) -> Targets {
    let mut agents: BTreeSet<String> = match targets {
        None => BTreeSet::new(),
        Some(Targets::AllAgents) => every.iter().cloned().collect(),
        Some(Targets::Agents(agents)) => agents.clone(),
    };
    if held(targets, ids) {
        for id in ids {
            agents.remove(id);
        }
    } else {
        agents.extend(ids.iter().cloned());
    }
    Targets::Agents(agents)
}

/// A specific list's label: the one row it holds, how many it holds, or "No agents".
pub(super) fn held_label(named: &[&str]) -> String {
    match named {
        [] => "No agents".to_owned(),
        [name] => (*name).to_owned(),
        names => format!("{} agents", names.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_view::library_copies::Reader;

    fn row(name: &str, ids: &[&str]) -> AgentRow {
        located(name, ids, &format!("/home/{name}"))
    }

    fn located(name: &str, ids: &[&str], resolved: &str) -> AgentRow {
        AgentRow {
            reader: if name == "Universal" {
                Reader::Universal(Vec::new())
            } else {
                Reader::Agent(ids[0].to_owned())
            },
            name: name.to_owned(),
            names: vec![name.to_owned()],
            ids: ids.iter().map(|id| (*id).to_owned()).collect(),
            dir: String::new(),
            resolved: resolved.to_owned(),
        }
    }

    fn agents(ids: &[&str]) -> Targets {
        Targets::Agents(ids.iter().map(|id| (*id).to_owned()).collect())
    }

    #[test]
    fn choosing_a_row_toggles_its_agents_into_a_specific_list() {
        let universal = row("Universal", &["amp", "codex"]);
        let claude = row("Claude Code", &["claude-code"]);
        let every = [
            "amp".to_owned(),
            "codex".to_owned(),
            "claude-code".to_owned(),
        ];
        let all = Some(&Targets::AllAgents);
        assert_eq!(
            toggled(all, &universal.ids, &every),
            agents(&["claude-code"])
        );
        assert_eq!(toggled(None, &claude.ids, &every), agents(&["claude-code"]));
        let only_claude = agents(&["claude-code"]);
        assert_eq!(
            toggled(Some(&only_claude), &universal.ids, &every),
            agents(&["amp", "claude-code", "codex"]),
            "a specific list stays specific even once it names every agent"
        );
        assert_eq!(
            toggled(Some(&only_claude), &claude.ids, &every),
            agents(&[])
        );
    }

    #[test]
    fn rows_reading_one_directory_are_chosen_together() {
        let rows = [
            located("Universal", &["codex"], "/home/.agents/skills"),
            located("Claude Code", &["claude-code"], "/home/.agents/skills"),
            located("Zed", &["zed"], "/home/.zed/skills"),
        ];
        let linked = sharing(&rows[1], &rows);
        assert_eq!(linked, ["codex", "claude-code"]);
        let every = [
            "codex".to_owned(),
            "claude-code".to_owned(),
            "zed".to_owned(),
        ];
        assert_eq!(
            toggled(Some(&Targets::AllAgents), &linked, &every),
            agents(&["zed"])
        );
    }
}
