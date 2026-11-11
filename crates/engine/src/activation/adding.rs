//! Adding library skills to one place at once: the active preset for your agents, or a workspace.
//! A skill the place holds already keeps what it holds and gains the chosen agents; nothing is
//! removed. The whole batch is one save, one pass over the copies, and the caller's one scan.

use crate::worker::State;
use skillbinder_proto::*;
use std::collections::BTreeSet;

impl State {
    /// Adds `change`'s skills to its place, refusing the whole batch before anything is saved
    /// when it is empty, a skill has no agents, or a skill is not in the library.
    pub(crate) fn add_skills(&mut self, change: AddSkills) -> AppResult<()> {
        self.require_recovered()?;
        match change {
            AddSkills::YourAgents { skills } => {
                refuse(
                    skills
                        .iter()
                        .map(|(skill, agents)| (skill, agents.is_empty())),
                )?;
                self.organize(|library, _| {
                    let active = library.active_preset.clone();
                    for (id, agents) in skills {
                        let skill = library
                            .skills
                            .iter_mut()
                            .find(|skill| skill.id == id)
                            .ok_or_else(|| AppError::not_found("Skill no longer exists."))?;
                        let merged = agents_merged(skill.presets.get(&active), agents);
                        skill.presets.insert(active.clone(), merged);
                    }
                    Ok(())
                })
                .map(|_| ())
            }
            AddSkills::Workspace { id, dirs, skills } => {
                self.change_workspace_skills(&id, |workspace| {
                    let added: BTreeSet<String> = dirs
                        .into_iter()
                        .chain(skills.values().flat_map(|targets| match targets {
                            WorkspaceTargets::AllAgents => BTreeSet::new(),
                            WorkspaceTargets::Dirs(dirs) => dirs.clone(),
                        }))
                        .filter(|dir| !workspace.dirs.contains(dir))
                        .collect();
                    // All agents holds a skill nowhere while the workspace would use no agent.
                    let none = workspace.dirs.is_empty() && added.is_empty();
                    refuse(skills.iter().map(|(skill, targets)| {
                        let empty = match targets {
                            WorkspaceTargets::AllAgents => none,
                            WorkspaceTargets::Dirs(dirs) => dirs.is_empty(),
                        };
                        (skill, empty)
                    }))?;
                    let merged = skills
                        .into_iter()
                        .map(|(skill, targets)| {
                            let merged = workspace_merged(workspace.skills.get(&skill), targets);
                            (skill, Some(merged))
                        })
                        .collect();
                    Ok((added, merged))
                })
            }
        }
    }
}

/// Refuses an empty batch, then the first skill, in slug order, that `empty` says gains no agent.
fn refuse<'a>(skills: impl IntoIterator<Item = (&'a SkillId, bool)>) -> AppResult<()> {
    let mut skills = skills.into_iter().peekable();
    if skills.peek().is_none() {
        return Err(AppError::validation("Select skills to add."));
    }
    match skills.find(|(_, empty)| *empty) {
        Some((skill, _)) => Err(AppError::validation(format!("Pick an agent for {skill}."))),
        None => Ok(()),
    }
}

/// What the active preset holds a skill for once it gains `agents`: all agents stays so.
fn agents_merged(held: Option<&Targets>, agents: BTreeSet<String>) -> Targets {
    match held {
        Some(Targets::AllAgents) => Targets::AllAgents,
        Some(Targets::Agents(held)) => Targets::Agents(held.union(&agents).cloned().collect()),
        None => Targets::Agents(agents),
    }
}

/// What a workspace holds a skill for once it gains `targets`: all agents, held or added, wins.
fn workspace_merged(
    held: Option<&WorkspaceTargets>,
    targets: WorkspaceTargets,
) -> WorkspaceTargets {
    match (held, targets) {
        (Some(WorkspaceTargets::AllAgents), _) | (_, WorkspaceTargets::AllAgents) => {
            WorkspaceTargets::AllAgents
        }
        (Some(WorkspaceTargets::Dirs(held)), WorkspaceTargets::Dirs(dirs)) => {
            WorkspaceTargets::Dirs(held.union(&dirs).cloned().collect())
        }
        (None, targets) => targets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn your_agents_gain_the_ticked_agents_and_all_agents_stays() {
        let claude = || set(&["claude-code"]);
        assert_eq!(agents_merged(None, claude()), Targets::Agents(claude()));
        assert_eq!(
            agents_merged(Some(&Targets::Agents(set(&["codex"]))), claude()),
            Targets::Agents(set(&["claude-code", "codex"]))
        );
        assert_eq!(
            agents_merged(Some(&Targets::AllAgents), claude()),
            Targets::AllAgents
        );
    }

    #[test]
    fn a_workspace_gains_the_ticked_directories_and_all_agents_wins() {
        let dirs = |items: &[&str]| WorkspaceTargets::Dirs(set(items));
        let all = WorkspaceTargets::AllAgents;
        assert_eq!(workspace_merged(None, dirs(&["a"])), dirs(&["a"]));
        assert_eq!(workspace_merged(None, all.clone()), all);
        assert_eq!(
            workspace_merged(Some(&dirs(&["a"])), dirs(&["b"])),
            dirs(&["a", "b"])
        );
        assert_eq!(workspace_merged(Some(&dirs(&["a"])), all.clone()), all);
        assert_eq!(workspace_merged(Some(&all), dirs(&["b"])), all);
    }

    #[test]
    fn a_batch_needs_skills_each_with_an_agent() {
        let (pdf, review) = (SkillId("pdf".into()), SkillId("review".into()));
        let message = |result: AppResult<()>| result.err().map(|error| error.message);
        assert_eq!(
            message(refuse([])),
            Some("Select skills to add.".to_owned())
        );
        assert_eq!(
            message(refuse([(&pdf, false), (&review, true)])),
            Some("Pick an agent for review.".to_owned())
        );
        assert_eq!(message(refuse([(&pdf, false)])), None);
    }
}
