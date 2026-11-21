//! The menu the icon opens: a typed model the backends only draw (ADR 0021), derived from the
//! snapshot and the [`Gate`] alone.

use crate::gate::Gate;
use crate::native_view::{available, resolved_active, summary, waiting_copies};
use skillbinder_proto::{AppSnapshot, AppUpdateState, PresetId};

/// What a menu item does.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TrayAction {
    /// Opens the window on the review of the copies that wait for the user.
    ReviewCopies,
    /// Runs Library's Update all.
    UpdateAll,
    /// Opens the window on Library with the Updates filter on.
    LibraryUpdates,
    /// Switches to this preset.
    ActivatePreset(PresetId),
    /// Opens the window, or brings the open one forward.
    OpenWindow,
    /// Quits as ⌘Q or Ctrl+Q does.
    Quit,
}

const PRESET: &str = "preset:";

impl TrayAction {
    /// The item's stable id.
    #[cfg_attr(target_os = "linux", allow(dead_code))]
    pub(crate) fn id(&self) -> String {
        match self {
            Self::ReviewCopies => "review-copies".into(),
            Self::UpdateAll => "update-all".into(),
            Self::LibraryUpdates => "library-updates".into(),
            Self::ActivatePreset(PresetId(id)) => format!("{PRESET}{id}"),
            Self::OpenWindow => "open-window".into(),
            Self::Quit => "quit".into(),
        }
    }

    /// The action whose item has `id`.
    #[cfg_attr(target_os = "linux", allow(dead_code))]
    pub(crate) fn from_id(id: &str) -> Option<Self> {
        match id {
            "review-copies" => Some(Self::ReviewCopies),
            "update-all" => Some(Self::UpdateAll),
            "library-updates" => Some(Self::LibraryUpdates),
            "open-window" => Some(Self::OpenWindow),
            "quit" => Some(Self::Quit),
            id => id
                .strip_prefix(PRESET)
                .map(|preset| Self::ActivatePreset(PresetId(preset.to_owned()))),
        }
    }
}

/// One row of the menu.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TrayItem {
    Action {
        action: TrayAction,
        label: String,
        enabled: bool,
        /// Whether it shows a check, for a row that can show one.
        checked: Option<bool>,
        /// The key that runs it with ⌘ on macOS, or with Ctrl elsewhere.
        shortcut: Option<char>,
    },
    /// Disabled text, such as a header.
    Label {
        id: &'static str,
        label: String,
    },
    Separator,
}

impl TrayItem {
    /// The row's stable id; a backend redraws in place while the ids stay the same.
    #[cfg_attr(target_os = "linux", allow(dead_code))]
    pub(crate) fn id(&self) -> String {
        match self {
            Self::Action { action, .. } => action.id(),
            Self::Label { id, .. } => (*id).into(),
            Self::Separator => "separator".into(),
        }
    }

    fn action(action: TrayAction, label: impl Into<String>) -> Self {
        Self::Action {
            action,
            label: label.into(),
            enabled: true,
            checked: None,
            shortcut: None,
        }
    }
}

/// The menu, top to bottom, and whether the icon shows its dot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrayMenu {
    pub(crate) items: Vec<TrayItem>,
    pub(crate) dot: bool,
}

impl TrayMenu {
    /// The menu for `snapshot` while `gate` holds (spec rules 7–10 and 12): the attention item,
    /// the skill updates, the presets, then Open and Quit. Before onboarding finishes, only Open
    /// and Quit.
    pub(crate) fn new(snapshot: &AppSnapshot, gate: &Gate) -> Self {
        let mut items = Vec::new();
        if snapshot.bootstrap.onboarding.finished {
            match waiting_copies(snapshot) {
                0 => {}
                waiting => items.extend([
                    TrayItem::action(TrayAction::ReviewCopies, attention(waiting)),
                    TrayItem::Separator,
                ]),
            }
            items.extend(skill_updates(snapshot, gate));
            items.push(TrayItem::Separator);
            if let Some(library) = &snapshot.library {
                items.push(TrayItem::Label {
                    id: "presets",
                    label: presets_header(snapshot, gate),
                });
                let presets = &library.library.presets;
                let active = resolved_active(presets, &library.library.active_preset)
                    .map(|preset| &preset.id);
                items.extend(presets.iter().map(|preset| TrayItem::Action {
                    action: TrayAction::ActivatePreset(preset.id.clone()),
                    label: preset.name.clone(),
                    enabled: !gate.blocked(snapshot),
                    checked: Some(active == Some(&preset.id)),
                    shortcut: None,
                }));
                items.push(TrayItem::Separator);
            }
        }
        items.extend([
            TrayItem::action(TrayAction::OpenWindow, "Open SkillBinder"),
            TrayItem::Separator,
            TrayItem::Action {
                action: TrayAction::Quit,
                label: "Quit SkillBinder".into(),
                enabled: true,
                checked: None,
                shortcut: Some('Q'),
            },
        ]);
        Self {
            items,
            dot: dot(snapshot),
        }
    }
}

/// Whether something waits for the user (D6): skills with an update, copies to review, or an
/// app update that is ready or downloads from its release page.
fn dot(snapshot: &AppSnapshot) -> bool {
    !available(snapshot).is_empty()
        || waiting_copies(snapshot) > 0
        || matches!(
            snapshot.app_update.state,
            AppUpdateState::Ready { .. } | AppUpdateState::DownloadOnly { .. }
        )
}

/// The skill update group (spec rules 9 and 10): the first status that applies, and Update all
/// while skills have an update.
fn skill_updates(snapshot: &AppSnapshot, gate: &Gate) -> Vec<TrayItem> {
    let status = |label: String| TrayItem::Label {
        id: "skill-updates",
        label,
    };
    if snapshot.bootstrap.git_version.is_none() {
        return vec![status("Skill updates need Git".into())];
    }
    if let Some((done, total)) = snapshot.updating {
        return vec![status(format!("Updating {done} of {total} skills…"))];
    }
    if let Some(outcome) = &snapshot.update_all_outcome
        && !outcome.failed.is_empty()
    {
        return vec![TrayItem::action(
            TrayAction::LibraryUpdates,
            summary(outcome),
        )];
    }
    match available(snapshot).len() {
        0 if snapshot.checking => vec![status("Checking skill updates…".into())],
        0 => vec![status("Skills are up to date".into())],
        count => vec![
            status(match count {
                1 => "1 skill has an update".into(),
                count => format!("{count} skills have updates"),
            }),
            TrayItem::Action {
                action: TrayAction::UpdateAll,
                label: "Update all".into(),
                enabled: !gate.refuses_update_all(snapshot),
                checked: None,
                shortcut: None,
            },
        ],
    }
}

/// `⚠ <n> skill copies need attention…`, or `⚠ 1 skill copy needs attention…`.
fn attention(waiting: usize) -> String {
    match waiting {
        1 => "⚠ 1 skill copy needs attention…".into(),
        waiting => format!("⚠ {waiting} skill copies need attention…"),
    }
}

/// `Presets`, or while a blocking change runs, the strip's title and its percent, such as
/// `Switching to Review… 40%`.
fn presets_header(snapshot: &AppSnapshot, gate: &Gate) -> String {
    let Some(activity) = gate.running() else {
        return "Presets".into();
    };
    let title = activity.title();
    match &snapshot.activation {
        Some(progress) => format!("{title}… {}%", progress.percent()),
        None => format!("{title}…"),
    }
}

#[cfg(test)]
mod tests;
