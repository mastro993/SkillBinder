//! The Library page: every library skill, or one folder's, narrowed by search and the Active and
//! Updates filters, as a list or grid under a fixed toolbar, each showing where it is on. A
//! workspace's page lists the same skills, scoped to the workspace, with its activation controls.

use super::controls::exact_text::line_shortfall;
use super::library_copies::{AgentRow, placements};
use super::library_items::{Layout, RowEnd, SLUG_SIZE, SkillItem};
use super::snapshot_query::{
    LibraryFilter, Places, Query, filter_counts, has_update, library_skills,
};
use super::workspace_targets::dir_rows;
use super::{NativeView, Route};
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px, relative,
};
use skillbinder_proto::{LibrarySnapshot, LibraryView, PresetId, Skill};
use skillbinder_theme::library::{
    GRID_GAP, GRID_MIN, NARROW_LIST, NARROW_TILE, PAGE_BOTTOM, PAGE_GAP, PAGE_GUTTER, PAGE_TOP,
    ROW_PADDING_Y, TILE_PADDING,
};

/// A workspace's page keeps the shell's 24-point gutter on both sides.
const WORKSPACE_PADDING: f32 = 48.;

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
        let on_library = self.route == Route::Library;
        let places = Places::new(&self.snapshot);
        let query = Query {
            folder: self.shown_folder().map(|folder| &folder.id),
            search: &search,
            // A workspace's page has no filter row.
            filter: if on_library {
                self.library_filter
            } else {
                LibraryFilter::default()
            },
            keep: self.place_popover.as_ref().map(|popover| &popover.skill),
        };
        let shown = library_skills(&self.snapshot, &places, &active, query);
        let counts = on_library.then(|| filter_counts(&self.snapshot, &places, &active, query));
        let frame = Frame {
            width: self.content_width(window),
            scale: window.scale_factor(),
        };
        let toolbar = self.library_toolbar(counts, frame.width, window, cx);
        let results = self.library_results(library, &places, &active, &shown, &typed, frame, cx);
        let notes: Vec<AnyElement> = self
            .empty_scope_note(library, cx)
            .into_iter()
            .chain(self.render_update_outcome(cx))
            .collect();
        if !on_library {
            return v_flex()
                .gap_4()
                .child(toolbar)
                .children(notes)
                .child(results)
                .into_any_element();
        }
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

    /// A notice under the header while the open workspace, or else the active preset, holds no
    /// skills.
    fn empty_scope_note(
        &self,
        library: &LibrarySnapshot,
        cx: &Context<Self>,
    ) -> Option<AnyElement> {
        let holds = match self.scoped_workspace() {
            Some(workspace) => !workspace.skills.is_empty(),
            None => {
                let active = self.active_preset_id()?;
                library
                    .library
                    .skills
                    .iter()
                    .any(|skill| skill.presets.contains_key(&active))
            }
        };
        if holds {
            return None;
        }
        Some(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(format!("No skills active in {}", self.scope_name()))
                .into_any_element(),
        )
    }

    /// The rows the agents line names on a workspace's page: the workspace's skill directories.
    /// Library shows where skills are on instead, so only the copies that need review count there.
    fn placement_rows(&self) -> Vec<AgentRow> {
        self.scoped_workspace().map_or_else(Vec::new, |workspace| {
            dir_rows(&self.snapshot.agents, workspace, &self.snapshot.home)
                .into_iter()
                .map(|row| row.row)
                .collect()
        })
    }

    #[expect(clippy::too_many_arguments)]
    fn library_results(
        &self,
        library: &LibrarySnapshot,
        places: &Places<'_>,
        active: &PresetId,
        visible: &[&Skill],
        typed: &str,
        frame: Frame,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if visible.is_empty() {
            return self.empty_results(typed, cx);
        }
        let on_library = self.route == Route::Library;
        let grid = self.library_view == LibraryView::Grid;
        let width = frame.width;
        let columns = grid_columns(width);
        let layout = frame.layout(grid, columns);
        let placed = placements(&library.copies, &self.placement_rows());
        let overlaps = self.scoped_workspace().map(|workspace| &workspace.overlaps);
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
                overlap: overlaps
                    .and_then(|overlaps| overlaps.get(&skill.id))
                    .copied(),
                placement: placed.get(&skill.id),
                end: if on_library {
                    RowEnd::Places(places.of(skill, active))
                } else {
                    RowEnd::Activation
                },
            })
            .collect();
        if grid {
            div()
                .grid()
                .grid_cols(columns)
                .gap(px(GRID_GAP))
                .children(items.iter().map(|item| self.skill_card(item, layout, cx)))
                .into_any_element()
        } else {
            self.skill_list(&items, layout, cx)
        }
    }

    /// The list's width: the main column inside the page's padding.
    fn content_width(&self, window: &Window) -> f32 {
        let sidebar = if self.sidebar_collapsed {
            0.
        } else {
            self.sidebar_width
        };
        let padding = if self.route == Route::Library {
            2. * PAGE_GUTTER
        } else {
            WORKSPACE_PADDING
        };
        f32::from(window.viewport_size().width) - sidebar - padding
    }
}

/// Chrome rounds a box edge that falls on half a device pixel up and GPUI rounds it down, so the
/// ninth row's top, 1143.5 device pixels at 2x, would land a pixel higher than the prototype's.
/// Padding each row and tile by this much more breaks such ties upward and moves no other edge.
const TIE: f32 = 1. / 8192.;

/// The list's width and the window's scale.
#[derive(Clone, Copy)]
struct Frame {
    width: f32,
    scale: f32,
}

impl Frame {
    /// A list's rows, or a grid of `columns` tiles. A row's padding resolves against the card
    /// inside its 1-point border, and a tile's name line against the tile inside its border and
    /// padding.
    fn layout(self, grid: bool, columns: u16) -> Layout {
        let half = line_shortfall(SLUG_SIZE, self.scale) / 2. + TIE;
        if grid {
            let tile = tile_width(self.width, columns);
            Layout {
                narrow: tile < NARROW_TILE,
                // Padding cannot be negative; at 1x GPUI's line is the taller one.
                pad_y: relative(half.max(0.) / (tile - 2. * (TILE_PADDING + 1.))),
            }
        } else {
            Layout {
                narrow: self.width < NARROW_LIST,
                pad_y: relative((ROW_PADDING_Y + half) / (self.width - 2.)),
            }
        }
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

    #[test]
    fn rows_and_tiles_pad_by_half_the_name_lines_shortfall() {
        let frame = Frame {
            width: 916.,
            scale: 2.,
        };
        let half = 0.09375 / 2. + TIE;
        assert_eq!(
            frame.layout(false, 4).pad_y,
            relative((ROW_PADDING_Y + half) / 914.)
        );
        assert_eq!(frame.layout(true, 4).pad_y, relative(half / 195.5));
    }
}
