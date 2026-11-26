//! A scoped page's header in place of the breadcrumb: the place's glyph, its name in semibold, an
//! optional extra such as an avatar stack, a dot, then its folder in mono, muted, with home
//! shortened to `~`, truncating with an ellipsis. Below it, the page's skills in one card under
//! the search shared with Library, then its found skills' section.

use super::found_section::FoundPage;
use super::library_copies::shown_dir;
use super::skill_picker::Place;
use super::snapshot_query::matching;
use super::workspace_icons::menu_icon;
use super::{NativeView, Route};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, Div, InteractiveElement, IntoElement, ParentElement, Styled, Window, div,
    px,
};
use skillbinder_design::page::{GAP, GUTTER};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{CardList, EmptyState, IconSize, ListRow, SearchField, Text};
use skillbinder_proto::Skill;

/// The search field's width on a scoped page: layout measured from the approved scoped-page
/// prototype.
const SEARCH_WIDTH: f32 = 220.;

impl NativeView {
    /// The header's title slot: Library's folder tabs, a scoped page's head, or the breadcrumb.
    pub(super) fn render_title(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        match &self.route {
            Route::Library => self.render_folder_tabs(window, cx),
            Route::Workspace(id) => match self.workspace(id) {
                Some(workspace) => self.page_head(
                    menu_icon(workspace, cx),
                    workspace.name.clone(),
                    None,
                    &workspace.path,
                    cx,
                ),
                None => div().into_any_element(),
            },
            Route::Agent(id) => self.agent_head(&Place::Agent(id.clone()), cx),
            Route::Universal => self.agent_head(&Place::Universal, cx),
            _ => self.render_breadcrumb(cx).into_any_element(),
        }
    }

    /// `icon`, `name`, `extra`, a dot, and `path` on one line.
    pub(super) fn page_head(
        &self,
        icon: AnyElement,
        name: String,
        extra: Option<AnyElement>,
        path: &str,
        _cx: &Context<Self>,
    ) -> AnyElement {
        h_flex()
            .id("page-head")
            .min_w_0()
            .gap(px(space::MD))
            .child(
                h_flex()
                    .flex_none()
                    .size(px(IconSize::Lg.points()))
                    .justify_center()
                    .child(icon),
            )
            .child(
                Text::labeled("page-name", name)
                    .level(text::TITLE_MD)
                    .flex_none(),
            )
            .children(extra)
            .child(Text::new("·").level(text::TITLE_MD).muted().flex_none())
            .child(
                Text::labeled("page-path", shown_dir(path, &self.snapshot.home))
                    .level(text::MONO_SM)
                    .muted()
                    .min_w_0()
                    .truncate(),
            )
            .into_any_element()
    }

    /// A scoped page's column, inside the page's gutter.
    pub(super) fn scoped_page(&self) -> Div {
        v_flex().w_full().p(px(GUTTER)).gap(px(GAP))
    }

    /// `page` with the search, then the `held` skills it keeps in the card `list` draws, or
    /// while nothing is on there, the card's one line; then the `found` section with the rows it keeps. "No
    /// skills match" replaces them while it keeps nothing (U8, U9).
    pub(super) fn scoped_list(
        &self,
        page: Div,
        held: Vec<&Skill>,
        found: &FoundPage<'_>,
        list: impl FnOnce(&Self, &[&Skill], &mut Context<Self>) -> AnyElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let typed = self.search.read(cx).value().trim().to_owned();
        let search = typed.to_lowercase();
        let nothing_on = held.is_empty();
        let shown = matching(&self.snapshot, held, &search);
        let card = if !shown.is_empty() {
            Some(list(self, &shown, cx))
        } else if nothing_on && search.is_empty() {
            Some(nothing_on_card(&found.nothing_on))
        } else {
            None
        };
        let section = self.render_found_section(found, &search, cx);
        let page = page.child(SearchField::new("page-search", &self.search).width(SEARCH_WIDTH));
        if card.is_none() && section.is_none() {
            return page
                .child(
                    EmptyState::new("page-empty", format!("No skills match '{typed}'"))
                        .icon("search"),
                )
                .into_any_element();
        }
        page.children(card).children(section).into_any_element()
    }
}

/// A card of one muted line for a page with nothing on that lists found skills (U9).
fn nothing_on_card(line: &str) -> AnyElement {
    CardList::new("page-nothing-on")
        .row(
            ListRow::new("page-nothing-on-line").child(
                Text::labeled("nothing-on", line.to_owned())
                    .level(text::BODY_SM)
                    .muted(),
            ),
        )
        .into_any_element()
}
