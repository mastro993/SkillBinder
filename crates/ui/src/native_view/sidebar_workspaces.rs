//! The sidebar Workspaces section: user-ordered workspace rows with inline renaming, row menus,
//! and drag reordering. Each row counts the skills the last scan found.

use std::{cell::Cell, rc::Rc, time::Instant};

use super::controls::app_icon;
use super::menu::MenuItem;
use super::preset_colors::{color_menu, preset_tint};
use super::preset_icons::IconGrid;
use super::sidebar_presets::{RowPlacement, menu_action};
use super::sidebar_reorder::{RowDrag, RowDragPreview, grab_cursor};
use super::sidebar_row::{activate, add_button, count, name_field, row, with_menu};
use super::workspace_icons::{
    ChooseImage, choices as workspace_icon_choices, icon_picker, menu_icon, row_icon,
};
use super::{DialogState, Effect, NativeView, Route};
use gpui_kit::component::{
    ActiveTheme,
    input::{InputEvent, InputState},
};
use gpui_kit::{
    AnyElement, App, AppContext, Context, DragMoveEvent, Entity, InteractiveElement, IntoElement,
    MouseButton, ParentElement, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use skillbinder_proto::{PresetColor, PresetIcon, SidebarSection, Workspace, WorkspaceId};
use skillbinder_theme::sidebar::{ICON_SIZE, INSET, ROW_GAP};

impl NativeView {
    pub(super) fn render_workspaces(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let now = Instant::now();
        if self.workspace_reorder.moving(now) {
            window.request_animation_frame();
        }
        let (order, floating) = self
            .workspace_reorder
            .displayed(&self.workspace_ids(), cx.has_active_drag());
        let workspaces = &self.snapshot.workspaces;
        let rows: Vec<AnyElement> = order
            .iter()
            .filter_map(|id| workspaces.iter().find(|workspace| &workspace.id == id))
            .enumerate()
            .map(|(index, workspace)| {
                if self.workspace_edit.as_ref() == Some(&workspace.id) {
                    let tint = workspace
                        .color
                        .map(|color| preset_tint(color, cx.theme().mode.is_dark()));
                    return name_field(
                        "workspace-name-field",
                        row_icon(workspace),
                        tint,
                        &self.workspace_name,
                        cx,
                        Self::end_workspace_rename,
                    );
                }
                let placement = RowPlacement {
                    index,
                    len: order.len(),
                    offset: self.workspace_reorder.offset(&workspace.id, now),
                    floating: floating.as_ref() == Some(&workspace.id),
                };
                self.render_workspace_row(workspace, placement, cx)
            })
            .collect();
        let add = add_button("workspace-add", "Add workspace", cx).on_click(cx.listener(
            |view, _, _, cx| {
                view.unfold_section(SidebarSection::Workspaces, cx);
                view.choose_workspace_folder(cx)
            },
        ));
        let items = rows.len();
        div()
            .flex()
            .flex_col()
            .child(self.section_header(
                SidebarSection::Workspaces,
                "Workspaces",
                Some(add),
                items == 0,
                cx,
            ))
            .children(
                self.section_rows(
                    SidebarSection::Workspaces,
                    items,
                    // Drag moves are measured from the first workspace slot.
                    div()
                        .id("sidebar-workspaces")
                        .flex()
                        .flex_col()
                        .gap(px(ROW_GAP))
                        .on_drag_move(cx.listener(
                            |view, event: &DragMoveEvent<RowDrag<WorkspaceId>>, window, cx| {
                                let drag = event.drag(cx);
                                let top = drag.top(event.event.position, event.bounds);
                                view.drag_workspace(drag.id.clone(), top, window, cx)
                            },
                        ))
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(|view, _, _, cx| view.finish_workspace_drag(cx)),
                        )
                        .on_mouse_up_out(
                            MouseButton::Left,
                            cx.listener(|view, _, _, cx| view.finish_workspace_drag(cx)),
                        )
                        .children(rows),
                ),
            )
    }

    fn render_workspace_row(
        &self,
        workspace: &Workspace,
        placement: RowPlacement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let route = Route::Workspace(workspace.id.clone());
        let selected = self.route == route;
        let label: SharedString = workspace.name.clone().into();
        let tint = workspace
            .color
            .map(|color| preset_tint(color, cx.theme().mode.is_dark()));
        let accessible = if workspace.missing {
            format!("{}, folder missing", workspace.name)
        } else {
            workspace.name.clone()
        };
        let found = self
            .found_where(|location| location.workspaces.contains(&workspace.id))
            .len();
        let trailing = if workspace.missing {
            app_icon("alert-circle")
                .size(px(ICON_SIZE))
                .flex_shrink_0()
                .text_color(cx.theme().warning)
                .into_any_element()
        } else {
            count(found, cx)
        };
        let (index, len) = (placement.index, placement.len);
        let row = activate(
            row(
                format!("workspace-{}", workspace.id),
                row_icon(workspace),
                tint,
                label.clone(),
                selected,
                cx,
            ),
            accessible,
            cx,
            move |view, window, cx| view.navigate(route.clone(), window, cx),
        );
        let owner = format!("workspace-{}", workspace.id);
        let menu_workspace = workspace.clone();
        with_menu(row, owner, selected, self, cx, move |_, window, cx| {
            workspace_menu(&menu_workspace, index, len, window, cx)
        })
        .child(trailing)
        .relative()
        .top(px(placement.offset))
        .when(placement.floating, |row| row.opacity(0.0))
        .on_drag(
            RowDrag {
                id: workspace.id.clone(),
                icon: row_icon(workspace),
                label,
                tint,
                count: Some(found),
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

    fn workspace_ids(&self) -> Vec<WorkspaceId> {
        self.snapshot
            .workspaces
            .iter()
            .map(|workspace| workspace.id.clone())
            .collect()
    }

    fn drag_workspace(
        &mut self,
        id: WorkspaceId,
        top: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        grab_cursor(window, cx);
        let stored = self.workspace_ids();
        if self
            .workspace_reorder
            .drag(&stored, &id, top, !cx.reduce_motion())
        {
            cx.notify();
        }
    }

    fn finish_workspace_drag(&mut self, cx: &mut Context<Self>) {
        let stored = self.workspace_ids();
        let (released, animate) = (cx.has_active_drag(), !cx.reduce_motion());
        if let Some(order) = self
            .workspace_reorder
            .finish(&stored, released, self.busy, animate)
        {
            self.save_workspace_order(order, cx);
        }
        cx.notify();
    }

    fn move_workspace(&mut self, id: &WorkspaceId, target: usize, cx: &mut Context<Self>) {
        let stored = self.workspace_ids();
        let animate = !cx.reduce_motion();
        if let Some(order) = self
            .workspace_reorder
            .move_to(&stored, id, target, self.busy, animate)
        {
            self.save_workspace_order(order, cx);
        }
        cx.notify();
    }

    fn save_workspace_order(&mut self, order: Vec<WorkspaceId>, cx: &mut Context<Self>) {
        let client = self.client.clone();
        self.run(
            async move { client.reorder_workspaces(order).await.map(|_| Effect::None) },
            cx,
        );
    }

    fn begin_workspace_rename(
        &mut self,
        id: &WorkspaceId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = self
            .snapshot
            .workspaces
            .iter()
            .find(|workspace| &workspace.id == id)
            .map(|workspace| workspace.name.clone())
            .unwrap_or_default();
        self.workspace_edit = Some(id.clone());
        self.workspace_name.update(cx, |input, cx| {
            input.set_value(name, window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    pub(super) fn workspace_name_event(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { .. } => self.submit_workspace_rename(window, cx),
            InputEvent::Blur => self.end_workspace_rename(window, cx),
            InputEvent::Change | InputEvent::Focus => {}
        }
    }

    fn submit_workspace_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = Self::input_text(&self.workspace_name, cx);
        let Some(id) = self.workspace_edit.clone() else {
            return;
        };
        self.end_workspace_rename(window, cx);
        if name.is_empty() {
            return;
        }
        let client = self.client.clone();
        self.run(
            async move {
                client
                    .rename_workspace(id, name)
                    .await
                    .map(|_| Effect::None)
            },
            cx,
        );
    }

    /// Closes the inline field and returns keyboard focus to the window's shortcuts.
    fn end_workspace_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspace_edit.take().is_some() {
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }
}

/// Rename, appearance, order, and remove actions for the workspace at `index` of `len`.
fn workspace_menu(
    workspace: &Workspace,
    index: usize,
    len: usize,
    window: &mut Window,
    cx: &mut Context<NativeView>,
) -> Vec<MenuItem> {
    let view = cx.entity().downgrade();
    let on = |action: fn(&mut NativeView, &WorkspaceId, &mut Window, &mut Context<NativeView>)| {
        menu_action(view.clone(), workspace.id.clone(), action)
    };
    let (id, color, icon) = (workspace.id.clone(), workspace.color, workspace.icon);
    let has_image = workspace.image.is_some();
    let choose: ChooseImage = {
        let view = view.clone();
        let id = id.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| {
            let _ = view.update(cx, |view, cx| {
                if let Some(menu) = view.menu.as_ref() {
                    menu.close(window, cx);
                }
                view.choose_workspace_image(id.clone(), cx);
            });
        })
    };
    let leading = workspace.clone();
    let grid_id = id.clone();
    let grid_view = view.clone();
    let grid = cx.new(|cx| {
        IconGrid::new(
            icon,
            workspace_icon_choices(),
            color,
            cx,
            move |picked, window, cx| {
                let _ = grid_view.update(cx, |view, cx| {
                    view.change_workspace_icon(grid_id.clone(), picked, cx);
                    if let Some(menu) = view.menu.as_ref() {
                        menu.close(window, cx);
                    }
                });
            },
        )
    });
    let focus = grid.read(cx).focus().clone();
    let mut items = vec![
        MenuItem::new("rename", "Rename")
            .icon("pencil-edit")
            .on_select(on(|view, id, window, cx| {
                view.begin_workspace_rename(id, window, cx)
            })),
        MenuItem::new("color", "Color")
            .icon("paint-board")
            .submenu(color_menu(
                &view,
                id.clone(),
                color,
                set_workspace_color,
                window,
                cx,
            )),
        MenuItem::new("icon", "Icon")
            .leading(move |cx| menu_icon(&leading, cx))
            .panel(focus, move |_, cx| {
                icon_picker(grid.clone(), has_image, choose.clone(), cx)
            }),
    ];
    if has_image {
        items.push(
            MenuItem::new("workspace-image-remove", "Remove Image")
                .icon("delete")
                .on_select(menu_action(
                    view.clone(),
                    (id.clone(), icon),
                    move |view, (id, icon), _, cx| {
                        view.change_workspace_icon(id.clone(), *icon, cx)
                    },
                )),
        );
    }
    items.extend([
        MenuItem::separator(),
        MenuItem::new("move-up", "Move Up")
            .icon("arrow-up")
            .disabled(index == 0)
            .on_select(menu_action(
                view.clone(),
                id.clone(),
                move |view, id, _, cx| view.move_workspace(id, index.saturating_sub(1), cx),
            )),
        MenuItem::new("move-down", "Move Down")
            .icon("arrow-down")
            .disabled(index + 1 >= len)
            .on_select(menu_action(
                view.clone(),
                id.clone(),
                move |view, id, _, cx| view.move_workspace(id, index + 1, cx),
            )),
        MenuItem::separator(),
        MenuItem::new("remove", "Remove…")
            .icon("delete")
            .danger()
            .on_select(on(|view, id, _, cx| {
                view.dialog = DialogState::RemoveWorkspace(id.clone());
                cx.notify();
            })),
    ]);
    items
}

/// Sets the color of workspace `id` from its menu.
fn set_workspace_color(
    view: &mut NativeView,
    id: &WorkspaceId,
    color: Option<PresetColor>,
    cx: &mut Context<NativeView>,
) {
    let client = view.client.clone();
    let id = id.clone();
    view.run(
        async move {
            client
                .set_workspace_color(id, color)
                .await
                .map(|_| Effect::None)
        },
        cx,
    );
}

impl NativeView {
    fn change_workspace_icon(
        &mut self,
        id: WorkspaceId,
        icon: Option<PresetIcon>,
        cx: &mut Context<Self>,
    ) {
        let client = self.client.clone();
        self.run(
            async move {
                client
                    .set_workspace_icon(id, icon)
                    .await
                    .map(|_| Effect::None)
            },
            cx,
        );
    }
}
