//! Library's toolbar and empty results. The toolbar reads: the search field, then the All,
//! Active, and Inactive segments with their counts and the Updates chip, a flexible gap, and
//! the List and Grid segments. The segments are Kit's segmented tab bars. The filters last the
//! session, like the folder tab, and combine with it and the search; "Clear filters" resets them
//! and the search but keeps the folder tab.

use super::NativeView;
use super::snapshot_query::{ActiveFilter, FilterCounts, LibraryFilter};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{Sizable, h_flex};
use gpui_kit::{
    AnyElement, ClickEvent, Context, Div, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, Window, div, px,
};
use skillbinder_design::library::TOOLBAR_HEIGHT;
use skillbinder_design::tokens::space;
use skillbinder_design::{
    Button, EmptyState, IconSize, SearchField, SegmentedFilter, Text, TextTone, ToggleChip,
    focus_ring, icon, tooltip,
};
use skillbinder_proto::LibraryView;

/// The All, Active, and Inactive filters, in the order of their segments.
const FILTERS: [(ActiveFilter, &str); 3] = [
    (ActiveFilter::All, "All"),
    (ActiveFilter::Active, "Active"),
    (ActiveFilter::Inactive, "Inactive"),
];
/// The List and Grid views, in the order of their segments, with their glyphs.
const VIEWS: [(LibraryView, &str, &str); 2] = [
    (LibraryView::List, "List", "list-view"),
    (LibraryView::Grid, "Grid", "grid-view"),
];
/// The gap between the filter segments and the Updates chip.
const FILTERS_GAP: f32 = space::SM;

impl NativeView {
    /// The toolbar, its filters with their `counts`.
    pub(super) fn library_toolbar(
        &self,
        counts: FilterCounts,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let search = SearchField::new("library-search", &self.search)
            .flex_basis(px(120.))
            .flex_grow(1.)
            .flex_shrink(1.)
            .min_w(px(100.));
        h_flex()
            .flex_none()
            .h(px(TOOLBAR_HEIGHT))
            .gap(px(space::MD))
            .child(search)
            .child(self.library_filters(counts, window, cx))
            .child(div().flex_1())
            .child(self.view_segments(cx))
            .into_any_element()
    }

    /// The All, Active, and Inactive segments, each with its count, then the Updates chip.
    fn library_filters(
        &self,
        counts: FilterCounts,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let chosen = FILTERS
            .iter()
            .position(|(filter, _)| *filter == self.library_filter.active)
            .unwrap_or_default();
        let values = [counts.all, counts.active, counts.inactive];
        let select = cx.listener(|view, index: &usize, window, cx| {
            let Some(&(filter, _)) = FILTERS.get(*index) else {
                return;
            };
            view.library_filter.active = filter;
            view.close_place_popover(window, cx);
            cx.notify();
        });
        h_flex()
            .flex_none()
            .gap(px(FILTERS_GAP))
            .child(
                SegmentedFilter::new(
                    "library-filter",
                    FILTERS
                        .iter()
                        .zip(values)
                        .map(|(&(_, label), value)| (label, value)),
                )
                .selected(chosen)
                .on_select(move |index, window, cx| select(&index, window, cx)),
            )
            .child(self.updates_chip(counts.updates, window, cx))
    }

    /// The Updates chip: a toggle chip holding the update glyph and how many skills have one,
    /// outlined while off and accent while on. Tab reaches it and Enter or Space toggles it; only focus that a key gave
    /// it shows Kit's focus ring.
    fn updates_chip(
        &self,
        value: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let on = self.library_filter.updates;
        let focus = window
            .use_keyed_state("library-filter-updates-focus", cx, |_, cx| {
                cx.focus_handle()
            })
            .read(cx)
            .clone();
        let toggle = cx.listener(|view, _: &ClickEvent, window, cx| {
            view.library_filter.updates = !view.library_filter.updates;
            view.close_place_popover(window, cx);
            cx.notify();
        });
        let count = Text::labeled_count("count", value).tone(if on {
            TextTone::Default
        } else {
            TextTone::Muted
        });
        let chip = ToggleChip::new("library-filter-updates", on)
            .accessibility_label("Updates")
            .track_focus(&focus)
            .on_change(move |_, event, window, cx| toggle(event, window, cx))
            .h(px(TOOLBAR_HEIGHT))
            .child(icon("upload-circle").with_size(IconSize::Sm))
            .child(count)
            .tooltip(tooltip("Updates"));
        focus_ring(chip, &focus, window, cx).into_any_element()
    }

    /// The List and Grid segments; the choice is kept across launches.
    fn view_segments(&self, cx: &mut Context<Self>) -> AnyElement {
        let chosen = VIEWS
            .iter()
            .position(|(view, ..)| *view == self.library_view)
            .unwrap_or_default();
        let segments = VIEWS.map(|(_, label, icon)| {
            Tab::new()
                .aria_label(label)
                .icon(skillbinder_design::icon(icon))
                .cursor_pointer()
                .tooltip(tooltip(label))
        });
        TabBar::new("library-view")
            .segmented()
            .flex_none()
            .selected_index(chosen)
            .children(segments)
            .on_click(cx.listener(|view, index: &usize, window, cx| {
                let Some(&(library_view, ..)) = VIEWS.get(*index) else {
                    return;
                };
                view.library_view = library_view;
                view.close_place_popover(window, cx);
                view.persist_preferences(cx);
                cx.notify();
            }))
            .into_any_element()
    }

    /// Why nothing is listed: a filter or the search matches nothing, with Clear filters, or the
    /// chosen folder holds no skill.
    pub(super) fn empty_results(&self, typed: &str, cx: &mut Context<Self>) -> AnyElement {
        if self.library_filter.narrows() || !typed.is_empty() {
            let clear = Button::new("library-clear-filters")
                .label("Clear filters")
                .small()
                .on_click(cx.listener(|view, _, window, cx| {
                    view.library_filter = LibraryFilter::default();
                    view.search
                        .update(cx, |input, cx| input.set_value("", window, cx));
                    cx.notify();
                }));
            return EmptyState::new("library-empty", "No skills match")
                .icon("search")
                .action(clear)
                .into_any_element();
        }
        match self.shown_folder() {
            Some(folder) => {
                EmptyState::new("library-empty", format!("No skills in {}", folder.name))
                    .icon("folder")
                    .into_any_element()
            }
            None => self.library_empty(cx),
        }
    }
}
