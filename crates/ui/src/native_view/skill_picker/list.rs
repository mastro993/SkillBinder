//! The picker's library pane: the search, the library folder chips, and a bordered list of the
//! library skills it matches, each picked or dropped with its checkbox or a click on its row. A
//! skill the place holds already is ticked, disabled, and muted.

use super::super::NativeView;
use super::super::controls::{Inactive, app_icon, button, checkbox, text_button, tooltip};
use super::super::library_items::update_glyph;
use super::super::snapshot_query::{has_update, searched};
use super::SkillPicker;
use gpui_kit::component::{
    ActiveTheme, Sizable, Size,
    button::{Button, ButtonVariants},
    h_flex,
    input::Input,
    scroll::ScrollableElement,
    v_flex,
};
use gpui_kit::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Text, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::{FolderId, Skill, SkillId};
use std::collections::BTreeSet;

/// How faded a skill the place holds already is.
const HERE: f32 = 0.5;

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
        let muted = cx.theme().muted_foreground;
        let field = div().id("picker-search").w(px(280.)).flex_none().child(
            Input::new(&picker.search)
                .prefix(app_icon("search").size(px(15.)).text_color(muted))
                .bg(cx.theme().background),
        );
        let mut chips = Vec::new();
        if !self.folders().is_empty() {
            let count = |folder: Option<&FolderId>| searched(&self.snapshot, folder, &search).len();
            chips.push(self.folder_chip(picker, None, "All", count(None), cx));
            for folder in self.folders() {
                let id = Some(folder.id.clone());
                let chip = self.folder_chip(picker, id, &folder.name, count(Some(&folder.id)), cx);
                chips.push(chip);
            }
        }
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
                    .children(chips),
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

    /// A folder chip, `All` with no `id`: its name and how many skills it holds that the search
    /// matches. The chosen chip is filled.
    fn folder_chip(
        &self,
        picker: &SkillPicker,
        id: Option<FolderId>,
        name: &str,
        count: usize,
        cx: &mut Context<Self>,
    ) -> Button {
        let element = id.as_ref().map_or_else(
            || "picker-folder-all".to_owned(),
            |id| format!("picker-folder-{id}"),
        );
        let chosen = picker.folder == id;
        let tint = if chosen {
            cx.theme().primary_foreground.opacity(0.7)
        } else {
            cx.theme().muted_foreground
        };
        button(element)
            .small()
            .rounded_full()
            .accessibility_label(name.to_owned())
            .map(|chip| {
                if chosen {
                    chip.primary()
                } else {
                    chip.outline()
                }
            })
            .child(
                h_flex()
                    .gap(px(5.))
                    .text_size(px(12.5))
                    .child(div().font_weight(FontWeight::MEDIUM).child(name.to_owned()))
                    .child(div().text_color(tint).child(count.to_string())),
            )
            .on_click(cx.listener(move |view, _, _, cx| {
                let id = id.clone();
                view.edit_picker(|picker, _| picker.folder = id, cx);
            }))
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
        let check = checkbox(format!("picker-check-{slug}"))
            .checked(held || picked)
            .inactive(held)
            .on_change(move |_, _, cx| {
                let _ = weak.update(cx, |view, cx| toggle(view, cx));
            });
        let name = h_flex()
            .gap(px(6.))
            .min_w_0()
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .truncate()
                    .child(Text::new("slug".into(), slug.clone().into())),
            )
            .when(has_update(&self.snapshot, &skill.id), |name| {
                name.child(update_glyph(&skill.id, theme.info))
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
        v_flex()
            .id("picker-empty")
            .items_center()
            .gap(px(10.))
            .py(px(36.))
            .text_color(cx.theme().muted_foreground)
            .child(message)
            .when(!library_empty, |empty| {
                empty.child(
                    text_button("picker-clear-search", "Clear search", Size::Small)
                        .outline()
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
