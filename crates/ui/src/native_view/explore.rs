//! The Explore page: the catalog's Trending skills and Popular creators under a search field, or
//! the results a search and a picked source narrow it to, skills.sh's appended below the
//! catalog's, each with its Install action, with one quiet line while the catalog is offline,
//! could not be refreshed, or needs a newer app (ADR 0014), and one while installing needs Git
//! (ADR 0015). Clicking a skill opens its detail dialog.

mod cards;
mod detail;
mod install;
mod query;
mod results;
mod sections;
mod skills_sh;

pub(super) use detail::Detail;

use super::NativeView;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::{
    AnyElement, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Subscription, Window, div, px,
};
use skillbinder_design::library::TOOLBAR_HEIGHT;
use skillbinder_design::page::{GAP, GUTTER};
use skillbinder_design::{Notice, SearchField};
use skillbinder_proto::{CatalogState, ExploreSnapshot};

/// The source Explore is narrowed to.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Scope {
    /// A creator, by id, picked on a chip, a creator chip, a result's source tag, or a detail
    /// dialog.
    Creator(String),
    /// skills.sh's results, picked on its chip.
    SkillsSh,
}

/// What Explore keeps while it shows: its search and the source it is narrowed to.
pub(super) struct Explore {
    search: Entity<InputState>,
    scope: Option<Scope>,
    /// Counts the search's edits, so only the last one asks skills.sh after its pause.
    edits: u64,
    /// A detail dialog's preview is open in the projection, until the dialog closes.
    previewing: bool,
    _search_subscription: Subscription,
}

impl Explore {
    pub(super) fn new(window: &mut Window, cx: &mut Context<NativeView>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search skills"));
        let subscription = cx.subscribe(&search, |view, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                view.search_edited(cx);
                cx.notify();
            }
        });
        Self {
            search,
            scope: None,
            edits: 0,
            previewing: false,
            _search_subscription: subscription,
        }
    }

    /// Clears the search and the picked source, as leaving the page does.
    pub(super) fn clear(&mut self, window: &mut Window, cx: &mut Context<NativeView>) {
        self.scope = None;
        self.search
            .update(cx, |input, cx| input.set_value("", window, cx));
    }

    /// The creator picked, if one is.
    fn creator(&self) -> Option<&str> {
        match &self.scope {
            Some(Scope::Creator(id)) => Some(id),
            Some(Scope::SkillsSh) | None => None,
        }
    }
}

impl NativeView {
    pub(super) fn render_explore(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let explore = &self.snapshot.explore;
        let typed = self.explore.search.read(cx).value().trim().to_owned();
        let search = typed.to_lowercase();
        let toolbar = h_flex()
            .flex_none()
            .h(px(TOOLBAR_HEIGHT))
            .child(SearchField::new("explore-search", &self.explore.search).flex_1());
        let body = if search.is_empty() && self.explore.scope.is_none() {
            self.explore_sections(explore, window, cx)
        } else {
            self.explore_results(explore, &search, &typed, cx)
        };
        v_flex()
            .size_full()
            .px(px(GUTTER))
            .pt(px(GUTTER))
            .gap(px(GAP))
            .child(toolbar)
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        v_flex()
                            .id("explore-scroll")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.content_scroll.handle)
                            .gap(px(GAP))
                            .pb(px(GUTTER))
                            .children(catalog_line(explore))
                            .children(self.install_git_line(cx))
                            .child(body),
                    )
                    .child(
                        self.content_scroll
                            .scrollbar("explore-scrollbar", px(0.), cx),
                    ),
            )
            .into_any_element()
    }

    /// Narrows Explore to the creator `id`, or back to every source with `None`.
    fn pick_creator(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        self.pick(id.map(Scope::Creator), cx);
    }

    /// Narrows Explore to `scope`, or back to every source with `None`.
    fn pick(&mut self, scope: Option<Scope>, cx: &mut Context<Self>) {
        self.explore.scope = scope;
        self.content_scroll.handle.set_offset(Default::default());
        cx.notify();
    }
}

/// The quiet line saying why the catalog shown is not current, when it is not.
fn catalog_line(explore: &ExploreSnapshot) -> Option<AnyElement> {
    let cached = explore.catalog.is_some();
    let text = match explore.state {
        CatalogState::Offline if cached => "Offline · showing the last catalog",
        CatalogState::Unavailable if cached => "Couldn't refresh the catalog",
        CatalogState::Outdated => "Update SkillBinder to see the latest catalog",
        CatalogState::Loading
        | CatalogState::Ready
        | CatalogState::Offline
        | CatalogState::Unavailable => return None,
    };
    Some(
        Notice::new("explore-catalog-line", text)
            .glyph("cloud")
            .into_any_element(),
    )
}
