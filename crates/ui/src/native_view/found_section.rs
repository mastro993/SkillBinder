//! The Not in library section a workspace's, an agent's, or Universal's page lists under its
//! skills (ADR 0036, design U1 to U9): every found skill of the page's place that is not in
//! library, the ignored ones marked in place. Its head is a disclosure, open on every page visit,
//! that ends with Ignore all. It is absent with no row. The page's search narrows its rows, and
//! while it matches some, the section shows open with only those rows.

mod rows;

use super::NativeView;
use super::ignore_actions::Flash;
use super::snapshot_query::FoundRow;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled,
    prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{Button, CardList, Text, icon};
use skillbinder_proto::Workspace;

/// What the section keeps while its page shows.
#[derive(Debug, Default)]
pub(super) struct FoundSection {
    /// Whether the section is folded; leaving the page opens it.
    pub folded: bool,
    /// The skill brought back by Stop ignoring, which flashes once (U7).
    pub flash: Option<Flash>,
}

/// A page's found skills, and how its card reads with nothing on there.
pub(super) struct FoundPage<'a> {
    /// The skills found in the page's place that are not in library, by key.
    pub found: Vec<FoundRow<'a>>,
    /// The page's workspace, whose rows name the directories holding them.
    pub workspace: Option<&'a Workspace>,
    /// The card's one line while nothing is on there, such as "No skills on in docs-site" (U9).
    pub nothing_on: String,
}

impl NativeView {
    /// The section of `page` with the rows the lowercase `search` keeps, or none when it keeps
    /// none. Ignore all ignores every skill it lists that is not ignored yet, those the search
    /// hides included (U5).
    pub(super) fn render_found_section(
        &self,
        page: &FoundPage<'_>,
        search: &str,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let shown: Vec<&FoundRow<'_>> = page
            .found
            .iter()
            .filter(|row| row.matches(search))
            .collect();
        if shown.is_empty() {
            return None;
        }
        let offered: Vec<(String, String)> = page
            .found
            .iter()
            .filter(|row| !row.ignored)
            .map(|row| (row.shown.slug.clone(), row.key.to_owned()))
            .collect();
        let ignore_all = (!offered.is_empty()).then(|| {
            Button::new("found-ignore-all")
                .label("Ignore all")
                .ghost()
                .small()
                .inactive(self.blocked())
                .on_click(cx.listener(move |view, _, _, cx| view.ignore_found(offered.clone(), cx)))
        });
        let open = !self.found_section.folded || !search.is_empty();
        let toggle = Button::new("found-toggle")
            .label("Not in library")
            .icon(icon(if open { "arrow-down" } else { "chevron-right" }))
            .ghost()
            .small()
            .on_click(cx.listener(|view, _, _, cx| {
                view.found_section.folded = !view.found_section.folded;
                cx.notify();
            }));
        let hint = "Agents load these; SkillBinder does not manage them";
        let head = h_flex()
            .id("found-head")
            .gap(px(space::MD))
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(space::MD))
                    .child(toggle)
                    .child(Text::labeled_count("count", page.found.len()).flex_none())
                    .child(
                        Text::labeled("hint", hint)
                            .level(text::CAPTION)
                            .muted()
                            .min_w_0()
                            .truncate(),
                    ),
            )
            .children(ignore_all);
        let rows: Vec<_> = shown
            .into_iter()
            .map(|row| self.found_row(page, row, cx))
            .collect();
        Some(
            v_flex()
                .gap(px(space::SM))
                .child(head)
                .when(open, |section| {
                    section.child(CardList::new("found-list").rows(rows))
                })
                .into_any_element(),
        )
    }
}
