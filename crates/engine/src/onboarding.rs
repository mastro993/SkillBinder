//! Onboarding's steps, derived from the facts the engine saves (ADR 0010). Adding a step means a
//! variant, a place in [`ORDER`], and an arm in [`applies`] and [`done`].

use skillbinder_proto::{Onboarding, OnboardingStep, StepStatus};

/// Every step, in the order onboarding shows them.
const ORDER: [OnboardingStep; 4] = [
    OnboardingStep::DataLocation,
    OnboardingStep::Git,
    OnboardingStep::Agents,
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
    pub finished: bool,
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
        OnboardingStep::Import => true,
    }
}

fn done(step: OnboardingStep, facts: Facts) -> bool {
    match step {
        OnboardingStep::DataLocation => facts.storage_writable,
        OnboardingStep::Git => facts.git_found || facts.git_skipped,
        OnboardingStep::Agents => facts.agents_chosen,
        OnboardingStep::Import => facts.finished,
    }
}

#[cfg(test)]
mod tests {
    use super::{Facts, derive};
    use skillbinder_proto::OnboardingStep::{Agents, DataLocation, Git, Import};

    /// Facts with writable storage and Git found, nothing settled yet.
    const FRESH: Facts = Facts {
        storage_writable: true,
        git_found: true,
        git_skipped: false,
        agents_chosen: false,
        finished: false,
    };

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
                    finished: true,
                    ..FRESH
                },
                &[(Agents, true), (Import, true)],
                None,
            ),
        ];
        for (facts, steps, current) in cases {
            let onboarding = derive(facts);
            let derived: Vec<_> = onboarding
                .steps
                .iter()
                .map(|status| (status.step, status.done))
                .collect();
            assert_eq!(derived, steps, "{facts:?}");
            assert_eq!(onboarding.current(), current, "{facts:?}");
            assert_eq!(onboarding.finished, facts.finished);
        }
    }
}
