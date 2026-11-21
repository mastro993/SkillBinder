//! The menu the icon opens: a typed model the backends only draw (ADR 0021), derived from the
//! snapshot and the [`Gate`] alone.

use crate::gate::Gate;
use crate::native_view::{resolved_active, waiting_copies};
use skillbinder_proto::{AppSnapshot, PresetId};

/// What a menu item does.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TrayAction {
    /// Opens the window on the review of the copies that wait for the user.
    ReviewCopies,
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
    pub(crate) fn id(&self) -> String {
        match self {
            Self::ReviewCopies => "review-copies".into(),
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

/// The menu, top to bottom.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrayMenu {
    pub(crate) items: Vec<TrayItem>,
}

impl TrayMenu {
    /// The menu for `snapshot` while `gate` holds (spec rules 7, 8, and 12): the attention item,
    /// the presets, then Open and Quit. Before onboarding finishes, only Open and Quit.
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
                    enabled: !gate.blocked(),
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
        Self { items }
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
