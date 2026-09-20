use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OnboardingStep {
    Prerequisites,
    Boundaries,
    SyncChoice,
    Ready,
}

impl OnboardingStep {
    pub const fn next(self) -> Self {
        match self {
            Self::Prerequisites => Self::Boundaries,
            Self::Boundaries => Self::SyncChoice,
            Self::SyncChoice | Self::Ready => Self::Ready,
        }
    }

    const fn index(self) -> u8 {
        match self {
            Self::Prerequisites => 0,
            Self::Boundaries => 1,
            Self::SyncChoice => 2,
            Self::Ready => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
pub enum OnboardingTransitionError {
    #[error("cannot skip onboarding steps from {current:?} to {requested:?}")]
    SkippedStep {
        current: OnboardingStep,
        requested: OnboardingStep,
    },
    #[error("onboarding cannot finish from {0:?}")]
    NotReady(OnboardingStep),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingProgress {
    pub step: OnboardingStep,
    pub completed: bool,
}

impl Default for OnboardingProgress {
    fn default() -> Self {
        Self {
            step: OnboardingStep::Prerequisites,
            completed: false,
        }
    }
}

impl OnboardingProgress {
    pub fn resume_at(&mut self, step: OnboardingStep) -> Result<(), OnboardingTransitionError> {
        if self.completed {
            return Ok(());
        }
        if step.index() > self.step.index() + 1 {
            return Err(OnboardingTransitionError::SkippedStep {
                current: self.step,
                requested: step,
            });
        }
        self.step = step;
        Ok(())
    }

    pub fn complete(&mut self) -> Result<(), OnboardingTransitionError> {
        if self.completed {
            return Ok(());
        }
        if self.step != OnboardingStep::Ready {
            return Err(OnboardingTransitionError::NotReady(self.step));
        }
        self.step = OnboardingStep::Ready;
        self.completed = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_stop_at_ready() {
        assert_eq!(
            OnboardingStep::Prerequisites.next(),
            OnboardingStep::Boundaries
        );
        assert_eq!(
            OnboardingStep::Boundaries.next(),
            OnboardingStep::SyncChoice
        );
        assert_eq!(OnboardingStep::SyncChoice.next(), OnboardingStep::Ready);
        assert_eq!(OnboardingStep::Ready.next(), OnboardingStep::Ready);
    }

    #[test]
    fn completed_progress_cannot_be_reopened() {
        let mut progress = OnboardingProgress::default();
        progress.resume_at(OnboardingStep::Boundaries).unwrap();
        progress.resume_at(OnboardingStep::SyncChoice).unwrap();
        progress.resume_at(OnboardingStep::Ready).unwrap();
        progress.complete().unwrap();
        progress.resume_at(OnboardingStep::Prerequisites).unwrap();
        assert_eq!(progress.step, OnboardingStep::Ready);
        assert!(progress.completed);
    }

    #[test]
    fn progress_cannot_skip_required_steps() {
        let mut progress = OnboardingProgress::default();

        assert_eq!(
            progress.resume_at(OnboardingStep::Ready),
            Err(OnboardingTransitionError::SkippedStep {
                current: OnboardingStep::Prerequisites,
                requested: OnboardingStep::Ready,
            })
        );
        assert_eq!(progress, OnboardingProgress::default());
    }

    #[test]
    fn progress_cannot_complete_before_ready() {
        let mut progress = OnboardingProgress::default();

        assert_eq!(
            progress.complete(),
            Err(OnboardingTransitionError::NotReady(
                OnboardingStep::Prerequisites
            ))
        );
        assert_eq!(progress, OnboardingProgress::default());
    }
}
