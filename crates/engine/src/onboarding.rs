//! Onboarding's steps, derived from the facts the engine saves (ADR 0010). Adding a step means a
//! variant, a place in [`ORDER`], and an arm in [`applies`] and [`done`].

use skillbinder_proto::{CommandLineState, Onboarding, OnboardingStep, StepStatus};

/// Every step, in the order onboarding shows them.
const ORDER: [OnboardingStep; 6] = [
    OnboardingStep::DataLocation,
    OnboardingStep::Git,
    OnboardingStep::Agents,
    OnboardingStep::CommandLine,
    OnboardingStep::ConnectAgents,
    OnboardingStep::Import,
];

/// The saved facts the steps settle.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Facts {
    pub storage_writable: bool,
    pub git_found: bool,
    pub git_skipped: bool,
    /// The agents were chosen, whichever ones.
    pub agents_chosen: bool,
    /// What the command and agents steps read; `None` once onboarding finished, since users who
    /// finished it before those steps existed never see them (ADR 0027).
    pub offers: Option<Offers>,
    pub finished: bool,
}

/// The facts behind the command and agents steps (ADR 0027).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Offers {
    /// The `skillbinder` command's state (ADR 0024).
    pub command_line: CommandLineState,
    /// An agent Connect can connect is on this computer, and its agent is chosen, or found while
    /// the choice is not saved yet.
    pub connectable: bool,
    /// The command step was skipped, or its Install succeeded.
    pub command_line_asked: bool,
    /// The agents step was skipped, continued, or set up.
    pub mcp_asked: bool,
}

impl Offers {
    /// The command step offers Install: nothing is there, or the command leads to another
    /// SkillBinder. Once settled it keeps applying, so the step count stays the same.
    fn command_line(self) -> bool {
        self.command_line_asked
            || matches!(
                self.command_line,
                CommandLineState::NotInstalled | CommandLineState::OtherApp
            )
    }

    /// The agents step has an agent to connect, and Connect has a command to point it at.
    fn connect_agents(self) -> bool {
        let command = match self.command_line {
            CommandLineState::InTheWay | CommandLineState::CannotInstall(_) => false,
            CommandLineState::NotInstalled
            | CommandLineState::Installed
            | CommandLineState::Homebrew
            | CommandLineState::OtherApp
            | CommandLineState::NotOnPath => true,
        };
        self.mcp_asked || (self.connectable && command)
    }
}

/// The steps that apply on this device, each with whether it is done.
pub(crate) fn derive(facts: Facts) -> Onboarding {
    Onboarding {
        steps: ORDER
            .into_iter()
            .filter(|&step| applies(step, facts))
            .map(|step| StepStatus {
                step,
                done: done(step, facts),
            })
            .collect(),
        finished: done(OnboardingStep::Import, facts),
    }
}

fn applies(step: OnboardingStep, facts: Facts) -> bool {
    match step {
        OnboardingStep::DataLocation => !facts.storage_writable,
        OnboardingStep::Git => !facts.git_found,
        // With no agent found, it says SkillBinder will ask when it finds one.
        OnboardingStep::Agents => true,
        OnboardingStep::CommandLine => facts.offers.is_some_and(Offers::command_line),
        OnboardingStep::ConnectAgents => facts.offers.is_some_and(Offers::connect_agents),
        OnboardingStep::Import => true,
    }
}

fn done(step: OnboardingStep, facts: Facts) -> bool {
    match step {
        OnboardingStep::DataLocation => facts.storage_writable,
        OnboardingStep::Git => facts.git_found || facts.git_skipped,
        OnboardingStep::Agents => facts.agents_chosen,
        // Installed and not asked, the step no longer applies.
        OnboardingStep::CommandLine => facts.offers.is_some_and(|offers| offers.command_line_asked),
        OnboardingStep::ConnectAgents => facts.offers.is_some_and(|offers| offers.mcp_asked),
        OnboardingStep::Import => facts.finished,
    }
}

#[cfg(test)]
mod tests {
    use super::{Facts, Offers, derive};
    use skillbinder_proto::OnboardingStep::{
        self, Agents, CommandLine, ConnectAgents, DataLocation, Git, Import,
    };
    use skillbinder_proto::{CannotInstallCommand, CommandLineState};

    /// A copy that can't install the command, with no agent to connect.
    const NOTHING_OFFERED: Offers = Offers {
        command_line: CommandLineState::CannotInstall(CannotInstallCommand::OutsideApplications),
        connectable: false,
        command_line_asked: false,
        mcp_asked: false,
    };

    /// Facts with writable storage and Git found, nothing settled yet.
    const FRESH: Facts = Facts {
        storage_writable: true,
        git_found: true,
        git_skipped: false,
        agents_chosen: false,
        offers: Some(NOTHING_OFFERED),
        finished: false,
    };

    /// The command not installed and an agent to connect, past Choose your agents.
    const OFFERED: Facts = Facts {
        agents_chosen: true,
        offers: Some(Offers {
            command_line: CommandLineState::NotInstalled,
            connectable: true,
            ..NOTHING_OFFERED
        }),
        ..FRESH
    };

    fn derived(facts: Facts) -> (Vec<(OnboardingStep, bool)>, Option<OnboardingStep>) {
        let onboarding = derive(facts);
        let steps = onboarding
            .steps
            .iter()
            .map(|status| (status.step, status.done))
            .collect();
        (steps, onboarding.current())
    }

