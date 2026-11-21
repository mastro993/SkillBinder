//! The menu the icon opens: a typed model the backends only draw (ADR 0021).

/// What a menu item does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TrayAction {
    /// Opens the window, or brings the open one forward.
    OpenWindow,
    /// Quits as ⌘Q or Ctrl+Q does.
    Quit,
}

impl TrayAction {
    const ALL: [Self; 2] = [Self::OpenWindow, Self::Quit];

    /// The item's stable id.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::OpenWindow => "open-window",
            Self::Quit => "quit",
        }
    }

    /// The action whose item has `id`.
    #[cfg_attr(target_os = "linux", allow(dead_code))]
    pub(crate) fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.id() == id)
    }
}

/// One row of the menu.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TrayItem {
    Action {
        action: TrayAction,
        label: String,
        /// The key that runs it with ⌘ on macOS, or with Ctrl elsewhere.
        shortcut: Option<char>,
    },
    Separator,
}

/// The menu, top to bottom.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TrayMenu {
    pub(crate) items: Vec<TrayItem>,
}

impl TrayMenu {
    /// Open SkillBinder, a separator, and Quit SkillBinder with the platform's quit shortcut
    /// (spec rule 7.5).
    pub(crate) fn new() -> Self {
        Self {
            items: vec![
                TrayItem::Action {
                    action: TrayAction::OpenWindow,
                    label: "Open SkillBinder".into(),
                    shortcut: None,
                },
                TrayItem::Separator,
                TrayItem::Action {
                    action: TrayAction::Quit,
                    label: "Quit SkillBinder".into(),
                    shortcut: Some('Q'),
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TrayAction, TrayItem, TrayMenu};

    #[test]
    fn menu_opens_the_window_and_quits() {
        let rows: Vec<_> = TrayMenu::new()
            .items
            .into_iter()
            .map(|item| match item {
                TrayItem::Action {
                    action,
                    label,
                    shortcut,
                } => Some((action, label, shortcut)),
                TrayItem::Separator => None,
            })
            .collect();
        assert_eq!(
            rows,
            [
                Some((TrayAction::OpenWindow, "Open SkillBinder".into(), None)),
                None,
                Some((TrayAction::Quit, "Quit SkillBinder".into(), Some('Q'))),
            ]
        );
    }

    #[test]
    fn every_action_is_found_by_its_id() {
        for action in TrayAction::ALL {
            assert_eq!(TrayAction::from_id(action.id()), Some(action));
        }
        assert_eq!(TrayAction::from_id("preset"), None);
    }
}
