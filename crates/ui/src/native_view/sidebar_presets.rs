//! The sidebar Presets section: user-ordered presets, inline naming, row menus,
//! colors, icons, and the reorder menu actions.

mod optimistic;

use std::{cell::Cell, collections::BTreeMap, time::Instant};

use super::controls::app_icon;
use super::menu::MenuItem;
use super::preset_colors::{color_menu, preset_tint};
use super::preset_icons::{IconGrid, icon_glyph, menu_glyph};
use super::sidebar_reorder::{RowDrag, RowDragPreview};
use super::sidebar_row::{
    activate, active_marker, add_button, count, icon_color, name_field, row, with_menu,
};
use super::{Effect, NativeView};
use gpui_kit::component::{
    ActiveTheme,
    input::{InputEvent, InputState},
};
use gpui_kit::{
    AnyElement, App, AppContext, Context, DragMoveEvent, Entity, Hsla, InteractiveElement,
    IntoElement, MouseButton, ParentElement, SharedString, StatefulInteractiveElement, Styled,
    WeakEntity, Window, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::{
    LibrarySnapshot, OrganizationChange, Preset, PresetIcon, PresetId, SidebarSection,
};
use skillbinder_theme::sidebar::{INSET, ROW_GAP, SECTION_GAP};

/// A preset shown while its creation or clone is being saved.
pub(super) struct OptimisticPreset {
    pub(super) preset: Preset,
    /// The source of a clone, so its row sits directly after it; `None` appends a new preset.
    pub(super) after: Option<PresetId>,
    pub(super) skills: usize,
}

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
        let active = self.active_preset_id();
        let presets = &library.library.presets;
        let mut rows: Vec<AnyElement> = order
            .iter()
            .filter_map(|id| presets.iter().find(|preset| &preset.id == id))
            .enumerate()
            .map(|(index, preset)| {
                if self.preset_edit == Some(PresetEdit::Rename(preset.id.clone())) {
                    return self.render_preset_name_field(Some(preset), cx);
                }
                let skills = counts.get(&preset.id).copied().unwrap_or(0);
                let active = active.as_ref() == Some(&preset.id);
                let placement = RowPlacement {
                    index,
                    len: order.len(),
                    offset: self.preset_reorder.offset(&preset.id, now),
                    floating: floating.as_ref() == Some(&preset.id),
                };
                self.render_preset_row(preset, active, placement, skills, cx)
            })
            .collect();
        for optimistic in &self.optimistic_presets {
            let index = optimistic
                .after
                .as_ref()
                .and_then(|source| order.iter().position(|id| id == source))
                .map_or(rows.len(), |index| index + 1);
            rows.insert(index, self.render_optimistic_preset_row(optimistic, cx));
        }
        let creating = self.preset_edit == Some(PresetEdit::Create);
        let items = rows.len() + usize::from(creating);
        div()
            .flex()
            .flex_col()
            .child(self.render_presets_header(items == 0, cx))
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

    fn render_presets_header(
        &self,
        empty: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let add = add_button("preset-create", "Add preset", cx).on_click(cx.listener(
            |view, _, window, cx| {
                view.unfold_section(SidebarSection::Presets, cx);
                view.begin_preset_edit(PresetEdit::Create, window, cx)
            },
        ));
        // The first section splits its gap with the divider above it, which sits under Library.
        self.section_header(SidebarSection::Presets, "Presets", Some(add), empty, cx)
            .mt(px(SECTION_GAP / 2.0))
    }

    fn render_preset_row(
        &self,
        preset: &Preset,
        active: bool,
        placement: RowPlacement,
        skills: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let dark = cx.theme().mode.is_dark();
        let id = preset.id.clone();
        let label: SharedString = self.preset_display_name(preset).to_owned().into();
        let tint = preset.color.map(|color| preset_tint(color, dark));
        let glyph = icon_glyph(preset.icon);
        let (menu_preset, index, len) = (preset.clone(), placement.index, placement.len);
        let busy = self.busy;
        let open = id.clone();
        let row = activate(
            row(
                format!("preset-{id}"),
                app_icon(glyph),
                tint,
                label.clone(),
                active,
                cx,
            ),
            label.clone(),
            cx,
            move |view, _, cx| view.set_active_preset(Some(open.clone()), cx),
        );
        let protected = preset.is_default();
        let row = with_menu(
            row,
            format!("preset-{id}"),
            active,
            self,
            cx,
            move |_, window, cx| preset_menu(&menu_preset, index, len, busy, window, cx),
        );
        let row = row
            .when(active, |row| row.bg(active_preset_fill(tint, cx)))
            .when(active, |row| {
                row.child(active_marker(id.as_str(), tint, cx))
            })
            .when(skills > 0, |row| row.child(count(skills, cx)))
            .relative()
            .top(px(placement.offset))
            .when(placement.floating, |row| row.opacity(0.0));
        if protected || self.busy {
            return row.into_any_element();
        }
        row.on_drag(
            RowDrag {
                id,
                icon: glyph,
                label,
                tint,
                count: (skills > 0).then_some(skills),
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

    /// The name shown for a preset, including one whose rename is still being saved.
    fn preset_display_name<'a>(&'a self, preset: &'a Preset) -> &'a str {
        shown_preset_name(preset, self.optimistic_preset_name.as_ref())
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
        let glyph = icon_glyph(preset.map_or_else(PresetIcon::default, |preset| preset.icon));
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
        match edit {
            _ if name.is_empty() => {}
            PresetEdit::Create => self.create_preset_optimistically(name, window, cx),
            PresetEdit::Rename(id) => self.rename_preset(id, name, window, cx),
        }
    }

    /// Renames a preset optimistically: the row shows `name` at once, and a refused rename
    /// restores the stored name and reports the reason in an error toast.
    fn rename_preset(
        &mut self,
        id: PresetId,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy {
            return;
        }
        let Some(stored) = self
            .snapshot
            .library
            .as_ref()
            .and_then(|library| {
                library
                    .library
                    .presets
                    .iter()
                    .find(|preset| preset.id == id)
            })
            .map(|preset| preset.name.clone())
        else {
            return;
        };
        if stored == name {
            return;
        }
        self.optimistic_preset_name = Some((id.clone(), name.clone()));
        self.save_optimistically(
            OrganizationChange::RenamePreset { id, name },
            "Preset rename failed",
            window,
            cx,
        );
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

/// The preset's stored name, or the optimistic name while its rename is in flight.
fn shown_preset_name<'a>(
    preset: &'a Preset,
    optimistic: Option<&'a (PresetId, String)>,
) -> &'a str {
    optimistic
        .filter(|(id, _)| id == &preset.id)
        .map_or(preset.name.as_str(), |(_, name)| name.as_str())
}

/// The active preset's row fill: its color at a low opacity, or the sidebar selection fill when
/// it has no color.
fn active_preset_fill(tint: Option<Hsla>, cx: &App) -> Hsla {
    tint.map_or_else(|| cx.theme().sidebar_accent, |color| color.opacity(0.22))
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

/// Rename, Clone, Color, Icon, order, and delete actions for a preset at `index` of `len`. The
/// Default preset's menu holds only Clone: the app never renames it, recolors it, re-icons it,
/// moves it, or removes it.
fn preset_menu(
    preset: &Preset,
    index: usize,
    len: usize,
    busy: bool,
    window: &mut Window,
    cx: &mut Context<NativeView>,
) -> Vec<MenuItem> {
    let view = cx.entity().downgrade();
    let (id, color, icon) = (preset.id.clone(), preset.color, preset.icon);
    let on = |action: fn(&mut NativeView, &PresetId, &mut Window, &mut Context<NativeView>)| {
        menu_action(view.clone(), id.clone(), action)
    };
    let clone = MenuItem::new("clone", "Clone")
        .icon("book-copy")
        .disabled(busy)
        .on_select(menu_action(
            view.clone(),
            id.clone(),
            |view, id, window, cx| {
                view.clone_preset_optimistically(id.clone(), window, cx);
            },
        ));
    if preset.is_default() {
        return vec![clone];
    }
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
            .disabled(busy)
            .on_select(on(|view, id, window, cx| {
                view.begin_preset_edit(PresetEdit::Rename(id.clone()), window, cx)
            })),
        clone,
        MenuItem::new("color", "Color")
            .icon("paint-board")
            .submenu(color_menu(&view, &id, color, window, cx)),
        MenuItem::new("icon", "Icon")
            .leading(move |cx| menu_glyph(icon, icon_color(cx)))
            .panel(focus, move |_, _| grid.clone().into_any_element()),
        MenuItem::separator(),
        // The default preset holds the first slot, so the next one cannot move up.
        MenuItem::new("move-up", "Move Up")
            .icon("arrow-up")
            .disabled(index <= 1)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(id: &str, name: &str) -> Preset {
        Preset {
            id: PresetId(id.to_owned()),
            name: name.to_owned(),
            position: 0,
            color: None,
            icon: PresetIcon::Layers,
        }
    }

    #[test]
    fn a_pending_rename_shows_only_on_its_preset() {
        let review = preset("review", "Review");
        let other = preset("other", "Other");
        let pending = Some((PresetId("review".to_owned()), "Reviewed".to_owned()));
        assert_eq!(shown_preset_name(&review, pending.as_ref()), "Reviewed");
        assert_eq!(shown_preset_name(&other, pending.as_ref()), "Other");
        assert_eq!(shown_preset_name(&review, None), "Review");
    }
}
