//! The sidebar Projects section: the pinned Root row, user-ordered project rows with inline
//! renaming, row menus, and drag reordering, the skeleton row of a project being added, and
//! the background rescan that opening a target starts.

use std::{cell::Cell, time::Instant};

use super::controls::{Inactive, app_icon};
use super::sidebar_folders::{RowPlacement, menu_action, menu_icon};
use super::sidebar_reorder::{RowDrag, RowDragPreview, grab_cursor};
use super::sidebar_row::{
    activate, add_button, count, icon_color, name_field, row, section_header, spinner,
};
use super::{DialogState, Effect, NativeView};
use gpui_kit::component::{
    ActiveTheme,
    input::{InputEvent, InputState},
    menu::{ContextMenuExt, PopupMenu, PopupMenuItem},
    skeleton::Skeleton,
};
use gpui_kit::{
    AnyElement, AppContext, Context, DragMoveEvent, Entity, InteractiveElement, IntoElement,
    MouseButton, ParentElement, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use skillbinder_proto::{DeploymentTarget, Project, ProjectId, ScanStatus, SkillId};
use skillbinder_theme::sidebar::{
    DETAIL_SIZE, ICON_GAP, ICON_SIZE, INSET, RADIUS, ROW_GAP, ROW_HEIGHT, ROW_PADDING,
};

const ROOT: &str = "Root";

impl NativeView {
    pub(super) fn render_projects(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let now = Instant::now();
        if self.project_reorder.moving(now) {
            window.request_animation_frame();
        }
        let (order, floating) = self
            .project_reorder
            .displayed(&self.project_ids(), cx.has_active_drag());
        let projects = &self.snapshot.projects;
        let rows: Vec<AnyElement> = order
            .iter()
            .filter_map(|id| projects.iter().find(|project| &project.id == id))
            .enumerate()
            .map(|(index, project)| {
                if self.project_edit.as_ref() == Some(&project.id) {
                    let icon = app_icon("code-folder").text_color(cx.theme().muted_foreground);
                    return name_field(
                        "project-name-field",
                        icon,
                        &self.project_name,
                        cx,
                        Self::end_project_rename,
                    );
                }
                let placement = RowPlacement {
                    index,
                    len: order.len(),
                    offset: self.project_reorder.offset(&project.id, now),
                    floating: floating.as_ref() == Some(&project.id),
                };
                self.render_project_row(project, placement, cx)
            })
            .collect();
        let adding = self.adding_project();
        let add = add_button("project-add", "Add project", cx)
            .inactive(adding)
            .on_click(cx.listener(|view, _, _, cx| view.choose_project_folder(cx)));
        div()
            .flex()
            .flex_col()
            .child(section_header("Projects", add, cx))
            .child(
                div()
                    .id("sidebar-projects")
                    .flex()
                    .flex_col()
                    .gap(px(ROW_GAP))
                    .child(self.render_root_row(cx))
                    .child(
                        // Drag moves are measured from the first project slot.
                        div()
                            .id("sidebar-project-rows")
                            .flex()
                            .flex_col()
                            .gap(px(ROW_GAP))
                            .on_drag_move(cx.listener(
                                |view, event: &DragMoveEvent<RowDrag<ProjectId>>, window, cx| {
                                    let drag = event.drag(cx);
                                    let top = drag.top(event.event.position, event.bounds);
                                    view.drag_project(drag.id.clone(), top, window, cx)
                                },
                            ))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|view, _, _, cx| view.finish_project_drag(cx)),
                            )
                            .on_mouse_up_out(
                                MouseButton::Left,
                                cx.listener(|view, _, _, cx| view.finish_project_drag(cx)),
                            )
                            .children(rows),
                    )
                    .when(adding, |list| list.child(skeleton_row())),
            )
    }

    fn render_root_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let target = DeploymentTarget::Root;
        let selected = self.selected_target.as_ref() == Some(&target);
        let trailing = self.target_trailing(&target, false, cx);
        activate(
            row(
                "project-root",
                app_icon("hard-drive"),
                None,
                ROOT,
                selected,
                cx,
            ),
            ROOT,
            cx,
            |view, _, cx| view.open_target(DeploymentTarget::Root, cx),
        )
        .child(trailing)
        .into_any_element()
    }

    fn render_project_row(
        &self,
        project: &Project,
        placement: RowPlacement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let target = DeploymentTarget::Project(project.id.clone());
        let selected = self.selected_target.as_ref() == Some(&target);
        let label: SharedString = project.name.clone().into();
        let accessible = if project.missing {
            format!("{}, folder missing", project.name)
        } else {
            project.name.clone()
        };
        let trailing = self.target_trailing(&target, project.missing, cx);
        let deployed = self.deployment_count(&target);
        let menu = self.project_menu(&project.id, placement.index, placement.len, cx);
        activate(
            row(
                format!("project-{}", project.id),
                app_icon("code-folder"),
                None,
                label.clone(),
                selected,
                cx,
            ),
            accessible,
            cx,
            move |view, _, cx| view.open_target(target.clone(), cx),
        )
        .child(trailing)
        .relative()
        .top(px(placement.offset))
        .when(placement.floating, |row| row.opacity(0.0))
        .on_drag(
            RowDrag {
                id: project.id.clone(),
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

    /// A spinner while rescanning, a warning while the folder is missing, else the count.
    fn target_trailing(
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
        } else {
            count(self.deployment_count(target), cx)
        }
    }

    /// Rename, order, and remove actions for one project row.
    fn project_menu(
        &self,
        id: &ProjectId,
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
                        |view, id, window, cx| view.begin_project_rename(id, window, cx),
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
                        move |view, id, _, cx| view.move_project(id, index.saturating_sub(1), cx),
                    )),
            )
            .item(
                PopupMenuItem::new("Move Down")
                    .icon(menu_icon("arrow-down", plain))
                    .disabled(index + 1 >= len)
                    .on_click(menu_action(
                        view.clone(),
                        id.clone(),
                        move |view, id, _, cx| view.move_project(id, index + 1, cx),
                    )),
            )
            .separator()
            .item(
                // A custom element, because plain items draw their label in the menu color.
                PopupMenuItem::element(move |_, _| div().text_color(danger).child("Remove…"))
                    .icon(menu_icon("delete", danger))
                    .on_click(menu_action(view.clone(), id.clone(), |view, id, _, cx| {
                        view.dialog = DialogState::RemoveProject(id.clone());
                        cx.notify();
                    })),
            )
        }
    }

    /// Selects a target and rescans it in the background. Its page arrives in a later slice.
    fn open_target(&mut self, target: DeploymentTarget, cx: &mut Context<Self>) {
        self.selected_target = Some(target.clone());
        self.client.spawn_rescan(target);
        cx.notify();
    }

    pub(super) fn adding_project(&self) -> bool {
        self.snapshot
            .project_addition
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

    /// Names of the targets that hold `skill`: Root first, then projects in sidebar order.
    pub(super) fn deployment_targets(&self, skill: &SkillId) -> Vec<String> {
        let holds = |target: DeploymentTarget| {
            self.snapshot
                .deployments
                .iter()
                .any(|deployment| &deployment.skill_id == skill && deployment.target == target)
        };
        let root = holds(DeploymentTarget::Root).then(|| ROOT.to_owned());
        root.into_iter()
            .chain(
                self.snapshot
                    .projects
                    .iter()
                    .filter(|project| holds(DeploymentTarget::Project(project.id.clone())))
                    .map(|project| project.name.clone()),
            )
            .collect()
    }

    fn project_ids(&self) -> Vec<ProjectId> {
        self.snapshot
            .projects
            .iter()
            .map(|project| project.id.clone())
            .collect()
    }

    fn drag_project(
        &mut self,
        id: ProjectId,
        top: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        grab_cursor(window, cx);
        let stored = self.project_ids();
        if self
            .project_reorder
            .drag(&stored, &id, top, !cx.reduce_motion())
        {
            cx.notify();
        }
    }

    fn finish_project_drag(&mut self, cx: &mut Context<Self>) {
        let stored = self.project_ids();
        let (released, animate) = (cx.has_active_drag(), !cx.reduce_motion());
        if let Some(order) = self
            .project_reorder
            .finish(&stored, released, self.busy, animate)
        {
            self.save_project_order(order, cx);
        }
        cx.notify();
    }

    fn move_project(&mut self, id: &ProjectId, target: usize, cx: &mut Context<Self>) {
        let stored = self.project_ids();
        let animate = !cx.reduce_motion();
        if let Some(order) = self
            .project_reorder
            .move_to(&stored, id, target, self.busy, animate)
        {
            self.save_project_order(order, cx);
        }
        cx.notify();
    }

    fn save_project_order(&mut self, order: Vec<ProjectId>, cx: &mut Context<Self>) {
        let client = self.client.clone();
        self.run(
            async move { client.reorder_projects(order).await.map(|_| Effect::None) },
            cx,
        );
    }

    fn begin_project_rename(
        &mut self,
        id: &ProjectId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = self
            .snapshot
            .projects
            .iter()
            .find(|project| &project.id == id)
            .map(|project| project.name.clone())
            .unwrap_or_default();
        self.project_edit = Some(id.clone());
        self.project_name.update(cx, |input, cx| {
            input.set_value(name, window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    pub(super) fn project_name_event(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { .. } => self.submit_project_rename(window, cx),
            InputEvent::Blur => self.end_project_rename(window, cx),
            InputEvent::Change | InputEvent::Focus => {}
        }
    }

    fn submit_project_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = Self::input_text(&self.project_name, cx);
        let Some(id) = self.project_edit.clone() else {
            return;
        };
        self.end_project_rename(window, cx);
        if name.is_empty() {
            return;
        }
        let client = self.client.clone();
        self.run(
            async move { client.rename_project(id, name).await.map(|_| Effect::None) },
            cx,
        );
    }

    /// Closes the inline field and returns keyboard focus to the window's shortcuts.
    fn end_project_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.project_edit.take().is_some() {
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }
}

/// Placeholder row for the project being added.
fn skeleton_row() -> AnyElement {
    let bar = || Skeleton::new().h(px(DETAIL_SIZE)).rounded(px(RADIUS / 2.0));
    div()
        .id("project-skeleton")
        .aria_label("Adding project")
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
