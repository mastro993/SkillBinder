//! The picker's library pane: the search, the library folder filter, and a bordered list of the
//! library skills it matches, each picked or dropped with its checkbox or a click on its row. A
//! skill the place holds already is ticked, disabled, and muted.

use super::super::NativeView;
use super::super::library_items::update_glyph;
use super::super::snapshot_query::{has_update, searched};
use super::SkillPicker;
use gpui_kit::component::{ActiveTheme, h_flex, scroll::ScrollableElement, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::text;
use skillbinder_design::{
    Button, Checkbox, EmptyState, SearchField, SegmentedFilter, Text, tooltip,
};
use skillbinder_proto::{FolderId, Skill, SkillId};
use std::collections::BTreeSet;

/// How faded a skill the place holds already is.
const HERE: f32 = 0.5;
/// The search field's width.
const SEARCH_WIDTH: f32 = 280.;

impl NativeView {
    /// The library pane; `here` holds the skills the place holds already.
    pub(super) fn picker_list(
        &self,
        picker: &SkillPicker,
        here: &BTreeSet<SkillId>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let search = picker.search.read(cx).value().trim().to_lowercase();
        let skills = searched(&self.snapshot, picker.folder.as_ref(), &search);
        let field = SearchField::new("picker-search", &picker.search).width(SEARCH_WIDTH);
        let filter = (!self.folders().is_empty()).then(|| self.folder_filter(picker, &search, cx));
        let rows: Vec<AnyElement> = skills
            .iter()
            .enumerate()
            .map(|(index, skill)| self.picker_row(picker, skill, here, index == 0, cx))
            .collect();
        let list = if rows.is_empty() {
            self.picker_empty(cx)
        } else {
            v_flex().children(rows).into_any_element()
        };
        v_flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .gap(px(12.))
            .child(
                h_flex()
                    .gap(px(8.))
                    .flex_wrap()
                    .items_center()
                    .child(field)
                    .children(filter),
            )
            .child(
                div()
                    .id("picker-list")
                    .flex_1()
                    .min_h_0()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(px(10.))
                    .child(list)
                    .overflow_y_scrollbar(),
            )
            .into_any_element()
    }

    /// The folder filter: All, then each library folder, each with how many skills it holds that
    /// the search matches.
    fn folder_filter(
        &self,
        picker: &SkillPicker,
        search: &str,
        cx: &mut Context<Self>,
    ) -> SegmentedFilter {
        let folders: Vec<Option<FolderId>> = std::iter::once(None)
            .chain(self.folders().iter().map(|folder| Some(folder.id.clone())))
            .collect();
        let items = std::iter::once("All".to_owned())
            .chain(self.folders().iter().map(|folder| folder.name.clone()))
            .zip(&folders)
            .map(|(name, folder)| {
                (
                    name,
                    searched(&self.snapshot, folder.as_ref(), search).len(),
                )
            })
            .collect::<Vec<_>>();
        let chosen = folders
            .iter()
            .position(|folder| *folder == picker.folder)
            .unwrap_or_default();
        let select = cx.listener(move |view, index: &usize, _, cx| {
            let Some(folder) = folders.get(*index).cloned() else {
                return;
            };
            view.edit_picker(|picker, _| picker.folder = folder, cx);
        });
        SegmentedFilter::new("picker-folders", items)
            .selected(chosen)
            .on_select(move |index, window, cx| select(&index, window, cx))
    }

    /// One library skill: its checkbox, its slug with the update glyph, and its description.
    fn picker_row(
        &self,
        picker: &SkillPicker,
        skill: &Skill,
        here: &BTreeSet<SkillId>,
        first: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let slug = skill.id.to_string();
        let held = here.contains(&skill.id);
        let picked = picker.selection.contains(&skill.id);
        let description = self
            .snapshot
            .library
            .as_ref()
            .and_then(|library| library.details.get(&skill.id))
            .map_or("", |details| details.description.as_str())
            .to_owned();
        let theme = cx.theme();
        let pick = |id: SkillId| {
            move |view: &mut Self, cx: &mut Context<Self>| {
                let id = id.clone();
                view.edit_picker(|picker, uses| picker.selection.pick(id, uses), cx);
            }
        };
        let weak = cx.entity().downgrade();
        let toggle = pick(skill.id.clone());
        let check = Checkbox::new(format!("picker-check-{slug}"))
            .checked(held || picked)
            .inactive(held)
            .on_change(move |_, _, cx| {
                let _ = weak.update(cx, |view, cx| toggle(view, cx));
            });
        let name = h_flex()
            .gap(px(6.))
            .min_w_0()
            .child(
                Text::labeled("slug", slug.clone())
                    .level(text::TITLE_SM)
                    .truncate(),
            )
            .when(has_update(&self.snapshot, &skill.id), |name| {
                name.child(update_glyph(&skill.id))
            });
        let row = h_flex()
            .id(format!("picker-skill-{slug}"))
            .items_start()
            .gap(px(10.))
            .px(px(12.))
            .py(px(10.))
            .when(!first, |row| row.border_t_1().border_color(theme.border))
            .when(picked, |row| row.bg(theme.muted))
            .child(
                div()
                    .id(format!("picker-check-box-{slug}"))
                    .pt(px(2.))
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(check),
            )
            .child(
                v_flex().min_w_0().flex_1().gap(px(2.)).child(name).child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .truncate()
                        .child(description),
                ),
            );
        if held {
            return row
                .opacity(HERE)
                .tooltip(tooltip("Already here"))
                .into_any_element();
        }
        let hover = theme.muted;
        let toggle = pick(skill.id.clone());
        row.cursor_pointer()
            .hover(move |style| style.bg(hover))
            .on_click(cx.listener(move |view, _, _, cx| toggle(view, cx)))
            .into_any_element()
    }

    /// Why the list is empty: the library holds no skill, or nothing matches, with Clear search.
    fn picker_empty(&self, cx: &mut Context<Self>) -> AnyElement {
        let library_empty = self
            .snapshot
            .library
            .as_ref()
            .is_none_or(|library| library.library.skills.is_empty());
        let message = if library_empty {
            "No skills in your library"
        } else {
            "No skills match"
        };
        EmptyState::new("picker-empty", message)
            .when(!library_empty, |empty| {
                empty.action(
                    Button::new("picker-clear-search")
                        .label("Clear search")
                        .small()
                        .on_click(cx.listener(|view, _, window, cx| {
                            if let super::super::DialogState::AddSkill(picker) = &mut view.dialog {
                                picker.folder = None;
                                picker
                                    .search
                                    .update(cx, |input, cx| input.set_value("", window, cx));
                                cx.notify();
                            }
                        })),
                )
            })
            .into_any_element()
    }
}
