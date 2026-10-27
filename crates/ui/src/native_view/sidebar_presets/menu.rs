//! Context menu entries for the sidebar's preset rows.

use super::super::menu::MenuItem;
use super::super::preset_colors::color_menu;
use super::super::preset_icons::{IconGrid, menu_glyph};
use super::super::sidebar_row::icon_color;
use super::super::{Effect, NativeView};
use super::PresetEdit;
use gpui_kit::{App, AppContext, Context, IntoElement, WeakEntity, Window};
use skillbinder_proto::{OrganizationChange, Preset, PresetColor, PresetIcon, PresetId};

/// A menu handler that runs `action` on the view for one row.
pub(in crate::native_view) fn menu_action<Id: 'static>(
    view: WeakEntity<NativeView>,
    id: Id,
    action: impl Fn(&mut NativeView, &Id, &mut Window, &mut Context<NativeView>) + 'static,
) -> impl Fn(&mut Window, &mut App) + 'static {
    move |window, cx| {
        let _ = view.update(cx, |view, cx| action(view, &id, window, cx));
    }
}

/// Sets the color of preset `id` from its menu.
fn set_preset_color(
    view: &mut NativeView,
    id: &PresetId,
    color: Option<PresetColor>,
    cx: &mut Context<NativeView>,
) {
    let change = OrganizationChange::SetPresetColor {
        id: id.clone(),
        color,
    };
    view.change_organization(change, cx);
}

/// Rename, Clone, Color, Icon, order, and delete actions for a preset at `index` of `len`. The
/// Default preset's menu holds only Clone: the app never renames it, recolors it, re-icons it,
/// moves it, or removes it.
pub(super) fn preset_menu(
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
        IconGrid::new(
            Some(icon),
            PresetIcon::ALL.into_iter().map(Some),
            color,
            cx,
            move |choice, window, cx| {
                let Some(icon) = choice else {
                    return;
                };
                let (view, id) = &picked;
                let _ = view.update(cx, |view, cx| {
                    let id = id.clone();
                    view.change_organization(OrganizationChange::SetPresetIcon { id, icon }, cx);
                    if let Some(menu) = view.menu.as_ref() {
                        menu.close(window, cx);
                    }
                });
            },
        )
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
            .submenu(color_menu(
                &view,
                id.clone(),
                color,
                set_preset_color,
                window,
                cx,
            )),
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
