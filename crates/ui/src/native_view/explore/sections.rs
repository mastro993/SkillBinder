//! Explore's sections: Trending cards and Popular creators chips, skeletons while the catalog
//! first loads, or one line with Retry when there is no catalog to show; then Your repositories.

use super::super::NativeView;
use super::super::library::grid_columns;
use super::cards::{skeleton_card, skeleton_chip};
use super::query::{TRENDING, popular_creators, trending};
use gpui_kit::component::{Sizable, h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use skillbinder_design::explore::{CHIP_GAP, SECTION_GAP};
use skillbinder_design::library::GRID_GAP;
use skillbinder_design::page::GAP;
use skillbinder_design::{Button, EmptyState, IconSize, SectionHeader, icon};
use skillbinder_proto::{CatalogState, ExploreSnapshot};

/// How many chips stand in for creators while the catalog first loads.
const SKELETON_CHIPS: usize = 4;

impl NativeView {
    pub(super) fn explore_sections(
        &self,
        explore: &ExploreSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let columns = grid_columns(self.content_width(window));
        let grid = div().grid().grid_cols(columns).gap(px(GRID_GAP));
        let rail = h_flex()
            .id("explore-creators")
            .gap(px(CHIP_GAP))
            .overflow_x_scroll();
        let catalog: Option<(Vec<AnyElement>, Vec<AnyElement>)> =
            match (&explore.catalog, explore.state) {
                (Some(catalog), _) => Some((
                    trending(catalog)
                        .iter()
                        .map(|skill| self.card(skill, cx))
                        .collect(),
                    popular_creators(catalog)
                        .iter()
                        .map(|ranked| self.creator_chip(ranked, cx))
                        .collect(),
                )),
                (None, CatalogState::Loading) => Some((
                    (0..TRENDING).map(skeleton_card).collect(),
                    (0..SKELETON_CHIPS)
                        .map(|index| skeleton_chip(index, cx))
                        .collect(),
                )),
                // The catalog line, or the unreachable line and Retry, says what to do.
                (
                    None,
                    CatalogState::Outdated
                    | CatalogState::Ready
                    | CatalogState::Offline
                    | CatalogState::Unavailable,
                ) => None,
            };
        let unreachable = explore.catalog.is_none()
            && matches!(
                explore.state,
                CatalogState::Offline | CatalogState::Unavailable
            );
        let sections = catalog.map(|(cards, chips)| {
            [
                v_flex()
                    .gap(px(SECTION_GAP))
                    .child(section_header("fire", "Trending"))
                    .child(grid.children(cards)),
                v_flex()
                    .gap(px(SECTION_GAP))
                    .child(section_header("people", "Popular creators"))
                    .child(rail.children(chips)),
            ]
        });
        // Your repositories are kept on this device, so they show without a catalog (C4).
        v_flex()
            .gap(px(GAP))
            .children(unreachable.then(|| self.catalog_unreachable(cx)))
            .children(sections.into_iter().flatten())
            .child(self.repositories_section(explore, cx))
            .into_any_element()
    }

    /// With no catalog cached and none fetched: one line and Retry, in place of both sections.
    fn catalog_unreachable(&self, cx: &mut Context<Self>) -> AnyElement {
        let retry = Button::new("explore-retry")
            .label("Retry")
            .small()
            .on_click(cx.listener(|view, _, _, _| view.client.refresh_catalog()));
        EmptyState::new("explore-unreachable", "Couldn't load the catalog")
            .icon("cloud")
            .action(retry)
            .into_any_element()
    }
}

/// A section's header: its glyph, then its name.
pub(super) fn section_header(glyph: &str, label: &'static str) -> SectionHeader {
    SectionHeader::custom(
        h_flex()
            .gap(px(CHIP_GAP))
            .child(icon(glyph).with_size(IconSize::Sm))
            .child(label),
    )
}
