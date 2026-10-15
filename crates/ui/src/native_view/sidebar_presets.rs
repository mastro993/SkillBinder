//! The sidebar Presets section: user-ordered presets, inline naming, row menus,
//! colors, icons, and the reorder menu actions.

use std::{cell::Cell, collections::BTreeMap, time::Instant};

use super::controls::app_icon;
use super::menu::MenuItem;
use super::preset_colors::{color_menu, preset_tint};
use super::preset_icons::{IconGrid, icon_glyph, menu_glyph};
use super::sidebar_reorder::{RowDrag, RowDragPreview};
use super::sidebar_row::{activate, add_button, count, icon_color, name_field, row, with_menu};
use super::{Effect, LibraryFilter, NativeView, Route};
use gpui_kit::component::{
    ActiveTheme,
    input::{InputEvent, InputState},
};
use gpui_kit::{
    AnyElement, App, AppContext, Context, DragMoveEvent, Entity, InteractiveElement, IntoElement,
    MouseButton, ParentElement, SharedString, StatefulInteractiveElement, Styled, WeakEntity,
    Window, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::{
    LibrarySnapshot, OrganizationChange, Preset, PresetIcon, PresetId, SidebarSection,
};
use skillbinder_theme::sidebar::{INSET, ROW_GAP, SECTION_GAP};

/// Which preset name the inline sidebar field edits.
#[derive(Clone, PartialEq, Eq)]
pub(super) enum PresetEdit {
    Create,
    Rename(PresetId),
}

/// How a reorderable row is drawn this frame.
pub(super) struct RowPlacement {
    pub(super) index: usize,
    pub(super) len: usize,
    /// Vertical distance from its slot while it slides.
    pub(super) offset: f32,
    /// The row is lifted under the pointer, so its slot stays empty.
    pub(super) floating: bool,
}

impl NativeView {
    pub(super) fn render_presets(
        &self,
        library: &LibrarySnapshot,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mut counts = BTreeMap::<&PresetId, usize>::new();
        for preset in library
            .library
            .skills
            .iter()
            .flat_map(|skill| &skill.preset_ids)
        {
            *counts.entry(preset).or_default() += 1;
        }
        let (order, floating) = self
            .preset_reorder
            .displayed(&self.preset_ids(), cx.has_active_drag());
        let now = Instant::now();
        if self.preset_reorder.moving(now) {
            window.request_animation_frame();
        }
        let selected = |candidate: LibraryFilter| self.route == Route::Library(candidate);
        let presets = &library.library.presets;
        let rows: Vec<AnyElement> = order
            .iter()
            .filter_map(|id| presets.iter().find(|preset| &preset.id == id))
            .enumerate()
            .map(|(index, preset)| {
                if self.preset_edit == Some(PresetEdit::Rename(preset.id.clone())) {
                    return self.render_preset_name_field(Some(preset), cx);
                }
                let skills = counts.get(&preset.id).copied().unwrap_or(0);
                let selected = selected(LibraryFilter::Preset(preset.id.clone()));
                let placement = RowPlacement {
                    index,
                    len: order.len(),
                    offset: self.preset_reorder.offset(&preset.id, now),
                    floating: floating.as_ref() == Some(&preset.id),
                };
                self.render_preset_row(preset, placement, skills, selected, cx)
            })
            .collect();
        let creating = self.preset_edit == Some(PresetEdit::Create);
        let items = rows.len() + usize::from(creating);
        div()
            .flex()
            .flex_col()
            .child(self.render_presets_header(cx))
            .children(
                self.section_rows(
                    SidebarSection::Presets,
                    items,
                    div()
                        .id("sidebar-presets")
                        .flex()
                        .flex_col()
                        .gap(px(ROW_GAP))
                        .child(
                            // Drag moves are measured from the first preset slot; releasing the
                            // pointer anywhere keeps the order shown.
                            div()
                                .id("sidebar-preset-rows")
                                .flex()
                                .flex_col()
                                .gap(px(ROW_GAP))
                                .on_drag_move(cx.listener(
                                    |view, event: &DragMoveEvent<RowDrag<PresetId>>, window, cx| {
                                        let drag = event.drag(cx);
                                        let top = drag.top(event.event.position, event.bounds);
                                        view.drag_preset(drag.id.clone(), top, window, cx)
                                    },
                                ))
                                .on_mouse_up(
                                    MouseButton::Left,
                                    cx.listener(|view, _, _, cx| view.finish_preset_drag(cx)),
                                )
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    cx.listener(|view, _, _, cx| view.finish_preset_drag(cx)),
                                )
                                .children(rows),
                        )
                        .when(creating, |list| {
                            list.child(self.render_preset_name_field(None, cx))
                        }),
                ),
            )
    }

    fn render_presets_header(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let add = add_button("preset-create", "Add preset", cx).on_click(cx.listener(
            |view, _, window, cx| {
                view.unfold_section(SidebarSection::Presets, cx);
                view.begin_preset_edit(PresetEdit::Create, window, cx)
            },
        ));
        // The first section splits its gap with the divider above it, which sits under Library.
        self.section_header(SidebarSection::Presets, "Presets", Some(add), cx)
            .mt(px(SECTION_GAP / 2.0))
    }

    fn render_preset_row(
        &self,
        preset: &Preset,
        placement: RowPlacement,
        skills: usize,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let dark = cx.theme().mode.is_dark();
        let id = preset.id.clone();
        let label: SharedString = preset.name.clone().into();
        let route = Route::Library(LibraryFilter::Preset(id.clone()));
        let tint = preset.color.map(|color| preset_tint(color, dark));
        let glyph = icon_glyph(preset.icon);
        let (menu_preset, index, len) = (preset.clone(), placement.index, placement.len);
        let row = activate(
            row(
                format!("preset-{id}"),
                app_icon(glyph),
                tint,
                label.clone(),
                selected,
                cx,
            ),
            label.clone(),
            cx,
            move |view, window, cx| view.navigate(route.clone(), window, cx),
        );
        with_menu(
            row,
            format!("preset-{id}"),
            selected,
            self,
            cx,
            move |_, window, cx| preset_menu(&menu_preset, index, len, window, cx),
        )
        .child(count(skills, cx))
        .relative()
        .top(px(placement.offset))
        .when(placement.floating, |row| row.opacity(0.0))
        .on_drag(
            RowDrag {
                id,
                icon: glyph,
                label,
                tint,
                count: skills,
                width: self.sidebar_width - 2.0 * INSET,
                grab: Cell::new(0.0),
            },
            |drag, offset, _, cx| {
                drag.grab.set(f32::from(offset.y));
                cx.new(|_| RowDragPreview::new(drag))
            },
        )
        .into_any_element()
    }

    /// The inline name field shown in place of a preset row, or below the list when creating.
    fn render_preset_name_field(
        &self,
        preset: Option<&Preset>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let tint = preset
            .and_then(|preset| preset.color)
            .map(|color| preset_tint(color, theme.mode.is_dark()));
        let glyph = icon_glyph(preset.map_or(PresetIcon::Folder, |preset| preset.icon));
        let icon = app_icon(glyph).text_color(tint.unwrap_or(theme.muted_foreground));
        name_field(
            "preset-name-field",
            icon,
            &self.preset_name,
            cx,
            Self::end_preset_edit,
        )
    }

    pub(super) fn preset_name_event(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { .. } => self.submit_preset_edit(window, cx),
            InputEvent::Blur => self.end_preset_edit(window, cx),
            InputEvent::Change | InputEvent::Focus => {}
        }
    }

    fn begin_preset_edit(&mut self, edit: PresetEdit, window: &mut Window, cx: &mut Context<Self>) {
        let name = match &edit {
            PresetEdit::Create => String::new(),
            PresetEdit::Rename(id) => self
                .snapshot
                .library
                .iter()
                .flat_map(|library| &library.library.presets)
                .find(|preset| &preset.id == id)
                .map(|preset| preset.name.clone())
                .unwrap_or_default(),
        };
        self.preset_edit = Some(edit);
        self.preset_name.update(cx, |input, cx| {
            input.set_value(name, window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    fn submit_preset_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = Self::input_text(&self.preset_name, cx);
        let Some(edit) = self.preset_edit.clone() else {
            return;
        };
        self.end_preset_edit(window, cx);
        let change = match edit {
            _ if name.is_empty() => return,
            PresetEdit::Create => OrganizationChange::CreatePreset { name },
            PresetEdit::Rename(id) => OrganizationChange::RenamePreset { id, name },
        };
        self.change_organization(change, cx);
    }

    /// Closes the inline field and returns keyboard focus to the window's shortcuts.
    pub(super) fn end_preset_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.preset_edit.take().is_some() {
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    pub(super) fn change_organization(
        &mut self,
        change: OrganizationChange,
        cx: &mut Context<Self>,
    ) {
        let client = self.client.clone();
        self.run(
            async move {
                client
                    .change_organization(change)
                    .await
                    .map(|_| Effect::None)
            },
            cx,
        );
    }
}

/// A menu handler that runs `action` on the view for one row.
pub(super) fn menu_action<Id: 'static>(
    view: WeakEntity<NativeView>,
    id: Id,
    action: impl Fn(&mut NativeView, &Id, &mut Window, &mut Context<NativeView>) + 'static,
) -> impl Fn(&mut Window, &mut App) + 'static {
    move |window, cx| {
        let _ = view.update(cx, |view, cx| action(view, &id, window, cx));
    }
}

/// Rename, Color, Icon, order, and delete actions for the preset at `index` of `len`.
fn preset_menu(
    preset: &Preset,
    index: usize,
    len: usize,
    window: &mut Window,
    cx: &mut Context<NativeView>,
) -> Vec<MenuItem> {
    let view = cx.entity().downgrade();
    let (id, color, icon) = (preset.id.clone(), preset.color, preset.icon);
    let on = |action: fn(&mut NativeView, &PresetId, &mut Window, &mut Context<NativeView>)| {
        menu_action(view.clone(), id.clone(), action)
    };
    let picked = (view.clone(), id.clone());
    let grid = cx.new(|cx| {
        IconGrid::new(icon, color, cx, move |icon, window, cx| {
            let (view, id) = &picked;
            let _ = view.update(cx, |view, cx| {
                let id = id.clone();
                view.change_organization(OrganizationChange::SetPresetIcon { id, icon }, cx);
                if let Some(menu) = view.menu.as_ref() {
                    menu.close(window, cx);
                }
            });
        })
    });
    let focus = grid.read(cx).focus().clone();
    vec![
        MenuItem::new("rename", "Rename")
            .icon("pencil-edit")
            .on_select(on(|view, id, window, cx| {
                view.begin_preset_edit(PresetEdit::Rename(id.clone()), window, cx)
            })),
        MenuItem::new("color", "Color")
            .icon("paint-board")
            .submenu(color_menu(&view, &id, color, window, cx)),
        MenuItem::new("icon", "Icon")
            .leading(move |cx| menu_glyph(icon, icon_color(cx)))
            .panel(focus, move |_, _| grid.clone().into_any_element()),
        MenuItem::separator(),
        MenuItem::new("move-up", "Move Up")
            .icon("arrow-up")
            .disabled(index == 0)
            .on_select(menu_action(
                view.clone(),
                id.clone(),
                move |view, id, _, cx| view.move_preset(id, index.saturating_sub(1), cx),
            )),
        MenuItem::new("move-down", "Move Down")
            .icon("arrow-down")
            .disabled(index + 1 >= len)
            .on_select(menu_action(
                view.clone(),
                id.clone(),
                move |view, id, _, cx| view.move_preset(id, index + 1, cx),
            )),
        MenuItem::separator(),
        MenuItem::new("delete", "Delete…")
            .icon("delete")
            .danger()
            .on_select(on(|view, id, _, cx| {
                let (client, id) = (view.client.clone(), id.clone());
                view.run(
                    async move { client.preview_delete(id).await.map(Effect::DeletePreview) },
                    cx,
                );
            })),
    ]
}
