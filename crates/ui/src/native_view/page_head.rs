//! A scoped page's header in place of the breadcrumb: the place's glyph, its name in semibold, an
//! optional extra such as an avatar stack, then its folder, muted, with home shortened to `~`,
//! truncating with an ellipsis. Below it, the page's skills in one card under the search shared
//! with Library.

use super::library_copies::shown_dir;
use super::library_filters::{empty_state, glyph};
use super::skill_picker::Place;
use super::snapshot_query::matching;
use super::workspace_icons::menu_icon;
use super::{NativeView, Route};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, input::Input, v_flex};
use gpui_kit::{
    AnyElement, Context, Div, InteractiveElement, IntoElement, ParentElement, Styled, Window, div,
    px,
};
use skillbinder_design::page::{GAP, GUTTER};
use skillbinder_design::tokens::text;
use skillbinder_design::{IconSize, Text, icon};
use skillbinder_proto::Skill;

/// The glyph's square.
const ICON: f32 = 18.;
/// The search field's width on a scoped page.
const SEARCH_WIDTH: f32 = 220.;
/// The empty state's distance from the top of a scoped page.
pub(super) const EMPTY_TOP: f32 = 36.;

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

    /// `icon`, `name`, `extra`, and `path` on one line.
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
            .gap(px(8.))
            .child(
                h_flex()
                    .flex_none()
                    .size(px(ICON))
                    .justify_center()
                    .child(icon),
            )
            .child(
                Text::labeled("page-name", name)
                    .level(text::TITLE_SM)
                    .flex_none(),
            )
            .children(extra)
            .child(
                Text::labeled("page-path", shown_dir(path, &self.snapshot.home))
                    .level(text::CAPTION)
                    .muted()
                    .min_w_0()
                    .truncate(),
            )
            .into_any_element()
    }

    /// A scoped page's column, inside the panel's gutter.
    pub(super) fn scoped_page(&self) -> Div {
        v_flex().w_full().p(px(GUTTER)).gap(px(GAP))
    }

    /// `page` with the search, then the `held` skills it keeps in the card `list` draws, or "No
    /// skills match" when it keeps none.
    pub(super) fn scoped_list(
        &self,
        page: Div,
        held: Vec<&Skill>,
        list: impl FnOnce(&Self, &[&Skill], &mut Context<Self>) -> AnyElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let typed = self.search.read(cx).value().trim().to_owned();
        let shown = matching(&self.snapshot, held, &typed.to_lowercase());
        let search = div().w(px(SEARCH_WIDTH)).max_w_full().child(
            Input::new(&self.search)
                .prefix(
                    icon("search")
                        .with_size(IconSize::Sm)
                        .text_color(cx.theme().muted_foreground),
                )
                .bg(cx.theme().background),
        );
        let results = if shown.is_empty() {
            empty_state(
                glyph("search", cx),
                format!("No skills match '{typed}'"),
                cx,
            )
            .into_any_element()
        } else {
            list(self, &shown, cx)
        };
        page.child(search).child(results).into_any_element()
    }
}
