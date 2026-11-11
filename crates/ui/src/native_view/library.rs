//! The Library page: every library skill, or one folder's, narrowed by search and the Active and
//! Updates filters, as a list or grid under a fixed toolbar, each showing where it is on.

use super::NativeView;
use super::library_copies::placements;
use super::library_items::{RowEnd, SkillItem};
use super::snapshot_query::{Places, Query, filter_counts, has_update, library_skills};
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use skillbinder_proto::{LibrarySnapshot, LibraryView, PresetId, Skill};
use skillbinder_theme::library::{
    GRID_GAP, GRID_MIN, NARROW_LIST, NARROW_TILE, PAGE_BOTTOM, PAGE_GAP, PAGE_GUTTER, PAGE_TOP,
};

impl NativeView {
    pub(super) fn render_library(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let (Some(library), Some(active)) = (&self.snapshot.library, self.active_preset_id())
        else {
            return div()
                .child("Finish onboarding to open your library.")
                .into_any_element();
        };
        let typed = self.search.read(cx).value().trim().to_owned();
        let search = typed.to_lowercase();
        let places = Places::new(&self.snapshot);
        let query = Query {
            folder: self.shown_folder().map(|folder| &folder.id),
            search: &search,
            filter: self.library_filter,
            keep: self.place_popover.as_ref().map(|popover| &popover.skill),
        };
        let shown = library_skills(&self.snapshot, &places, &active, query);
        let counts = filter_counts(&self.snapshot, &places, &active, query);
        let width = self.content_width(window);
        let toolbar = self.library_toolbar(counts, window, cx);
        let results = self.library_results(library, &places, &active, &shown, &typed, width, cx);
        let notes: Vec<AnyElement> = self
            .empty_scope_note(library, cx)
            .into_iter()
            .chain(self.render_update_outcome(cx))
            .collect();
        // The toolbar stays put while the list under it scrolls.
        v_flex()
            .size_full()
            .px(px(PAGE_GUTTER))
            .pt(px(PAGE_TOP))
            .gap(px(PAGE_GAP))
            .child(toolbar)
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        v_flex()
                            .id("library-scroll")
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&self.content_scroll.handle)
                            .on_scroll_wheel(cx.listener(|view, _, window, cx| {
                                view.close_place_popover(window, cx);
                            }))
                            .gap(px(PAGE_GAP))
                            .pb(px(PAGE_BOTTOM))
                            .children(notes)
                            .child(results),
                    )
                    .child(
                        self.content_scroll
                            .scrollbar("library-scrollbar", px(0.), cx),
                    ),
            )
            .into_any_element()
    }

    /// A notice under the header while the active preset holds no skills.
    fn empty_scope_note(
        &self,
        library: &LibrarySnapshot,
        cx: &Context<Self>,
    ) -> Option<AnyElement> {
        let active = self.active_preset_id()?;
        let holds = library
            .library
            .skills
            .iter()
            .any(|skill| skill.presets.contains_key(&active));
        if holds {
            return None;
        }
        Some(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(format!("No skills active in {}", self.active_preset_name()))
                .into_any_element(),
        )
    }

    #[expect(clippy::too_many_arguments)]
    fn library_results(
        &self,
        library: &LibrarySnapshot,
        places: &Places<'_>,
        active: &PresetId,
        visible: &[&Skill],
        typed: &str,
        width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if visible.is_empty() {
            return self.empty_results(typed, cx);
        }
        let grid = self.library_view == LibraryView::Grid;
        let columns = grid_columns(width);
        let narrow = if grid {
            tile_width(width, columns) < NARROW_TILE
        } else {
            width < NARROW_LIST
        };
        // Library shows where skills are on instead, so only the copies that need review count.
        let placed = placements(&library.copies, &[]);
        let items: Vec<SkillItem<'_>> = visible
            .iter()
            .map(|skill| SkillItem {
                skill,
                details: library.details.get(&skill.id),
                // A narrow list has no room for folder tags.
                folder: self
                    .folder_label(skill)
                    .filter(|_| grid || width >= NARROW_LIST),
                update: has_update(&self.snapshot, &skill.id),
                drifted: skill.drift.is_some(),
                placement: placed.get(&skill.id),
                end: RowEnd::Places(places.of(skill, active)),
            })
            .collect();
        if grid {
            div()
                .grid()
                .grid_cols(columns)
                .gap(px(GRID_GAP))
                .children(items.iter().map(|item| self.skill_card(item, narrow, cx)))
                .into_any_element()
        } else {
            self.skill_list(&items, narrow, cx)
        }
    }

    /// The list's width: the main column inside the page's padding.
    fn content_width(&self, window: &Window) -> f32 {
        let sidebar = if self.sidebar_collapsed {
            0.
        } else {
            self.sidebar_width
        };
        f32::from(window.viewport_size().width) - sidebar - 2. * PAGE_GUTTER
    }
}

/// As many columns of at least `GRID_MIN` as fit, at least one.
fn grid_columns(width: f32) -> u16 {
    // Truncation is intended: only whole columns fit.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let columns = ((width + GRID_GAP) / (GRID_MIN + GRID_GAP)).floor().max(1.) as u16;
    columns
}

/// One tile's width in a grid of `columns`.
fn tile_width(width: f32, columns: u16) -> f32 {
    let columns = f32::from(columns);
    (width - (columns - 1.) * GRID_GAP) / columns
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_fits_whole_columns_and_keeps_one() {
        assert_eq!(grid_columns(100.), 1);
        assert_eq!(grid_columns(369.), 1);
        assert_eq!(grid_columns(370.), 2);
        assert_eq!(grid_columns(916.), 4);
        assert_eq!(tile_width(916., 4), 221.5);
    }
}
