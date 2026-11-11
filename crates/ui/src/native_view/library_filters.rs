//! Library's toolbar and empty results. The toolbar reads: the search field, then on Library the
//! All, Active, and Inactive segments with their counts and the Updates chip, a flexible gap, and
//! the List and Grid segments. The segments are Kit's segmented tab bars. The filters last the
//! session, like the folder tab, and combine with it and the search; "Clear filters" resets them
//! and the search but keeps the folder tab.

use super::controls::exact_text::{sized, tabular};
use super::controls::{app_icon, keyboard_focused, text_button, tooltip};
use super::snapshot_query::{ActiveFilter, FilterCounts, LibraryFilter};
use super::{NativeView, Route};
use gpui_kit::base::Toggle;
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{ActiveTheme, Size, ThemeStyled, h_flex, input::Input, v_flex};
use gpui_kit::{
    AnyElement, App, ClickEvent, Context, Div, FontWeight, InteractiveElement, IntoElement,
    ParentElement, StatefulInteractiveElement, Styled, Text, Window, div, prelude::FluentBuilder,
    px,
};
use skillbinder_proto::LibraryView;
use skillbinder_theme::MEDIUM;
use skillbinder_theme::library::TOOLBAR_HEIGHT;

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
const FILTERS_GAP: f32 = 6.;
/// The Updates chip's padding, glyph, and gap between glyph and count.
const CHIP_PADDING: f32 = 8.;
const UPDATE_GLYPH: f32 = 14.;
const CHIP_GAP: f32 = 5.;

impl NativeView {
    /// The toolbar; `counts` are given on Library, which shows the filters.
    pub(super) fn library_toolbar(
        &self,
        counts: Option<FilterCounts>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let search = div()
            .flex_basis(px(120.))
            .flex_grow(1.)
            .flex_shrink(1.)
            .min_w(px(100.))
            .child(
                Input::new(&self.search)
                    .prefix(app_icon("search").size(px(15.)).text_color(muted))
                    .bg(cx.theme().background),
            );
        h_flex()
            .flex_none()
            .h(px(TOOLBAR_HEIGHT))
            .gap(px(8.))
            .child(search)
            .children(counts.map(|counts| self.library_filters(counts, window, cx)))
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
        let segments = FILTERS.iter().zip(values).map(|(&(_, label), value)| {
            Tab::new()
                .aria_label(label)
                .cursor_pointer()
                .child(div().text_size(px(12.5)).font_weight(MEDIUM).child(label))
                .child(count(value, cx))
        });
        h_flex()
            .flex_none()
            .gap(px(FILTERS_GAP))
            .child(
                TabBar::new("library-filter")
                    .segmented()
                    .flex_none()
                    .selected_index(chosen)
                    .children(segments)
                    .on_click(cx.listener(|view, index: &usize, window, cx| {
                        let Some(&(filter, _)) = FILTERS.get(*index) else {
                            return;
                        };
                        view.library_filter.active = filter;
                        view.close_place_popover(window, cx);
                        cx.notify();
                    })),
            )
            .child(self.updates_chip(counts.updates, window, cx))
    }

    /// The Updates chip: Kit's base toggle holding the update glyph and how many skills have one,
    /// filled while on. Tab reaches it and Enter or Space toggles it; only focus that a key gave
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
        let ring = keyboard_focused(&focus, window);
        let toggle = cx.listener(|view, _: &ClickEvent, window, cx| {
            view.library_filter.updates = !view.library_filter.updates;
            view.close_place_popover(window, cx);
            cx.notify();
        });
        let theme = cx.theme();
        let (glyph_color, count_color) = if on {
            (theme.background, theme.background.opacity(0.7))
        } else {
            (theme.foreground, theme.muted_foreground)
        };
        let (fill, hover) = (theme.foreground, theme.muted);
        let chip = Toggle::new("library-filter-updates")
            .pressed(on)
            .accessibility_label("Updates")
            .track_focus(&focus)
            .on_change(move |_, event, window, cx| toggle(event, window, cx))
            .flex_none()
            .h(px(TOOLBAR_HEIGHT))
            .px(px(CHIP_PADDING))
            .gap(px(CHIP_GAP))
            .border_1()
            .border_color(theme.border)
            .rounded_full()
            .bg(theme.background)
            .cursor_pointer()
            .when(on, |chip| chip.bg(fill).border_color(fill))
            .when(!on, |chip| chip.hover(move |style| style.bg(hover)))
            .child(
                app_icon("upload-circle")
                    .size(px(UPDATE_GLYPH))
                    .text_color(glyph_color),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .font_features(tabular())
                    .text_color(count_color)
                    .child(Text::new("count".into(), value.to_string().into())),
            )
            .tooltip(tooltip("Updates"))
            .when(ring, |chip| chip.focus_ring_style(window, cx));
        chip.into_any_element()
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
                .icon(app_icon(icon))
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
    /// chosen folder holds no skill. A workspace's page keeps its plain search message.
    pub(super) fn empty_results(&self, typed: &str, cx: &mut Context<Self>) -> AnyElement {
        if self.route != Route::Library {
            if typed.is_empty() {
                return div().into_any_element();
            }
            return div()
                .py_8()
                .text_center()
                .text_color(cx.theme().muted_foreground)
                .child(format!("No skills match '{typed}'"))
                .into_any_element();
        }
        if self.library_filter.narrows() || !typed.is_empty() {
            let clear = text_button("library-clear-filters", "Clear filters", Size::Small)
                .outline()
                .on_click(cx.listener(|view, _, window, cx| {
                    view.library_filter = LibraryFilter::default();
                    view.search
                        .update(cx, |input, cx| input.set_value("", window, cx));
                    cx.notify();
                }));
            return empty_state("search", "No skills match".to_owned(), cx)
                .child(div().mt(px(8.)).child(clear))
                .into_any_element();
        }
        match self.shown_folder() {
            Some(folder) => empty_state("folder", format!("No skills in {}", folder.name), cx)
                .into_any_element(),
            None => div().into_any_element(),
        }
    }
}

/// The empty state's `icon` glyph in a 40 px rounded square, then its title.
fn empty_state(icon: &str, title: String, cx: &Context<NativeView>) -> Div {
    let theme = cx.theme();
    v_flex()
        .w_full()
        .items_center()
        .gap(px(6.))
        .py(px(36.))
        .px(px(16.))
        .text_color(theme.muted_foreground)
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(40.))
                .mb(px(4.))
                .rounded(px(10.))
                .bg(theme.muted)
                .child(app_icon(icon).size(px(20.)).text_color(theme.foreground)),
        )
        .child(
            sized(div().id("empty-title"), 14.)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(title),
        )
}

/// A filter segment's count: muted 11 px medium tabular figures, its text `count`.
fn count(value: usize, cx: &App) -> Div {
    div()
        .ml(px(5.))
        .text_size(px(11.))
        .font_weight(MEDIUM)
        .font_features(tabular())
        .text_color(cx.theme().muted_foreground)
        .child(Text::new("count".into(), value.to_string().into()))
}
