//! Library's toolbar and empty results. The toolbar reads: the search field, then on Library the
//! All, Active, and Inactive segments with their counts and the Updates chip, a flexible gap, and
//! the List and Grid segments. The filters last the session, like the folder tab, and combine with
//! it and the search; "Clear filters" resets them and the search but keeps the folder tab.

use super::controls::chip::{chip, outline_button};
use super::controls::logo::Logo;
use super::controls::segmented::{
    SEGMENT_GAP, SEGMENT_PADDING, TRACK_PADDING, count, glyph, segment, track,
};
use super::controls::{app_icon, text, tooltip::tip};
use super::snapshot_query::{ActiveFilter, FilterCounts, LibraryFilter};
use super::{NativeView, Route};
use gpui_kit::component::{ActiveTheme, h_flex, input::Input, v_flex};
use gpui_kit::{
    AnyElement, Context, Div, FontWeight, InteractiveElement, IntoElement, ParentElement, Role,
    StatefulInteractiveElement, Styled, Toggled, div, prelude::FluentBuilder, px, relative,
};
use skillbinder_proto::LibraryView;
use skillbinder_theme::library::TOOLBAR_HEIGHT;

/// The gap between the segments' track and the Updates chip.
const FILTERS_GAP: f32 = 6.;
/// The Updates chip's border, padding, glyph, and gap between glyph and count.
const CHIP_BORDER: f32 = 1.;
const CHIP_PADDING: f32 = 8.;
const UPDATE_GLYPH: f32 = 14.;
const CHIP_GAP: f32 = 5.;

impl NativeView {
    /// The toolbar; `counts` are given on Library, which shows the filters.
    pub(super) fn library_toolbar(
        &self,
        counts: Option<FilterCounts>,
        width: f32,
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
            .children(counts.map(|counts| self.library_filters(counts, width, cx)))
            .child(div().flex_1())
            .child(self.view_segments(cx))
            .into_any_element()
    }

    /// The All, Active, and Inactive segments, each with its count, then the Updates chip, in a
    /// toolbar `toolbar` points wide. Their widths follow from their text, which Chrome keeps to
    /// the fraction; GPUI rounds a width in points to the device pixel but keeps a fraction of the
    /// parent's width, so each box takes its exact width as one, and the field and the gap beside
    /// them split the rest as the prototype's do.
    fn library_filters(&self, counts: FilterCounts, toolbar: f32, cx: &mut Context<Self>) -> Div {
        let chosen = self.library_filter.active;
        let label =
            |label: &str| text::width(&label.into(), 12.5, text::MEDIUM, Default::default(), cx);
        let figures = |value: usize| {
            text::width(
                &value.to_string().into(),
                11.,
                text::MEDIUM,
                text::tabular(),
                cx,
            )
        };
        let widths = [
            ("All", counts.all),
            ("Active", counts.active),
            ("Inactive", counts.inactive),
        ]
        .map(|(name, value)| 2. * SEGMENT_PADDING + label(name) + SEGMENT_GAP + figures(value));
        let segments_width: f32 = widths.iter().sum();
        let inner = segments_width + 2. * TRACK_PADDING;
        let track_width = inner + 2. * TRACK_PADDING;
        let updates = text::width(
            &counts.updates.to_string().into(),
            12.,
            FontWeight::NORMAL,
            text::tabular(),
            cx,
        );
        let chip_width = 2. * (CHIP_BORDER + CHIP_PADDING) + UPDATE_GLYPH + CHIP_GAP + updates;
        let filters_width = track_width + FILTERS_GAP + chip_width;
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
        .into_iter()
        .zip(widths)
        .map(|((key, label, filter, value), width)| {
            segment(format!("library-filter-{key}"), chosen == filter, cx)
                .w(relative(width / inner))
                .role(Role::Tab)
                .aria_label(label)
                .child(text::run(label, 12.5, text::MEDIUM, false, cx))
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
            .w(relative(filters_width / toolbar))
            .gap(px(FILTERS_GAP))
            .child(
                track("library-filter", cx)
                    .w(relative(track_width / filters_width))
                    .role(Role::TabList)
                    .children(segments),
            )
            .child(self.updates_chip(counts.updates, chip_width / filters_width, cx))
    }

    /// The Updates chip: the update glyph and how many skills have one; filled while on.
    fn updates_chip(&self, value: usize, share: f32, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let on = self.library_filter.updates;
        let (glyph_color, count_color) = if on {
            (theme.background, theme.background.opacity(0.7))
        } else {
            (theme.foreground, theme.muted_foreground)
        };
        let (fill, hover) = (theme.foreground, theme.muted);
        let chip = chip("library-filter-updates", TOOLBAR_HEIGHT, cx)
            .w(relative(share))
            .px(px(CHIP_PADDING))
            .gap(px(CHIP_GAP))
            .cursor_pointer()
            .role(Role::Button)
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .aria_label("Updates")
            .when(on, |chip| chip.bg(fill).border_color(fill))
            .when(!on, |chip| chip.hover(move |style| style.bg(hover)))
            .child(Logo::Glyph("icons/upload-circle.svg").element(
                UPDATE_GLYPH,
                glyph_color.into(),
                false,
            ))
            .child(
                text::sized(
                    text::run(value.to_string(), 12., FontWeight::NORMAL, true, cx),
                    12.,
                )
                .text_color(count_color),
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
            text::observed(text::sized(div(), 14.), "empty-title")
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(title),
        )
}
