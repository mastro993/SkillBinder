//! Your agents (ADR 0012): the detected agents whose global skill directory the user chose.
//! SkillBinder scans, writes, and adopts copies only in theirs. The choice is made per global
//! skill directory, so agents that share one, such as the universal agents, are one choice.

use crate::{
    persistence,
    scan::registry::{self, Reach},
    storage,
    worker::State,
};
use serde::{Deserialize, Serialize};
use skillbinder_proto::{AgentOption, AppError, AppResult, Bootstrap, OnboardingStep};
use std::{collections::BTreeSet, path::PathBuf};

/// Device setting holding the [`AgentChoices`], saved once the agents are chosen.
const AGENTS: &str = "agents";

/// The global skill directories the user chose, and those they were asked about whatever the
/// answer, as [`AgentOption::global_skills_dir`] names them: as shown, so a choice holds when a
/// link changes where the directory leads. Every chosen directory was asked about.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(crate) struct AgentChoices {
    pub chosen: BTreeSet<String>,
    pub asked: BTreeSet<String>,
}

/// Your agents among `agents`: the one filter that decides which agents SkillBinder scans and
/// writes for.
pub(crate) fn chosen_agents(agents: &[AgentOption]) -> impl Iterator<Item = &AgentOption> {
    agents.iter().filter(|agent| agent.chosen)
}

impl State {
    /// The saved choices, or `None` before the agents were chosen.
    pub(crate) fn agent_choices(&self) -> AppResult<Option<AgentChoices>> {
        self.setting(AGENTS)
    }

    /// Every registry agent, by display name, with whether it is one of your agents.
    pub(crate) fn agents(&self) -> AppResult<Vec<AgentOption>> {
        registry::agent_options(&self.config, &self.agent_choices()?.unwrap_or_default())
    }

    /// What a scan reads: your agents' global skill directories and the present workspaces.
    pub(crate) fn reach(&self) -> AppResult<Reach> {
        Ok(Reach {
            agents: chosen_agents(&self.agents()?)
                .map(|agent| agent.id.clone())
                .collect(),
            workspaces: self
                .workspaces()?
                .into_iter()
                .filter(|workspace| !workspace.missing)
                .map(|workspace| (workspace.id, PathBuf::from(workspace.path)))
                .collect(),
        })
    }

    /// Saves onboarding's choice of agents: of the global skill directories `shown`, those in
    /// `chosen` become yours and the rest not, and every one shown counts as asked about. Then
    /// creates the library on the way into the import, a repository when Git is found. Refused before the earlier steps are done
    /// and once onboarding finished.
    pub(crate) fn choose_agents(
        &self,
        shown: BTreeSet<String>,
        chosen: BTreeSet<String>,
    ) -> AppResult<Bootstrap> {
        let onboarding = self.bootstrap()?.onboarding;
        if onboarding.finished {
            return Err(AppError::validation("Your agents are already chosen."));
        }
        let earlier_done = onboarding
            .steps
            .iter()
            .take_while(|status| status.step != OnboardingStep::Agents)
            .all(|status| status.done);
        if !earlier_done {
            return Err(AppError::validation(
                "Finish the earlier setup steps first.",
            ));
        }
        if !chosen.is_subset(&shown) {
            return Err(AppError::validation("Choose among the agents shown."));
        }
        let mut choices = self.agent_choices()?.unwrap_or_default();
        choices.chosen.retain(|dir| !shown.contains(dir));
        choices.chosen.extend(chosen);
        choices.asked.extend(shown);
        self.set_setting(AGENTS, &choices)?;
        self.ensure_library()?;
        self.ensure_repository()?;
        self.bootstrap()
    }

    /// Turns the agents reading the global skill directories `dirs` on or off after onboarding,
    /// from Settings or the new agent notice, each counting as asked about (ADR 0012). On applies
    /// the active preset to them, adopting copies with the library's content and asking about the
    /// others; off leaves their copies in place, unmanaged. Off for agents that are off already
    /// only records the answer, as Not now does.
    pub(crate) fn set_agents_chosen(&self, dirs: BTreeSet<String>, chosen: bool) -> AppResult<()> {
        self.require_recovered()?;
        if !self.onboarding_finished()? {
            return Err(AppError::validation(
                "Choose your agents during setup first.",
            ));
        }
        let found: BTreeSet<String> = self
            .agents()?
            .into_iter()
            .filter(|agent| agent.detected)
            .filter_map(|agent| agent.global_skills_dir)
            .collect();
        if dirs.is_empty() || !dirs.is_subset(&found) {
            return Err(AppError::validation(
                "Choose among the agents found on this computer.",
            ));
        }
        let mut choices = self.agent_choices()?.unwrap_or_default();
        choices.asked.extend(dirs.iter().cloned());
        if chosen {
            choices.chosen.extend(dirs);
            return self.change_places(|state| state.set_setting(AGENTS, &choices));
        }
        choices.chosen.retain(|dir| !dirs.contains(dir));
        let after = registry::agent_options(&self.config, &choices)?;
        let value =
            serde_json::to_string(&choices).map_err(storage::error("serializing agent choices"))?;
        self.release_places(&after, |connection| {
            persistence::put_setting(connection, AGENTS, &value)
        })
    }

    /// Makes every detected agent yours, and asked about, when onboarding finished before agents
    /// were chosen, such as with data saved at the old `Complete` position.
    pub(crate) fn choose_agents_of_finished_onboarding(&self) -> AppResult<()> {
        if !self.onboarding_finished()? || self.agent_choices()?.is_some() {
            return Ok(());
        }
        let detected: BTreeSet<String> =
            registry::agent_options(&self.config, &AgentChoices::default())?
                .into_iter()
                .filter(|agent| agent.detected)
                .filter_map(|agent| agent.global_skills_dir)
                .collect();
        self.set_setting(
            AGENTS,
            &AgentChoices {
                chosen: detected.clone(),
                asked: detected,
            },
        )
    }
}
