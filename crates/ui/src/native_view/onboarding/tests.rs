use super::{Place, place};
use skillbinder_proto::OnboardingStep::{self, Agents, DataLocation, Import};
use skillbinder_proto::{Onboarding, StepStatus};

fn onboarding(steps: &[(OnboardingStep, bool)]) -> Onboarding {
    Onboarding {
        steps: steps
            .iter()
            .map(|&(step, done)| StepStatus { step, done })
            .collect(),
        finished: false,
    }
}

#[test]
fn the_frame_counts_only_the_steps_that_apply_and_moves_back_and_forward_through_done_steps() {
    let cases = [
        // Only the import applies: one step, no Back.
        (
            &[(Import, false)][..],
            None,
            Some((Import, 1, 1, None, None)),
        ),
        (
            &[(DataLocation, false), (Import, false)],
            None,
            Some((DataLocation, 1, 2, None, None)),
        ),
        // A done step before the current one is where Back returns, and from it Continue leads on.
        (
            &[(Agents, true), (Import, false)],
            None,
            Some((Import, 2, 2, Some(Agents), None)),
        ),
        (
            &[(Agents, true), (Import, false)],
            Some(Agents),
            Some((Agents, 1, 2, None, Some(Import))),
        ),
        // A step gone back to that no longer applies, or lies ahead, is not shown.
        (
            &[(DataLocation, false), (Import, false)],
            Some(Import),
            Some((DataLocation, 1, 2, None, None)),
        ),
        (
            &[(Import, false)],
            Some(Agents),
            Some((Import, 1, 1, None, None)),
        ),
        // Every step done: onboarding is over.
        (&[(Import, true)], None, None),
    ];
    for (steps, back_to, expected) in cases {
        let place = place(&onboarding(steps), back_to);
        let expected = expected.map(|(step, number, count, back, forward)| Place {
            step,
            number,
            count,
            back,
            forward,
        });
        assert_eq!(place, expected, "{steps:?} back to {back_to:?}");
    }
}

#[test]
fn the_eyebrow_names_the_app_and_the_step_among_those_that_apply() {
    let place = Place {
        step: Import,
        number: 2,
        count: 3,
        back: Some(Agents),
        forward: None,
    };
    assert_eq!(place.eyebrow(), "SkillBinder · Step 2 of 3");
}
