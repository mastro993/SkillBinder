//! The blocking progress strip (ADR 0007). A change that can write many copies blocks the app from
//! the click: a transparent layer takes every pointer event, and after 300 ms the window takes a
//! wash of the page color and a pill at its top shows how far the change has got. Once shown, the
//! strip stays at least 500 ms. It ends in a toast when nothing needs the user, or in `Done · N
//! need attention` with Review and Close.

mod flow;
mod render;
#[cfg(test)]
mod tests;

use skillbinder_proto::ActivationProgress;
use std::time::{Duration, Instant};

/// How long a change runs before the strip shows.
pub(super) const REVEAL_AFTER: Duration = Duration::from_millis(300);
/// How long a shown strip stays at least.
pub(super) const SHOWN_AT_LEAST: Duration = Duration::from_millis(500);

/// The change the app is blocked for, which names the strip's label and its end toast.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Activity {
    /// Activating the preset named `preset`.
    Switching { preset: String },
    /// Deleting the active preset named `preset`, which activates the Default preset.
    DeletingActive { preset: String },
    /// Applying an import of `skills` reviewed skills.
    Importing { skills: usize },
}

impl Activity {
    /// The running pill's label.
    pub(super) fn label(&self) -> &'static str {
        match self {
            Self::Switching { .. } | Self::DeletingActive { .. } => "Switching…",
            Self::Importing { .. } => "Importing…",
        }
    }

    /// The first line of the running pill's tooltip.
    pub(super) fn title(&self) -> String {
        match self {
            Self::Switching { preset } => format!("Switching to {preset}"),
            Self::DeletingActive { .. } => "Switching to Default".to_owned(),
            Self::Importing { skills: count } => skills("Importing", *count),
        }
    }

    /// The toast once the change lands with nothing for the user.
    pub(super) fn toast(&self, landing: &Landing) -> String {
        match self {
            Self::Switching { preset } => format!("Switched to {preset}"),
            Self::DeletingActive { preset } => format!("Deleted {preset} · Switched to Default"),
            Self::Importing { .. } => skills("Imported", landing.imported),
        }
    }
}

/// `<verb> 1 skill` or `<verb> <n> skills`.
fn skills(verb: &str, count: usize) -> String {
    match count {
        1 => format!("{verb} 1 skill"),
        count => format!("{verb} {count} skills"),
    }
}

/// The Done pill's label.
pub(super) fn done_label(attention: usize) -> String {
    match attention {
        1 => "Done · 1 needs attention".to_owned(),
        count => format!("Done · {count} need attention"),
    }
}

/// What a blocking change reports when it lands.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Landing {
    /// The final counts of its passes.
    pub progress: ActivationProgress,
    /// The skills an import brought into the library.
    pub imported: usize,
}

impl From<ActivationProgress> for Landing {
    fn from(progress: ActivationProgress) -> Self {
        Self {
            progress,
            imported: 0,
        }
    }
}

/// How far the strip has got.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Stage {
    Working,
    /// The change landed and the strip holds at 100% until it has shown long enough.
    Landed(Landing),
    /// The change was refused; the strip holds until it has shown long enough.
    Failed,
    /// The change landed with copies that need the user.
    Done(usize),
}

/// What the strip draws.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Shown {
    /// Only the transparent layer that blocks input.
    Blocker,
    /// The wash and the running pill at this percent.
    Working(u8),
    /// The wash and the Done pill for this many copies.
    Done(usize),
}

/// How the strip ends once its time is up.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Finish {
    /// It closes and this toast confirms the change.
    Toast(String),
    /// It closes without a toast: the change was refused.
    Closed,
    /// It stays, as the Done pill.
    Done,
}

/// The strip while a blocking change runs and until the user closes its Done pill.
pub(super) struct Strip {
    pub(super) activity: Activity,
    /// When the change started; timers set for another strip check it.
    started: Instant,
    revealed: Option<Instant>,
    /// The highest percent shown so far.
    percent: u8,
    /// The latest step the change reported, for the tooltip.
    step: Option<skillbinder_proto::ActivationStep>,
    stage: Stage,
    /// When the strip may end, once the change has landed or been refused.
    ends: Option<Instant>,
}