    /// `OFFERED` with these offers changed.
    fn offered(change: impl FnOnce(&mut Offers)) -> Facts {
        let mut facts = OFFERED;
        if let Some(offers) = &mut facts.offers {
            change(offers);
        }
        facts
    }

    #[test]
    fn only_the_steps_whose_fact_is_missing_apply_and_the_first_not_done_is_current() {
        let cases = [
            (FRESH, &[(Agents, false), (Import, false)][..], Some(Agents)),
            (
                Facts {
                    storage_writable: false,
                    ..FRESH
                },
                &[(DataLocation, false), (Agents, false), (Import, false)],
                Some(DataLocation),
            ),
            (
                Facts {
                    git_found: false,
                    ..FRESH
                },
                &[(Git, false), (Agents, false), (Import, false)],
                Some(Git),
            ),
            (
                Facts {
                    git_found: false,
                    git_skipped: true,
                    ..FRESH
                },
                &[(Git, true), (Agents, false), (Import, false)],
                Some(Agents),
            ),
            (
                Facts {
                    agents_chosen: true,
                    ..FRESH
                },
                &[(Agents, true), (Import, false)],
                Some(Import),
            ),
            (
                Facts {
                    storage_writable: false,
                    git_found: false,
                    ..FRESH
                },
                &[
                    (DataLocation, false),
                    (Git, false),
                    (Agents, false),
                    (Import, false),
                ],
                Some(DataLocation),
            ),
            (
                Facts {
                    agents_chosen: true,
                    offers: None,
                    finished: true,
                    ..FRESH
                },
                &[(Agents, true), (Import, true)],
                None,
            ),
        ];
        for (facts, steps, current) in cases {
            assert_eq!(derived(facts), (steps.to_vec(), current), "{facts:?}");
            assert_eq!(derive(facts).finished, facts.finished);
        }
    }

    #[test]
    fn the_command_step_offers_install_until_it_is_installed_or_settled() {
        use CommandLineState::{Homebrew, InTheWay, Installed, NotInstalled, NotOnPath, OtherApp};
        let cannot = CommandLineState::CannotInstall(CannotInstallCommand::NotAppImage);
        for (state, applies) in [
            (NotInstalled, true),
            (OtherApp, true),
            (Installed, false),
            (Homebrew, false),
            (NotOnPath, false),
            (InTheWay, false),
            (cannot, false),
        ] {
            let facts = offered(|offers| {
                offers.command_line = state;
                offers.connectable = false;
            });
            let expected = if applies {
                vec![(Agents, true), (CommandLine, false), (Import, false)]
            } else {
                vec![(Agents, true), (Import, false)]
            };
            assert_eq!(derived(facts).0, expected, "{state:?}");
        }
        // Settled, it stays in the count whatever the command's state is now.
        let settled = offered(|offers| {
            offers.command_line = Installed;
            offers.connectable = false;
            offers.command_line_asked = true;
        });
        assert_eq!(
            derived(settled),
            (
                vec![(Agents, true), (CommandLine, true), (Import, false)],
                Some(Import)
            )
        );
    }

    #[test]
    fn the_agents_step_needs_an_agent_to_connect_and_a_command_to_point_it_at() {
        use CommandLineState::{Homebrew, InTheWay, Installed, NotOnPath, OtherApp};
        let cannot = CommandLineState::CannotInstall(CannotInstallCommand::OutsideApplications);
        for (state, applies) in [
            (Installed, true),
            (Homebrew, true),
            (NotOnPath, true),
            (OtherApp, true),
            (InTheWay, false),
            (cannot, false),
        ] {
            let facts = offered(|offers| offers.command_line = state);
            let applied = derived(facts).0.contains(&(ConnectAgents, false));
            assert_eq!(applied, applies, "{state:?}");
        }
        let unconnectable = offered(|offers| offers.connectable = false);
        assert!(!derived(unconnectable).0.contains(&(ConnectAgents, false)));
    }

    #[test]
    fn both_steps_sit_between_the_agents_and_the_import_and_homebrew_drops_the_command_step() {
        assert_eq!(
            derived(Facts {
                git_found: false,
                ..OFFERED
            }),
            (
                vec![
                    (Git, false),
                    (Agents, true),
                    (CommandLine, false),
                    (ConnectAgents, false),
                    (Import, false),
                ],
                Some(Git),
            )
        );
        let homebrew = offered(|offers| offers.command_line = CommandLineState::Homebrew);
        assert_eq!(
            derived(homebrew).0,
            [(Agents, true), (ConnectAgents, false), (Import, false)]
        );
    }

    #[test]
    fn settling_each_step_moves_on_and_both_settled_resume_at_the_import() {
        let skipped_command = offered(|offers| offers.command_line_asked = true);
        assert_eq!(derived(skipped_command).1, Some(ConnectAgents));
        let both = offered(|offers| {
            offers.command_line_asked = true;
            offers.mcp_asked = true;
        });
        assert_eq!(
            derived(both),
            (
                vec![
                    (Agents, true),
                    (CommandLine, true),
                    (ConnectAgents, true),
                    (Import, false),
                ],
                Some(Import),
            )
        );
    }

    #[test]
    fn a_finished_onboarding_stays_finished_with_neither_fact_saved() {
        let finished = Facts {
            offers: None,
            finished: true,
            ..OFFERED
        };
        let onboarding = derive(finished);
        assert!(onboarding.finished);
        assert_eq!(onboarding.current(), None);
        assert_eq!(derived(finished).0, [(Agents, true), (Import, true)]);
    }
}
