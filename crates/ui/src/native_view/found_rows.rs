//! A workspace's, an agent's, or Universal's one list (ADR 0036, design U1 to U9): the library
//! skills on there and the found skills of the page's place that are not in library, sorted by
//! slug in one card. A found row is marked Not in library, or Ignored while its slug is ignored.
//! The search row ends with how many found skills are not in library, Ignore all, and Import….

mod row;

use super::NativeView;
use super::library_items::{SkillItem, row_frame};
use super::snapshot_query::FoundRow;
use gpui_kit::component::{ActiveTheme, h_flex};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, px};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{Button, Level, Surface, Text};
use skillbinder_proto::Workspace;
use std::collections::BTreeSet;

/// The hover group of a found row, whose button shows while it is hovered.
const ROW_GROUP: &str = "found-row";

/// A page's found skills, and how its card reads with nothing on there.
pub(super) struct FoundPage<'a> {
    /// The skills found in the page's place that are not in library, by key.
    pub found: Vec<FoundRow<'a>>,
    /// The page's workspace, whose rows name the directories holding them.
    pub workspace: Option<&'a Workspace>,
    /// The card's first line while nothing is on there, such as "No skills on in docs-site" (U9).
    pub nothing_on: String,
}

/// One row of a page's list.
enum Entry<'a, 'b> {
    Skill(&'b SkillItem<'a>),
    Found(&'b FoundRow<'a>),
}

impl Entry<'_, '_> {
    /// The slug the list sorts by, ignoring case.
    fn order(&self) -> String {
        match self {
            Self::Skill(item) => item.skill.id.as_str().to_lowercase(),
            Self::Found(row) => row.key.to_owned(),
        }
    }
}

impl NativeView {
    /// The page's card: `items`, the library skills on there the search kept, and the found skills
    /// the lowercase `search` keeps, by slug. While nothing is on there and nothing is searched,
    /// its first line says so.
    pub(super) fn page_list(
        &self,
        items: &[SkillItem<'_>],
        page: &FoundPage<'_>,
        search: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut entries: Vec<Entry<'_, '_>> = items
            .iter()
            .map(Entry::Skill)
            .chain(
                page.found
                    .iter()
                    .filter(|row| row.matches(search))
                    .map(Entry::Found),
            )
            .collect();
        entries.sort_by_cached_key(Entry::order);
        let nothing_on = (items.is_empty() && search.is_empty()).then(|| {
            let last = entries.is_empty();
            row_frame("page-nothing-on".to_owned(), ROW_GROUP, 0, last).child(
                Text::labeled("nothing-on", page.nothing_on.clone())
                    .level(text::BODY_SM)
                    .muted(),
            )
        });
        let first = usize::from(nothing_on.is_some());
        let last = (first + entries.len()).saturating_sub(1);
        let rows: Vec<AnyElement> = entries
            .iter()
            .enumerate()
            .map(|(at, entry)| {
                let index = first + at;
                match entry {
                    Entry::Skill(item) => self.skill_row(item, index, index == last, false, cx),
                    Entry::Found(row) => self.found_row(page, row, index, index == last, cx),
                }
            })
            .collect();
        Surface::new(Level::Flat)
            .apply(gpui_kit::component::v_flex(), cx.theme())
            .children(nothing_on)
            .children(rows)
            .into_any_element()
    }

    /// How many found skills are not in library, Ignore all, which ignores every one not ignored
    /// yet, those the search hides included, and Import…, which opens the Import skills dialog
    /// with only those (U5, U6); nothing when every one is ignored.
    pub(super) fn found_tools(
        &self,
        page: &FoundPage<'_>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let offered: Vec<(String, String)> = page
            .found
            .iter()
            .filter(|row| !row.ignored)
            .map(|row| (row.shown.slug.clone(), row.key.to_owned()))
            .collect();
        if offered.is_empty() {
            return None;
        }
        let count = format!("{} not in library", offered.len());
        let only: BTreeSet<String> = offered.iter().map(|(_, key)| key.clone()).collect();
        let ignore_all = Button::new("found-ignore-all")
            .label("Ignore all")
            .ghost()
            .small()
            .tooltip("Ignore every skill not in library here")
            .inactive(self.blocked())
            .on_click(cx.listener(move |view, _, _, cx| view.ignore_found(offered.clone(), cx)));
        let import = Button::new("found-import-all")
            .label("Import…")
            .small()
            .inactive(self.blocked())
            .on_click(cx.listener(move |view, _, window, cx| {
                view.open_import_dialog_for(only.clone(), window, cx);
            }));
        Some(
            h_flex()
                .ml_auto()
                .flex_none()
                .gap(px(space::MD))
                .child(
                    Text::labeled("found-count", count)
                        .level(text::CAPTION)
                        .muted()
                        .tabular(),
                )
                .child(ignore_all)
                .child(import)
                .into_any_element(),
        )
    }
}
