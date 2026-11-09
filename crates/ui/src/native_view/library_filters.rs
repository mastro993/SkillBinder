//! Library's toolbar and empty results. The toolbar reads: the search field, then on Library the
//! All, Active, and Inactive segments with their counts and the Updates chip, a flexible gap, and
//! the List and Grid segments. The filters last the session, like the folder tab, and combine with
//! it and the search; "Clear filters" resets them and the search but keeps the folder tab.

use super::controls::chip::{chip, outline_button};
use super::controls::logo::Logo;
use super::controls::segmented::{count, glyph, segment, track};
use super::controls::{app_icon, text, tooltip::tip};
use super::snapshot_query::{ActiveFilter, FilterCounts, LibraryFilter};
use super::{NativeView, Route};
use gpui_kit::component::{ActiveTheme, h_flex, input::Input, v_flex};
use gpui_kit::{
    AnyElement, Context, Div, FontWeight, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, Toggled, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::LibraryView;
use skillbinder_theme::library::TOOLBAR_HEIGHT;

impl NativeView {
    /// The toolbar; `counts` are given on Library, which shows the filters.
    pub(super) fn library_toolbar(
        &self,
        counts: Option<FilterCounts>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let field = Input::new(&self.search)
            .prefix(app_icon("search").size(px(15.)).text_color(muted))
            .bg(cx.theme().background);
        let search = div()
            .flex_basis(px(120.))
            .flex_grow(1.)
            .flex_shrink(1.)
            .min_w(px(100.))
            .child(
                // The glyph sits 8 points from the field's outer edge and the text 29, as the
                // prototype places them over its 1-point border. `Input::h` sizes only a
                // multi-line field, so the height goes through `Styled`; Kit's 8-point vertical
                // padding would clip the text's descenders at that height.
                Styled::h(field, px(TOOLBAR_HEIGHT))
                    .py(px(0.))
                    .pl(px(7.))
                    .pr(px(8.))
                    .gap(px(6.))
                    .rounded(px(6.))
                    .text_size(px(13.)),
            );
        h_flex()
            .flex_none()
            .h(px(TOOLBAR_HEIGHT))
            .gap(px(8.))
            .child(search)
            .children(counts.map(|counts| self.library_filters(counts, cx)))
            .child(div().flex_1())
            .child(self.view_segments(cx))
            .into_any_element()
    }

    /// The All, Active, and Inactive segments, each with its count, then the Updates chip.
    fn library_filters(&self, counts: FilterCounts, cx: &mut Context<Self>) -> Div {
        let chosen = self.library_filter.active;
        let segments = [
            ("all", "All", ActiveFilter::All, counts.all),
            ("active", "Active", ActiveFilter::Active, counts.active),
            (
                "inactive",
                "Inactive",
                ActiveFilter::Inactive,
                counts.inactive,
            ),
        ]
        .map(|(key, label, filter, value)| {
            segment(format!("library-filter-{key}"), chosen == filter, cx)
                .role(Role::Tab)
                .aria_label(label)
                .child(label)
                .child(count(value, cx))
                .on_click(cx.listener(move |view, _, window, cx| {
                    view.library_filter.active = filter;
                    view.close_place_popover(window, cx);
                    cx.notify();
                }))
        });
        h_flex()
            .flex_none()
            .min_w_0()
            .gap(px(6.))
            .child(
                track("library-filter", cx)
                    .role(Role::TabList)
                    .children(segments),
            )
            .child(self.updates_chip(counts.updates, cx))
    }

    /// The Updates chip: the update glyph and how many skills have one; filled while on.
    fn updates_chip(&self, value: usize, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let on = self.library_filter.updates;
        let (glyph_color, count_color) = if on {
            (theme.background, theme.background.opacity(0.7))
        } else {
            (theme.foreground, theme.muted_foreground)
        };
        let (fill, hover) = (theme.foreground, theme.muted);
        let chip = chip("library-filter-updates", TOOLBAR_HEIGHT, cx)
            .px(px(8.))
            .gap(px(5.))
            .cursor_pointer()
            .role(Role::Button)
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .aria_label("Updates")
            .when(on, |chip| chip.bg(fill).border_color(fill))
            .when(!on, |chip| chip.hover(move |style| style.bg(hover)))
            .child(Logo::Glyph("icons/upload-circle.svg").element(14., glyph_color.into(), false))
            .child(
                text::sized(div(), 12.)
                    .font_weight(FontWeight::NORMAL)
                    .font_features(text::tabular())
                    .text_color(count_color)
                    .child(value.to_string()),
            )
            .on_click(cx.listener(|view, _, window, cx| {
                view.library_filter.updates = !view.library_filter.updates;
                view.close_place_popover(window, cx);
                cx.notify();
            }));
        tip(chip, "Updates").into_any_element()
    }

    /// The List and Grid segments; the choice is kept across launches.
    fn view_segments(&self, cx: &mut Context<Self>) -> AnyElement {
        let segments = [
            ("list", "List", "icons/list-view.svg", LibraryView::List),
            ("grid", "Grid", "icons/grid-view.svg", LibraryView::Grid),
        ]
        .map(|(key, label, icon, view)| {
            let chosen = self.library_view == view;
            let segment = segment(format!("library-view-{key}"), chosen, cx)
                .role(Role::Tab)
                .aria_label(label)
                .child(glyph(icon, chosen, cx))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.library_view = view;
                    this.close_place_popover(window, cx);
                    this.persist_preferences(cx);
                    cx.notify();
                }));
            tip(segment, label)
        });
        track("library-view", cx)
            .role(Role::TabList)
            .children(segments)
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
            let clear = outline_button("library-clear-filters", "Clear filters", cx).on_click(
                cx.listener(|view, _, window, cx| {
                    view.library_filter = LibraryFilter::default();
                    view.search
                        .update(cx, |input, cx| input.set_value("", window, cx));
                    cx.notify();
                }),
            );
            return empty_state("icons/search.svg", "No skills match".to_owned(), cx)
                .child(div().mt(px(8.)).child(clear))
                .into_any_element();
        }
        match self.shown_folder() {
            Some(folder) => empty_state(
                "icons/folder.svg",
                format!("No skills in {}", folder.name),
                cx,
            )
            .into_any_element(),
            None => div().into_any_element(),
        }
    }
}

/// The empty state's glyph in a 40 px rounded square, then its title.
fn empty_state(icon: &'static str, title: String, cx: &Context<NativeView>) -> Div {
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
                .child(Logo::Glyph(icon).element(20., theme.foreground.into(), false)),
        )
        .child(
            text::sized(div(), 14.)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(title),
        )
}
