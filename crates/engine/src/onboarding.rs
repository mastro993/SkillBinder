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
        // Not offered until choosing agents is built.
        OnboardingStep::Agents => false,
        OnboardingStep::Import => true,
    }
}

fn done(step: OnboardingStep, facts: Facts) -> bool {
    match step {
        OnboardingStep::DataLocation => facts.storage_writable,
        OnboardingStep::Git => facts.git_found,
        OnboardingStep::Agents => false,
        OnboardingStep::Import => facts.finished,
    }
}

#[cfg(test)]
mod tests {
    use super::{Facts, derive};
    use skillbinder_proto::OnboardingStep::{DataLocation, Git, Import};

    #[test]
    fn only_the_steps_whose_fact_is_missing_apply_and_the_first_not_done_is_current() {
        let cases = [
            (true, true, false, &[(Import, false)][..], Some(Import)),
            (
                false,
                true,
                false,
                &[(DataLocation, false), (Import, false)],
                Some(DataLocation),
            ),
            (
                true,
                false,
                false,
                &[(Git, false), (Import, false)],
                Some(Git),
            ),
            (
                false,
                false,
                false,
                &[(DataLocation, false), (Git, false), (Import, false)],
                Some(DataLocation),
            ),
            (true, true, true, &[(Import, true)], None),
        ];
        for (storage_writable, git_found, finished, steps, current) in cases {
            let onboarding = derive(Facts {
                storage_writable,
                git_found,
                finished,
            });
            let derived: Vec<_> = onboarding
                .steps
                .iter()
                .map(|status| (status.step, status.done))
                .collect();
            assert_eq!(
                derived, steps,
                "writable {storage_writable}, Git {git_found}"
            );
            assert_eq!(onboarding.current(), current);
            assert_eq!(onboarding.finished, finished);
        }
    }
}
