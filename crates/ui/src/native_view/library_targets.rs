//! Which agents the active preset holds a library skill for: the Active checkbox holds it for all
//! your agents, and the trailing agents menu picks specific ones. Universal is one row, since its
//! agents share one directory.

use std::collections::BTreeSet;

use super::NativeView;
use super::controls::{agent_icon, app_icon};
use super::library_copies::{AgentRow, Reader, agent_rows};
use super::menu::{MenuItem, OpenMenu};
use super::sidebar_presets::menu_action;
use gpui_kit::component::{ActiveTheme, Sizable, h_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, ParentElement, StatefulInteractiveElement, Styled, prelude::FluentBuilder, px,
};
use skillbinder_proto::{OrganizationChange, SkillId, Targets};
use skillbinder_theme::sidebar::ICON_SIZE;

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

    /// The Active checkbox and the agents menu's button, at a row's or card's trailing edge.
    pub(super) fn activation_controls(
        &self,
        skill: &SkillId,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        h_flex()
            // Enter or Space on the focused checkbox toggles it rather than opening the skill.
            .on_key_down(|event: &KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    cx.stop_propagation();
                }
            })
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .child(self.activation_toggle(skill, cx))
            .child(self.targets_button(skill, cx))
            .into_any_element()
    }

    /// "All agents", one row's name, or how many rows the skill is held for, with a chevron that
    /// opens the agents menu.
    pub(super) fn targets_button(&self, skill: &SkillId, cx: &mut Context<Self>) -> AnyElement {
        let rows = agent_rows(&self.snapshot.agents, &self.snapshot.home);
        let label = targets_label(self.skill_targets(skill), &rows);
        let owner = format!("targets-{skill}");
        let open = self.menu.as_ref().is_some_and(|menu| menu.is_for(&owner));
        let (busy, fill) = (self.busy, cx.theme().muted);
        let skill = skill.clone();
        h_flex()
            .id(owner.clone())
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .px(px(6.))
            .py(px(2.))
            .rounded(px(6.))
            .text_size(px(12.))
            .text_color(cx.theme().muted_foreground)
            .aria_label(format!("Agents: {label}"))
            .when(open, |button| button.bg(fill))
            .when(!busy, |button| {
                button.cursor_pointer().hover(move |button| button.bg(fill))
            })
            .child(label)
            .child(app_icon("arrow-down").xsmall())
            // The press opens the menu; the click must not open the skill's page.
            .on_click(|_, _, cx| cx.stop_propagation())
            .when(!busy, |button| {
                button.on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        let items = view.targets_menu(&skill, cx);
                        view.menu = Some(OpenMenu::new(
                            owner.clone(),
                            items,
                            event.position,
                            view.focus.clone(),
                            window,
                            cx,
                            |view: &mut NativeView| &mut view.menu,
                        ));
                    }),
                )
            })
            .into_any_element()
    }

    /// All agents, then a row for Universal and each other detected agent, checked while the
    /// active preset holds the skill for it. Choosing a row adds or removes its agents.
    fn targets_menu(&self, skill: &SkillId, cx: &mut Context<Self>) -> Vec<MenuItem> {
        let view = cx.entity().downgrade();
        let rows = agent_rows(&self.snapshot.agents, &self.snapshot.home);
        let targets = self.skill_targets(skill).cloned();
        let every: Vec<String> = rows.iter().flat_map(|row| row.ids.clone()).collect();
        let busy = self.busy;
        let choose = |key: String, label: String, next: Targets| {
            MenuItem::new(key, label)
                .disabled(busy)
                .on_select(menu_action(
                    view.clone(),
                    skill.clone(),
                    move |view, skill, _, cx| view.set_targets(skill.clone(), next.clone(), cx),
                ))
        };
        let mut items = vec![
            choose(
                "targets-all".into(),
                "All agents".into(),
                Targets::AllAgents,
            )
            .checked(targets == Some(Targets::AllAgents)),
        ];
        if !rows.is_empty() {
            items.push(MenuItem::separator());
        }
        for row in &rows {
            let ids = sharing(row, &rows);
            let next = toggled(targets.as_ref(), &ids, &every);
            let item = match &row.reader {
                Reader::Universal(_) => {
                    choose("targets-universal".into(), row.name.clone(), next).icon("layers")
                }
                Reader::Agent(id) => {
                    let logo = id.clone();
                    choose(format!("targets-agent-{id}"), row.name.clone(), next)
                        .leading(move |_| agent_icon(&logo).size(px(ICON_SIZE)).into_any_element())
                }
            };
            items.push(item.checked(held(targets.as_ref(), &ids)));
        }
        items
    }

    /// Holds `skill` in the active preset for `targets`; no agents removes it from the preset.
    fn set_targets(&mut self, skill: SkillId, targets: Targets, cx: &mut Context<Self>) {
        let Some(id) = self.active_preset_id() else {
            return;
        };
        self.change_organization(OrganizationChange::SetTargets { id, skill, targets }, cx);
    }
}

/// The agents of `row` and of every other row reading the same directory, such as an agent whose
/// global skill directory links to the universal one: the target is the directory, so they are
/// chosen together.
fn sharing(row: &AgentRow, rows: &[AgentRow]) -> Vec<String> {
    rows.iter()
        .filter(|other| other.resolved == row.resolved)
        .flat_map(|other| other.ids.iter().cloned())
        .collect()
}

/// Whether `targets` hold the skill for any of `ids`.
fn held(targets: Option<&Targets>, ids: &[String]) -> bool {
    targets.is_some_and(|targets| ids.iter().any(|id| targets.includes(id)))
}

/// The targets after choosing a row reading for `ids`: they leave when the skill is held for them
/// and join otherwise. Leaving all agents keeps every other detected agent as a specific list,
/// which never gains agents; `every` lists the detected agents.
fn toggled(targets: Option<&Targets>, ids: &[String], every: &[String]) -> Targets {
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

/// The agents button's label.
fn targets_label(targets: Option<&Targets>, rows: &[AgentRow]) -> String {
    let Some(Targets::Agents(_)) = targets else {
        return "All agents".to_owned();
    };
    let named: Vec<_> = rows
        .iter()
        .filter(|row| held(targets, &sharing(row, rows)))
        .collect();
    match named.as_slice() {
        [] => "No agents".to_owned(),
        [row] => row.name.clone(),
        rows => format!("{} agents", rows.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn the_label_names_all_agents_one_row_or_a_count() {
        let rows = [
            row("Universal", &["amp", "codex"]),
            row("Claude Code", &["claude-code"]),
        ];
        assert_eq!(targets_label(None, &rows), "All agents");
        assert_eq!(
            targets_label(Some(&Targets::AllAgents), &rows),
            "All agents"
        );
        assert_eq!(targets_label(Some(&agents(&["codex"])), &rows), "Universal");
        let both = agents(&["amp", "claude-code"]);
        assert_eq!(targets_label(Some(&both), &rows), "2 agents");
        assert_eq!(targets_label(Some(&agents(&["gone"])), &rows), "No agents");
    }
}
