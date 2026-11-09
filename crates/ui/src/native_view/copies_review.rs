//! The review list that Review opens from the Done pill (spec/progress-3-toast-review.md): every
//! copy that needed the user when it opened, in a card, each with the two answers its state offers.
//! An answered row ends in a muted check and what was done.

mod render;
use skillbinder_proto::{CopyState, ManagedCopy, SkillId};
use std::collections::BTreeMap;

/// A copy the review lists, by directory and skill.
type Item = (String, SkillId);

/// The review's copies, each with why it needed the user when the review opened, which its row
/// keeps once answered, and what the user answered for each.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Review {
    items: Vec<(Item, &'static str)>,
    answers: BTreeMap<Item, Answer>,
}

impl Review {
    /// The copies among `copies` that need the user now.
    pub(super) fn of(copies: &[ManagedCopy]) -> Self {
        Self {
            items: copies
                .iter()
                .filter(|copy| copy.state.needs_attention())
                .filter_map(|copy| {
                    let choice = Choice::for_state(&copy.state)?;
                    Some(((copy.directory.clone(), copy.skill.clone()), choice.reason))
                })
                .collect(),
            answers: BTreeMap::new(),
        }
    }

    /// Whether it lists no copy.
    pub(super) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// How `item`'s row stands, given the library's `copies` now.
    fn row(&self, item: &Item, copies: &[ManagedCopy]) -> Row {
        let answer = self.answers.get(item).copied();
        if answer == Some(Answer::Skip) {
            return Row::Settled(Answer::Skip.settled());
        }
        let current = copies
            .iter()
            .find(|copy| copy.directory == item.0 && copy.skill == item.1);
        let asking = current.and_then(|copy| Some((Choice::for_state(&copy.state)?, copy.clone())));
        match asking {
            Some((choice, copy)) => Row::Ask(choice, copy),
            None => Row::Settled(answer.map_or("Done", Answer::settled)),
        }
    }
}

/// An answer a review row offers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Answer {
    Replace,
    Remove,
    Keep,
    Retry,
    Skip,
}

impl Answer {
    fn label(self) -> &'static str {
        match self {
            Self::Replace => "Replace",
            Self::Remove => "Remove",
            Self::Keep => "Keep",
            Self::Retry => "Retry",
            Self::Skip => "Skip",
        }
    }

    /// What the row says once the answer took effect.
    fn settled(self) -> &'static str {
        match self {
            Self::Replace => "Replaced",
            Self::Remove => "Removed",
            Self::Keep => "Kept",
            Self::Retry => "Done",
            Self::Skip => "Skipped",
        }
    }
}

/// Why a copy needs the user, and its two answers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Choice {
    reason: &'static str,
    yes: Answer,
    no: Answer,
}

impl Choice {
    fn for_state(state: &CopyState) -> Option<Self> {
        let (reason, yes, no) = match state {
            CopyState::Conflict => ("Different copy there", Answer::Replace, Answer::Keep),
            CopyState::Edited { active: true } => {
                ("You changed it here", Answer::Replace, Answer::Keep)
            }
            CopyState::Edited { active: false } => {
                ("You changed it here", Answer::Remove, Answer::Keep)
            }
            CopyState::NotApplied(_) | CopyState::NotRemoved(_) => {
                ("Can't write the folder", Answer::Retry, Answer::Skip)
            }
            CopyState::Applied | CopyState::Skipped => return None,
        };
        Some(Self { reason, yes, no })
    }
}

/// How a review row stands.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Row {
    /// It still needs the user, who has this choice about the copy.
    Ask(Choice, ManagedCopy),
    /// It is answered or settled, as this word says.
    Settled(&'static str),
}

#[cfg(test)]
mod tests;
