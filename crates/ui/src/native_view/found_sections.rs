//! The sections a workspace's, an agent's, or Universal's page lists under its skills (ADR 0036,
//! design U1 to U9): Not in library, open on every page visit, whose head ends with Ignore all,
//! then Ignored, collapsed on every page visit. Each head is a disclosure that folds its rows. A
//! section with no row is absent. The page's search narrows both, and while it matches a section's
//! skills, that section shows open with only those rows.

mod rows;

use super::NativeView;
use super::ignore_actions::{Flash, flash_overlay};
use super::snapshot_query::{FoundHere, FoundRow};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, Div, InteractiveElement, IntoElement, ParentElement, Styled, px,
};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{Button, CardList, Text, icon};
use skillbinder_proto::Workspace;

/// What the sections keep while their page shows.
#[derive(Debug, Default)]
pub(super) struct FoundSections {
    /// Whether Not in library is folded; leaving the page opens it.
    pub not_in_library_folded: bool,
    /// Whether Ignored is open; leaving the page closes it (U7).
    pub ignored_open: bool,
    /// What flashes once: the Ignored head, or a skill brought back (U5, U7).
    pub flash: Option<Flash>,
}

/// A page's found skills, and how its card reads with nothing on there.
pub(super) struct FoundPage<'a> {
    pub found: FoundHere<'a>,
    /// The page's workspace, whose rows name the directories holding them.
    pub workspace: Option<&'a Workspace>,
    /// The card's one line while nothing is on there, such as "No skills on in docs-site" (U9).
    pub nothing_on: String,
}

impl NativeView {
    /// The sections of `page` with the rows the lowercase `search` keeps; none when it keeps
    /// none.
    pub(super) fn found_sections(
        &self,
        page: &FoundPage<'_>,
        search: &str,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let kept = |rows: &'_ [FoundRow<'_>]| -> usize {
            rows.iter().filter(|row| row.matches(search)).count()
        };
        let mut sections = Vec::new();
        if kept(&page.found.not_in_library) > 0 {
            sections.push(self.not_in_library(page, search, cx));
        }
        if kept(&page.found.ignored) > 0 {
            sections.push(self.ignored_section(page, search, cx));
        }
        sections
    }

    /// Not in library: its disclosure head with Ignore all, which ignores every row the section
    /// lists, those the search hides included (U5), then while it is open, or the search matches
    /// some, the rows the search keeps.
    fn not_in_library(
        &self,
        page: &FoundPage<'_>,
        search: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let listed = &page.found.not_in_library;
        let ignored: Vec<(String, String)> = listed
            .iter()
            .map(|row| (row.shown.slug.clone(), row.key.to_owned()))
            .collect();
        let ignore_all = Button::new("found-ignore-all")
            .label("Ignore all")
            .ghost()
            .small()
            .inactive(self.blocked())
            .on_click(cx.listener(move |view, _, _, cx| view.ignore_found(ignored.clone(), cx)));
        let open = !self.found_sections.not_in_library_folded || !search.is_empty();
        let toggle = disclosure("found-toggle", "Not in library", open, cx, |sections| {
            sections.not_in_library_folded = !sections.not_in_library_folded;
        });
        let head = section_head(
            "found-head",
            toggle,
            listed.len(),
            Some("Agents load these; SkillBinder does not manage them"),
        )
        .child(ignore_all);
        if !open {
            return head.into_any_element();
        }
        let rows = listed
            .iter()
            .filter(|row| row.matches(search))
            .map(|row| self.found_row(page, row, cx));
        section(head, CardList::new("found-list").rows(rows))
    }

    /// Ignored: its disclosure head, which flashes once a skill is ignored, then while it is
    /// open, or the search matches some, the rows the search keeps (U7, U8).
    fn ignored_section(
        &self,
        page: &FoundPage<'_>,
        search: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let listed = &page.found.ignored;
        let open = self.found_sections.ignored_open || !search.is_empty();
        let toggle = disclosure("found-ignored-toggle", "Ignored", open, cx, |sections| {
            sections.ignored_open = !sections.ignored_open;
        });
        let flashing = self
            .found_sections
            .flash
            .as_ref()
            .filter(|flash| flash.flashes(None));
        let head = section_head("found-ignored-head", toggle, listed.len(), None)
            .children(flashing.map(|flash| flash_overlay(flash, cx)));
        if !open {
            return head.into_any_element();
        }
        let rows = listed
            .iter()
            .filter(|row| row.matches(search))
            .map(|row| self.ignored_found_row(page, row, cx));
        section(head, CardList::new("found-ignored-list").rows(rows))
    }
}

/// A section's disclosure, `id`: `label` with a chevron-right glyph, an arrow-down glyph while
/// `open`, whose click runs `toggle` on the sections' state.
fn disclosure(
    id: &'static str,
    label: &'static str,
    open: bool,
    cx: &mut Context<NativeView>,
    toggle: fn(&mut FoundSections),
) -> Button {
    Button::new(id)
        .label(label)
        .icon(icon(if open { "arrow-down" } else { "chevron-right" }))
        .ghost()
        .small()
        .on_click(cx.listener(move |view, _, _, cx| {
            toggle(&mut view.found_sections);
            cx.notify();
        }))
}

/// A section's head, `id`: `title`, its `count` in muted figures, and the muted `hint` if any;
/// the caller ends it with its actions.
fn section_head(
    id: &'static str,
    title: impl IntoElement,
    count: usize,
    hint: Option<&'static str>,
) -> gpui_kit::Stateful<Div> {
    h_flex().id(id).relative().gap(px(space::MD)).child(
        h_flex()
            .flex_1()
            .min_w_0()
            .gap(px(space::MD))
            .child(title)
            .child(Text::labeled_count("count", count).flex_none())
            .children(hint.map(|hint| {
                Text::labeled("hint", hint)
                    .level(text::CAPTION)
                    .muted()
                    .min_w_0()
                    .truncate()
            })),
    )
}

/// `head` over `list`.
fn section(head: impl IntoElement, list: CardList) -> AnyElement {
    v_flex()
        .gap(px(space::SM))
        .child(head)
        .child(list)
        .into_any_element()
}
