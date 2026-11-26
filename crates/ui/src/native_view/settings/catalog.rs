//! What Settings lists: its sections, each a few groups of rows, and the search that narrows them
//! (design `docs/design/settings`). A row is a setting's title, a muted line
//! under it, and one control at its end: a switch turns something on or off, a segmented control
//! picks one of two, a select one of three or more, a button runs an action, and muted text is a
//! fact the user cannot change.

#[cfg(test)]
mod tests;

use gpui_kit::{AnyElement, ElementId, IntoElement, SharedString};
use skillbinder_design::Text;
use skillbinder_design::tokens::text;

/// A section of Settings, in the order the page lists them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Section {
    General,
    Agents,
    Library,
    Updates,
    Troubleshooting,
}

impl Section {
    pub const ALL: [Self; 5] = [
        Self::General,
        Self::Agents,
        Self::Library,
        Self::Updates,
        Self::Troubleshooting,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Agents => "Agents",
            Self::Library => "Library & sync",
            Self::Updates => "Updates",
            Self::Troubleshooting => "Troubleshooting",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::General => "settings",
            Self::Agents => "robot",
            Self::Library => "libraries",
            Self::Updates => "refresh-03",
            Self::Troubleshooting => "wrench",
        }
    }

    /// The element id stem of the section, its label, and its tab.
    pub fn id(self) -> &'static str {
        match self {
            Self::General => "settings-general",
            Self::Agents => "settings-agents",
            Self::Library => "settings-library",
            Self::Updates => "settings-updates",
            Self::Troubleshooting => "settings-troubleshooting",
        }
    }
}

/// One setting: its title, the line under it, the words a search also finds it by, and its
/// control.
pub(super) struct SettingRow {
    pub id: String,
    pub title: SharedString,
    /// The muted line under the title, which the search reads.
    pub detail: Option<String>,
    /// Draws the line in place of plain muted text, such as a path in monospace.
    pub subtitle: Option<AnyElement>,
    pub keywords: &'static [&'static str],
    pub leading: Option<AnyElement>,
    /// Content under the title and its line, such as a command to copy.
    pub under: Vec<AnyElement>,
    pub control: Option<AnyElement>,
}

impl SettingRow {
    pub fn new(id: impl Into<String>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            detail: None,
            subtitle: None,
            keywords: &[],
            leading: None,
            under: Vec::new(),
            control: None,
        }
    }

    pub fn detail(self, detail: impl Into<String>) -> Self {
        Self {
            detail: Some(detail.into()),
            ..self
        }
    }

    /// The line under the title drawn as `subtitle`, which reads `detail` for the search.
    pub fn subtitle(self, detail: impl Into<String>, subtitle: impl IntoElement) -> Self {
        Self {
            subtitle: Some(subtitle.into_any_element()),
            ..self.detail(detail)
        }
    }

    pub fn keywords(self, keywords: &'static [&'static str]) -> Self {
        Self { keywords, ..self }
    }

    pub fn leading(self, leading: impl IntoElement) -> Self {
        Self {
            leading: Some(leading.into_any_element()),
            ..self
        }
    }

    pub fn under(mut self, under: impl IntoIterator<Item = AnyElement>) -> Self {
        self.under.extend(under);
        self
    }

    pub fn control(self, control: impl IntoElement) -> Self {
        Self {
            control: Some(control.into_any_element()),
            ..self
        }
    }

    /// Ends the row with `value`, a fact the user can't change, muted in tabular figures.
    pub fn fact(self, id: impl Into<ElementId>, value: impl Into<SharedString>) -> Self {
        self.control(
            Text::labeled(id, value)
                .level(text::BODY_SM)
                .muted()
                .tabular(),
        )
    }

    fn texts(&self) -> impl Iterator<Item = &str> {
        [Some(self.title.as_ref()), self.detail.as_deref()]
            .into_iter()
            .flatten()
            .chain(self.keywords.iter().copied())
    }
}

/// Rows under an optional title and description, in one card. `empty` shows in place of the card
/// while the group has no row and nothing is searched.
#[derive(Default)]
pub(super) struct Group {
    pub title: Option<&'static str>,
    /// A muted line under the title.
    pub description: Option<String>,
    pub rows: Vec<SettingRow>,
    pub empty: Option<AnyElement>,
}

impl Group {
    pub fn new(rows: Vec<SettingRow>) -> Self {
        Self {
            rows,
            ..Self::default()
        }
    }

    pub fn titled(title: &'static str, rows: Vec<SettingRow>) -> Self {
        Self {
            title: Some(title),
            ..Self::new(rows)
        }
    }
}

/// Keeps the rows of `groups` that `query` finds, ignoring case: in their title, their line, or
/// their keywords, or every row of a group or section whose title holds it. Groups left without a
/// row go, and so does what an empty group shows: a search lists only settings.
pub(super) fn search(section: Section, groups: Vec<Group>, query: &str) -> Vec<Group> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return groups;
    }
    let found = |text: &str| text.to_lowercase().contains(&query);
    let whole_section = found(section.title());
    groups
        .into_iter()
        .filter_map(
            |Group {
                 title,
                 description,
                 rows,
                 ..
             }| {
                let whole = whole_section || title.is_some_and(found);
                let rows: Vec<SettingRow> = rows
                    .into_iter()
                    .filter(|row| whole || row.texts().any(found))
                    .collect();
                (!rows.is_empty()).then_some(Group {
                    title,
                    description,
                    rows,
                    empty: None,
                })
            },
        )
        .collect()
}
