//! The Library page: every library skill, or one folder's, as a searchable list or grid.

use super::controls::app_icon;
use super::library_dates as dates;
use super::library_items::SkillItem;
use super::library_presence::presence;
use super::{LibraryFilter, NativeView};
use gpui_kit::component::{
    ActiveTheme, Sizable, h_flex,
    input::Input,
    tab::{Tab, TabBar},
    v_flex,
};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div, px};
use skillbinder_proto::{LibrarySnapshot, LibraryView, Skill};

/// Narrowest grid card.
const CARD_MIN: f32 = 260.;
/// Space between grid cards and between the grid and the detail panel.
const GAP: f32 = 12.;
/// The page's horizontal padding on both sides.
const PAGE_PADDING: f32 = 48.;

impl NativeView {
    pub(super) fn render_library(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(library) = &self.snapshot.library else {
            return div()
                .child("Finish onboarding to open your library.")
                .into_any_element();
        };
        let typed = self.search.read(cx).value().trim().to_owned();
        let query = typed.to_lowercase();
        let mut visible: Vec<&Skill> = library
            .library
            .skills
            .iter()
            .filter(|skill| {
                in_filter(skill, self.library_filter()) && matches(skill, library, &query)
            })
            .collect();
        visible.sort_by_cached_key(|skill| name_order(skill));
        v_flex()
            .gap_4()
            .child(self.library_header(visible.len(), cx))
            .child(self.library_results(library, &visible, &typed, window, cx))
            .children(self.render_shared_names(library, cx))
            .into_any_element()
    }

    /// Search and the skill count, then the list/grid control; the title is in the breadcrumb.
    fn library_header(&self, count: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.library_view;
        let weak = cx.entity().downgrade();
        h_flex()
            .gap_3()
            .child(
                div().w(px(260.)).child(
                    Input::new(&self.search)
                        .prefix(app_icon("search").small())
                        .cleanable(true),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("{count} skills")),
            )
            .child(
                TabBar::new("library-view")
                    .segmented()
                    .selected_index(match view {
                        LibraryView::List => 0,
                        LibraryView::Grid => 1,
                    })
                    .child(view_tab("list-view", "List"))
                    .child(view_tab("grid-view", "Grid"))
                    .on_click(move |index, _, cx| {
                        let _ = weak.update(cx, |view, cx| {
                            view.library_view = view_at(*index);
                            view.persist_preferences(cx);
                            cx.notify();
                        });
                    }),
            )
    }

    fn library_results(
        &self,
        library: &LibrarySnapshot,
        visible: &[&Skill],
        query: &str,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if visible.is_empty() && !query.is_empty() {
            return div()
                .py_8()
                .text_center()
                .text_color(cx.theme().muted_foreground)
                .child(format!("No skills match '{query}'"))
                .into_any_element();
        }
        let found = self
            .snapshot
            .scan
            .as_ref()
            .map(|scan| presence(scan, &self.snapshot.agents, &self.snapshot.home));
        let items: Vec<SkillItem<'_>> = visible
            .iter()
            .map(|skill| SkillItem {
                skill,
                description: library
                    .details
                    .get(&skill.id)
                    .map_or("", |details| details.description.as_str()),
                imported: skill.imported_at.as_deref().and_then(dates::parse),
                modified: library
                    .modified
                    .get(&skill.id)
                    .copied()
                    .and_then(dates::from_unix),
                holders: found
                    .as_ref()
                    .map(|found| found.get(&skill.id).map_or(&[][..], Vec::as_slice)),
            })
            .collect();
        match self.library_view {
            LibraryView::List => v_flex()
                .gap_2()
                .children(items.iter().map(|item| self.skill_row(item, cx)))
                .into_any_element(),
            LibraryView::Grid => div()
                .grid()
                .grid_cols(grid_columns(self.content_width(window)))
                .gap(px(GAP))
                .children(items.iter().map(|item| self.skill_card(item, cx)))
                .into_any_element(),
        }
    }

    /// The main column's width inside its padding.
    fn content_width(&self, window: &Window) -> f32 {
        let sidebar = if self.sidebar_collapsed {
            0.
        } else {
            self.sidebar_width
        };
        f32::from(window.viewport_size().width) - sidebar - PAGE_PADDING
    }
}

fn in_filter(skill: &Skill, filter: &LibraryFilter) -> bool {
    match filter {
        LibraryFilter::All => true,
        LibraryFilter::Folder(id) => skill.folder_id.as_ref() == Some(id),
    }
}

/// Alphabetical by shown name ignoring case, then by slug so equal names keep a stable order.
fn name_order(skill: &Skill) -> (String, String) {
    let name = skill.display_name.as_deref().unwrap_or(&skill.slug);
    (name.to_lowercase(), skill.slug.clone())
}

/// Search matches the slug, display name, or description, ignoring case.
fn matches(skill: &Skill, library: &LibrarySnapshot, query: &str) -> bool {
    query.is_empty()
        || skill.slug.to_lowercase().contains(query)
        || skill
            .display_name
            .as_ref()
            .is_some_and(|name| name.to_lowercase().contains(query))
        || library
            .details
            .get(&skill.id)
            .is_some_and(|details| details.description.to_lowercase().contains(query))
}

/// One segment of the list/grid control.
fn view_tab(icon: &str, label: &'static str) -> Tab {
    Tab::new()
        .icon(app_icon(icon))
        .aria_label(label)
        .cursor_pointer()
}

/// The view behind a segment of the list/grid control, in display order.
fn view_at(index: usize) -> LibraryView {
    if index == 1 {
        LibraryView::Grid
    } else {
        LibraryView::List
    }
}

/// As many columns of at least `CARD_MIN` as fit.
fn grid_columns(width: f32) -> u16 {
    // Truncation is intended: only whole columns fit.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let columns = ((width + GAP) / (CARD_MIN + GAP)).floor().max(1.) as u16;
    columns
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(slug: &str, display_name: Option<&str>) -> Skill {
        Skill {
            id: skillbinder_proto::SkillId::new(),
            slug: slug.to_owned(),
            display_name: display_name.map(str::to_owned),
            folder_id: None,
            tag_ids: Vec::new(),
            upstream_bindings: Vec::new(),
            digest: String::new(),
            file_count: 0,
            total_bytes: 0,
            imported_at: None,
        }
    }

    #[test]
    fn skills_sort_by_shown_name_ignoring_case() {
        let skills = [
            skill("pdf", None),
            skill("zeta", Some("Alpha")),
            skill("Docx", None),
            skill("beta", None),
        ];
        let mut sorted: Vec<&Skill> = skills.iter().collect();
        sorted.sort_by_cached_key(|skill| name_order(skill));
        let slugs: Vec<_> = sorted.iter().map(|skill| skill.slug.as_str()).collect();
        assert_eq!(slugs, ["zeta", "beta", "Docx", "pdf"]);
    }

    #[test]
    fn segments_are_list_then_grid() {
        assert_eq!(view_at(0), LibraryView::List);
        assert_eq!(view_at(1), LibraryView::Grid);
    }

    #[test]
    fn grid_fits_whole_columns_and_keeps_one() {
        assert_eq!(grid_columns(100.), 1);
        assert_eq!(grid_columns(531.), 1);
        assert_eq!(grid_columns(532.), 2);
        assert_eq!(grid_columns(804.), 3);
    }
}
