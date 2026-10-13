//! The sidebar Workspaces section: user-ordered workspace rows with inline renaming, row menus,
//! and drag reordering, the skeleton row of a workspace being added, and the background rescan
//! that opening a target starts.

use std::{cell::Cell, time::Instant};

use super::controls::{Inactive, app_icon};
use super::sidebar::SYSTEM;
use super::sidebar_folders::{RowPlacement, menu_action, menu_icon};
use super::sidebar_reorder::{RowDrag, RowDragPreview, grab_cursor};
use super::sidebar_row::{activate, add_button, count, dot, icon_color, name_field, row, spinner};
use super::{DialogState, Effect, NativeView, Route};
use gpui_kit::component::{
    ActiveTheme,
    input::{InputEvent, InputState},
    menu::{ContextMenuExt, PopupMenu, PopupMenuItem},
    skeleton::Skeleton,
    tooltip::Tooltip,
};
use gpui_kit::{
    AnyElement, AppContext, Context, DragMoveEvent, Entity, InteractiveElement, IntoElement,
    MouseButton, ParentElement, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use skillbinder_proto::{
    DeploymentTarget, ScanStatus, SidebarSection, SkillId, Workspace, WorkspaceId,
};
use skillbinder_theme::sidebar::{
    DETAIL_SIZE, ICON_GAP, ICON_SIZE, INSET, RADIUS, ROW_GAP, ROW_HEIGHT, ROW_PADDING,
};

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
                    let icon = app_icon("code-folder").text_color(cx.theme().muted_foreground);
                    return name_field(
                        "workspace-name-field",
                        icon,
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
        let adding = self.adding_workspace();
        let add = add_button("workspace-add", "Add workspace", cx)
            .inactive(adding)
            .on_click(cx.listener(|view, _, _, cx| {
                view.unfold_section(SidebarSection::Workspaces, cx);
                view.choose_workspace_folder(cx)
            }));
        let items = 1 + rows.len() + usize::from(adding);
        div()
            .flex()
            .flex_col()
            .child(self.section_header(SidebarSection::Workspaces, "Workspaces", Some(add), cx))
            .children(
                self.section_rows(
                    SidebarSection::Workspaces,
                    items,
                    div()
                        .id("sidebar-workspaces")
                        .flex()
                        .flex_col()
                        .gap(px(ROW_GAP))
                        .child(self.render_system_row(cx))
                        .child(
                            // Drag moves are measured from the first workspace slot.
                            div()
                                .id("sidebar-workspace-rows")
                                .flex()
                                .flex_col()
                                .gap(px(ROW_GAP))
                                .on_drag_move(cx.listener(
                                    |view,
                                     event: &DragMoveEvent<RowDrag<WorkspaceId>>,
                                     window,
                                     cx| {
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
                        )
                        .when(adding, |list| list.child(skeleton_row())),
                ),
            )
    }

    /// System is the Global deployment target: fixed first in Workspaces, so it cannot be
    /// renamed, moved, or removed. Its count is the number of skills installed there, and its
    /// tooltip shows the home folder its skills live under.
    fn render_system_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let target = DeploymentTarget::Global;
        let selected = self.route == Route::Target(target.clone());
        let trailing = self.target_trailing(&target, false, cx);
        let accessible = self.target_label(SYSTEM.to_owned(), &target);
        let home: SharedString = self.snapshot.home.clone().into();
        activate(
            row(
                "workspace-system",
                app_icon("hard-drive"),
                None,
                SYSTEM,
                selected,
                cx,
            ),
            accessible,
            cx,
            |view, window, cx| view.open_target(DeploymentTarget::Global, window, cx),
        )
        .child(trailing)
        .tooltip(move |window, cx| Tooltip::new(home.clone()).build(window, cx))
        .into_any_element()
    }

    fn render_workspace_row(
        &self,
        workspace: &Workspace,
        placement: RowPlacement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let target = DeploymentTarget::Workspace(workspace.id.clone());
        let selected = self.route == Route::Target(target.clone());
        let label: SharedString = workspace.name.clone().into();
        let accessible = if workspace.missing {
            format!("{}, folder missing", workspace.name)
        } else {
            self.target_label(workspace.name.clone(), &target)
        };
        let trailing = self.target_trailing(&target, workspace.missing, cx);
        let deployed = self.deployment_count(&target);
        let menu = self.workspace_menu(&workspace.id, placement.index, placement.len, cx);
        activate(
            row(
                format!("workspace-{}", workspace.id),
                app_icon("code-folder"),
                None,
                label.clone(),
                selected,
                cx,
            ),
            accessible,
            cx,
            move |view, window, cx| view.open_target(target.clone(), window, cx),
        )
        .child(trailing)
        .relative()
        .top(px(placement.offset))
        .when(placement.floating, |row| row.opacity(0.0))
        .on_drag(
            RowDrag {
                id: workspace.id.clone(),
                icon: "code-folder",
                label,
                tint: None,
                count: deployed,
                width: self.sidebar_width - 2.0 * INSET,
                grab: Cell::new(0.0),
            },
            |drag, offset, _, cx| {
                drag.grab.set(f32::from(offset.y));
                cx.new(|_| RowDragPreview::new(drag))
            },
        )
        .context_menu(menu)
        .into_any_element()
    }

    /// The row's accessible name, which reports drifted deployments.
    pub(super) fn target_label(&self, name: String, target: &DeploymentTarget) -> String {
        match self.drifted(target) {
            0 => name,
            1 => format!("{name}, 1 deployment drifted"),
            drifted => format!("{name}, {drifted} deployments drifted"),
        }
    }

    /// A spinner while rescanning, a warning while the folder is missing, a warning dot while a
    /// deployment drifted, else the target's number of deployments.
    pub(super) fn target_trailing(
        &self,
        target: &DeploymentTarget,
        missing: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.snapshot.rescanning.contains(target) {
            spinner(cx)
        } else if missing {
            app_icon("alert-circle")
                .size(px(ICON_SIZE))
                .flex_shrink_0()
                .text_color(cx.theme().warning)
                .into_any_element()
        } else if self.drifted(target) > 0 {
            dot(cx.theme().warning)
        } else {
            count(self.deployment_count(target), cx)
        }
    }

    /// Rename, order, and remove actions for one workspace row.
    fn workspace_menu(
        &self,
        id: &WorkspaceId,
        index: usize,
        len: usize,
        cx: &mut Context<Self>,
    ) -> impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static {
        let view = cx.entity().downgrade();
        let id = id.clone();
        move |menu, _, cx| {
            let danger = cx.theme().danger;
            let plain = icon_color(cx);
            menu.item(
                PopupMenuItem::new("Rename")
                    .icon(menu_icon("pencil-edit", plain))
                    .on_click(menu_action(
                        view.clone(),
                        id.clone(),
                        |view, id, window, cx| view.begin_workspace_rename(id, window, cx),
                    )),
            )
            .separator()
            .item(
                PopupMenuItem::new("Move Up")
                    .icon(menu_icon("arrow-up", plain))
                    .disabled(index == 0)
                    .on_click(menu_action(
                        view.clone(),
                        id.clone(),
                        move |view, id, _, cx| view.move_workspace(id, index.saturating_sub(1), cx),
                    )),
            )
            .item(
                PopupMenuItem::new("Move Down")
                    .icon(menu_icon("arrow-down", plain))
                    .disabled(index + 1 >= len)
                    .on_click(menu_action(
                        view.clone(),
                        id.clone(),
                        move |view, id, _, cx| view.move_workspace(id, index + 1, cx),
                    )),
            )
            .separator()
            .item(
                // A custom element, because plain items draw their label in the menu color.
                PopupMenuItem::element(move |_, _| div().text_color(danger).child("Remove…"))
                    .icon(menu_icon("delete", danger))
                    .on_click(menu_action(view.clone(), id.clone(), |view, id, _, cx| {
                        view.dialog = DialogState::RemoveWorkspace(id.clone());
                        cx.notify();
                    })),
            )
        }
    }

    /// Opens a target's page and rescans it in the background.
    pub(super) fn open_target(
        &mut self,
        target: DeploymentTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.navigate(Route::Target(target.clone()), window, cx);
        self.client.spawn_rescan(target);
    }

    pub(super) fn adding_workspace(&self) -> bool {
        self.snapshot
            .workspace_addition
            .as_ref()
            .is_some_and(|addition| addition.status == ScanStatus::Running)
    }

    pub(super) fn deployment_count(&self, target: &DeploymentTarget) -> usize {
        self.snapshot
            .deployments
            .iter()
            .filter(|deployment| &deployment.target == target)
            .count()
    }

    /// Names of the targets that hold `skill`: System first, then workspaces in sidebar order.
    pub(super) fn deployment_targets(&self, skill: &SkillId) -> Vec<String> {
        let holds = |target: DeploymentTarget| {
            self.snapshot
                .deployments
                .iter()
                .any(|deployment| &deployment.skill_id == skill && deployment.target == target)
        };
        let system = holds(DeploymentTarget::Global).then(|| SYSTEM.to_owned());
        system
            .into_iter()
            .chain(
                self.snapshot
                    .workspaces
                    .iter()
                    .filter(|workspace| holds(DeploymentTarget::Workspace(workspace.id.clone())))
                    .map(|workspace| workspace.name.clone()),
            )
            .collect()
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

/// Placeholder row for the workspace being added.
fn skeleton_row() -> AnyElement {
    let bar = || Skeleton::new().h(px(DETAIL_SIZE)).rounded(px(RADIUS / 2.0));
    div()
        .id("workspace-skeleton")
        .aria_label("Adding workspace")
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(ICON_GAP))
        .h(px(ROW_HEIGHT))
        .px(px(ROW_PADDING))
        .child(
            Skeleton::new()
                .size(px(ICON_SIZE))
                .rounded(px(RADIUS / 2.0)),
        )
        .child(bar().flex_1())
        .child(bar().w(px(DETAIL_SIZE)))
        .into_any_element()
}