impl Strip {
    pub(super) fn new(activity: Activity, now: Instant) -> Self {
        Self {
            activity,
            started: now,
            revealed: None,
            percent: 0,
            step: None,
            stage: Stage::Working,
            ends: None,
        }
    }

    pub(super) fn started(&self) -> Instant {
        self.started
    }

    /// Shows the wash and the pill, unless they show already. A change that lands first ends the
    /// strip at once, so the reveal never comes.
    pub(super) fn reveal(&mut self, now: Instant) {
        self.revealed.get_or_insert(now);
    }

    /// Follows the published progress. The percent never goes down, and a refresh that publishes
    /// no progress while the change runs keeps the last one.
    pub(super) fn follow(&mut self, reported: Option<&ActivationProgress>) {
        if self.stage != Stage::Working {
            return;
        }
        if let Some(reported) = reported {
            self.percent = self.percent.max(reported.percent());
            if reported.step.is_some() {
                self.step.clone_from(&reported.step);
            }
        }
    }

    /// The change landed at `now`: returns when the strip ends.
    pub(super) fn land(&mut self, landing: Landing, now: Instant) -> Instant {
        self.stage = Stage::Landed(landing);
        self.percent = 100;
        *self.ends.insert(ends_at(self.revealed, now))
    }

    /// The change was refused at `now`: returns when the strip ends.
    pub(super) fn fail(&mut self, now: Instant) -> Instant {
        self.stage = Stage::Failed;
        *self.ends.insert(ends_at(self.revealed, now))
    }

    /// When the strip may end, once the change has landed or been refused.
    #[cfg(any(test, feature = "native-test"))]
    pub(super) fn ends(&self) -> Option<Instant> {
        self.ends
    }

    /// Whether 300 ms have passed since the change started.
    #[cfg(feature = "native-test")]
    pub(super) fn due(&self, now: Instant) -> bool {
        now.duration_since(self.started) >= REVEAL_AFTER
    }

    /// Ends the strip at `now`, its time being up: copies that need the user turn it into the
    /// Done pill, which shows at once if it had not.
    pub(super) fn finish(&mut self, now: Instant) -> Finish {
        match &self.stage {
            Stage::Landed(landing) if landing.progress.attention > 0 => {
                self.stage = Stage::Done(landing.progress.attention);
                self.revealed.get_or_insert(now);
                Finish::Done
            }
            Stage::Landed(landing) => Finish::Toast(self.activity.toast(landing)),
            Stage::Failed => Finish::Closed,
            Stage::Working | Stage::Done(_) => Finish::Done,
        }
    }

    /// What the strip draws now.
    pub(super) fn shown(&self) -> Shown {
        match (&self.stage, self.revealed) {
            (Stage::Done(attention), _) => Shown::Done(*attention),
            (_, None) => Shown::Blocker,
            (_, Some(_)) => Shown::Working(self.percent),
        }
    }

    /// The running pill's tooltip: what the change does, then the step it is on.
    pub(super) fn tooltip(&self, place: impl Fn(&str) -> String) -> String {
        let title = self.activity.title();
        match &self.step {
            None => title,
            Some(step) if step.removing => format!(
                "{title}\nRemoving {} from {}",
                step.skill,
                place(&step.directory)
            ),
            Some(step) => format!(
                "{title}\nCopying {} → {}",
                step.skill,
                place(&step.directory)
            ),
        }
    }
}

/// When a strip revealed at `revealed`, if at all, may end for a change that landed at `landed`:
/// at once if it never showed, otherwise once it has shown for [`SHOWN_AT_LEAST`].
pub(super) fn ends_at(revealed: Option<Instant>, landed: Instant) -> Instant {
    revealed.map_or(landed, |revealed| landed.max(revealed + SHOWN_AT_LEAST))
}
