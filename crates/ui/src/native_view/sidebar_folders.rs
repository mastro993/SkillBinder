//! The sidebar Folders section: user-ordered folders, inline naming, row menus,
//! colors, and the reorder menu actions.

use std::{cell::Cell, collections::BTreeMap, time::Instant};

use super::controls::app_icon;
use super::sidebar_reorder::{RowDrag, RowDragPreview};
use super::sidebar_row::{activate, add_button, count, icon_color, name_field, row};
use super::{Effect, LibraryFilter, NativeView, Route};
use gpui_kit::component::{
    ActiveTheme, Icon, Side,
    input::{InputEvent, InputState},
    menu::{ContextMenuExt, PopupMenuItem},
};
use gpui_kit::{
    AnyElement, App, AppContext, ClickEvent, Context, DragMoveEvent, Entity, Hsla,
    InteractiveElement, IntoElement, MouseButton, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, WeakEntity, Window, div, prelude::FluentBuilder, px, rgb,
};
use skillbinder_proto::{
    Folder, FolderColor, FolderId, LibrarySnapshot, OrganizationChange, SidebarSection,
};
use skillbinder_theme::sidebar::{ICON_GAP, INSET, ROW_GAP, SECTION_GAP};

/// Which folder name the inline sidebar field edits.
#[derive(Clone, PartialEq, Eq)]
pub(super) enum FolderEdit {
    Create,
    Rename(FolderId),
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
    pub(super) fn render_folders(
        &self,
        library: &LibrarySnapshot,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mut counts = BTreeMap::<&FolderId, usize>::new();
        for folder in library
            .library
            .skills
            .iter()
            .filter_map(|skill| skill.folder_id.as_ref())
        {
            *counts.entry(folder).or_default() += 1;
        }
        let (order, floating) = self
            .folder_reorder
            .displayed(&self.folder_ids(), cx.has_active_drag());
        let now = Instant::now();
        if self.folder_reorder.moving(now) {
            window.request_animation_frame();
        }
        let selected = |candidate: LibraryFilter| self.route == Route::Library(candidate);
        let folders = &library.library.folders;
        let rows: Vec<AnyElement> = order
            .iter()
            .filter_map(|id| folders.iter().find(|folder| &folder.id == id))
            .enumerate()
            .map(|(index, folder)| {
                if self.folder_edit == Some(FolderEdit::Rename(folder.id.clone())) {
                    return self.render_folder_name_field(folder.color, cx);
                }
                let skills = counts.get(&folder.id).copied().unwrap_or(0);
                let selected = selected(LibraryFilter::Folder(folder.id.clone()));
                let placement = RowPlacement {
                    index,
                    len: order.len(),
                    offset: self.folder_reorder.offset(&folder.id, now),
                    floating: floating.as_ref() == Some(&folder.id),
                };
                self.render_folder_row(folder, placement, skills, selected, cx)
            })
            .collect();
        let creating = self.folder_edit == Some(FolderEdit::Create);
        let items = rows.len() + usize::from(creating);
        div()
            .flex()
            .flex_col()
            .child(self.render_folders_header(cx))
            .children(
                self.section_rows(
                    SidebarSection::Folders,
                    items,
                    div()
                        .id("sidebar-folders")
                        .flex()
                        .flex_col()
                        .gap(px(ROW_GAP))
                        .child(
                            // Drag moves are measured from the first folder slot; releasing the
                            // pointer anywhere keeps the order shown.
                            div()
                                .id("sidebar-folder-rows")
                                .flex()
                                .flex_col()
                                .gap(px(ROW_GAP))
                                .on_drag_move(cx.listener(
                                    |view, event: &DragMoveEvent<RowDrag<FolderId>>, window, cx| {
                                        let drag = event.drag(cx);
                                        let top = drag.top(event.event.position, event.bounds);
                                        view.drag_folder(drag.id.clone(), top, window, cx)
                                    },
                                ))
                                .on_mouse_up(
                                    MouseButton::Left,
                                    cx.listener(|view, _, _, cx| view.finish_folder_drag(cx)),
                                )
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    cx.listener(|view, _, _, cx| view.finish_folder_drag(cx)),
                                )
                                .children(rows),
                        )
                        .when(creating, |list| {
                            list.child(self.render_folder_name_field(None, cx))
                        }),
                ),
            )
    }

    fn render_folders_header(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let add = add_button("folder-create", "Add folder", cx).on_click(cx.listener(
            |view, _, window, cx| {
                view.unfold_section(SidebarSection::Folders, cx);
                view.begin_folder_edit(FolderEdit::Create, window, cx)
            },
        ));
        // The first section splits its gap with the divider above it, which sits under Library.
        self.section_header(SidebarSection::Folders, "Folders", Some(add), cx)
            .mt(px(SECTION_GAP / 2.0))
    }

    fn render_folder_row(
        &self,
        folder: &Folder,
        placement: RowPlacement,
        skills: usize,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let dark = cx.theme().mode.is_dark();
        let id = folder.id.clone();
        let label: SharedString = folder.name.clone().into();
        let route = Route::Library(LibraryFilter::Folder(id.clone()));
        let tint = folder.color.map(|color| folder_tint(color, dark));
        let menu = self.folder_menu(folder, placement.index, placement.len, cx);
        activate(
            row(
                format!("folder-{id}"),
                app_icon("folder"),
                tint,
                label.clone(),
                selected,
                cx,
            ),
            label.clone(),
            cx,
            move |view, window, cx| view.navigate(route.clone(), window, cx),
        )
        .child(count(skills, cx))
        .relative()
        .top(px(placement.offset))
        .when(placement.floating, |row| row.opacity(0.0))
        .on_drag(
            RowDrag {
                id,
                icon: "folder",
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
        .context_menu(menu)
        .into_any_element()
    }

    /// Rename, color, order, and delete actions for one folder row.
    fn folder_menu(
        &self,
        folder: &Folder,
        index: usize,
        len: usize,
        cx: &mut Context<Self>,
    ) -> impl Fn(
        gpui_kit::component::menu::PopupMenu,
        &mut Window,
        &mut Context<gpui_kit::component::menu::PopupMenu>,
    ) -> gpui_kit::component::menu::PopupMenu
    + 'static {
        let view = cx.entity().downgrade();
        let (id, current) = (folder.id.clone(), folder.color);
        move |menu, window, cx| {
            let (color_view, color_id) = (view.clone(), id.clone());
            let danger = cx.theme().danger;
            let plain = icon_color(cx);
            menu.item(
                PopupMenuItem::new("Rename")
                    .icon(menu_icon("pencil-edit", plain))
                    .on_click(menu_action(
                        view.clone(),
                        id.clone(),
                        |view, id, window, cx| {
                            view.begin_folder_edit(FolderEdit::Rename(id.clone()), window, cx)
                        },
                    )),
            )
            .submenu_with_icon(
                Some(menu_icon("paint-board", plain)),
                "Color",
                window,
                cx,
                move |menu, _, cx| {
                    let dark = cx.theme().mode.is_dark();
                    let plain = icon_color(cx);
                    let choices = std::iter::once(None).chain(FolderColor::ALL.map(Some));
                    // Each choice shows its tinted folder, so the check moves to the right.
                    choices.fold(menu.check_side(Side::Right), |menu, color| {
                        let (view, id) = (color_view.clone(), color_id.clone());
                        let swatch = match color {
                            Some(color) => menu_icon("folder", folder_tint(color, dark)),
                            None => menu_icon("folder", plain),
                        };
                        menu.item(
                            PopupMenuItem::new(color.map_or("None", color_name))
                                .icon(swatch)
                                .checked(current == color)
                                .on_click(move |_, _, cx| {
                                    let _ = view.update(cx, |view, cx| {
                                        view.set_folder_color(id.clone(), color, cx)
                                    });
                                }),
                        )
                    })
                },
            )
            .separator()
            .item(
                PopupMenuItem::new("Move Up")
                    .icon(menu_icon("arrow-up", plain))
                    .disabled(index == 0)
                    .on_click(menu_action(
                        view.clone(),
                        id.clone(),
                        move |view, id, _, cx| view.move_folder(id, index.saturating_sub(1), cx),
                    )),
            )
            .item(
                PopupMenuItem::new("Move Down")
                    .icon(menu_icon("arrow-down", plain))
                    .disabled(index + 1 >= len)
                    .on_click(menu_action(
                        view.clone(),
                        id.clone(),
                        move |view, id, _, cx| view.move_folder(id, index + 1, cx),
                    )),
            )
            .separator()
            .item(
                // A custom element, because plain items draw their label in the menu color.
                PopupMenuItem::element(move |_, _| div().text_color(danger).child("Delete…"))
                    .icon(menu_icon("delete", danger))
                    .on_click(menu_action(view.clone(), id.clone(), |view, id, _, cx| {
                        let (client, id) = (view.client.clone(), id.clone());
                        view.run(
                                async move {
                                    client.preview_delete(id).await.map(Effect::DeletePreview)
                                },
                                cx,
                            );
                    })),
            )
        }
    }

    /// The inline name field shown in place of a folder row, or below the list when creating.
    fn render_folder_name_field(
        &self,
        color: Option<FolderColor>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let tint = color.map(|color| folder_tint(color, theme.mode.is_dark()));
        let icon = app_icon("folder").text_color(tint.unwrap_or(theme.muted_foreground));
        name_field(
            "folder-name-field",
            icon,
            &self.folder_name,
            cx,
            Self::end_folder_edit,
        )
    }

    pub(super) fn folder_name_event(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { .. } => self.submit_folder_edit(window, cx),
            InputEvent::Blur => self.end_folder_edit(window, cx),
            InputEvent::Change | InputEvent::Focus => {}
        }
    }

    fn begin_folder_edit(&mut self, edit: FolderEdit, window: &mut Window, cx: &mut Context<Self>) {
        let name = match &edit {
            FolderEdit::Create => String::new(),
            FolderEdit::Rename(id) => self
                .snapshot
                .library
                .iter()
                .flat_map(|library| &library.library.folders)
                .find(|folder| &folder.id == id)
                .map(|folder| folder.name.clone())
                .unwrap_or_default(),
        };
        self.folder_edit = Some(edit);
        self.folder_name.update(cx, |input, cx| {
            input.set_value(name, window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    fn submit_folder_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = Self::input_text(&self.folder_name, cx);
        let Some(edit) = self.folder_edit.clone() else {
            return;
        };
        self.end_folder_edit(window, cx);
        let change = match edit {
            _ if name.is_empty() => return,
            FolderEdit::Create => OrganizationChange::CreateFolder { name },
            FolderEdit::Rename(id) => OrganizationChange::RenameFolder { id, name },
        };
        self.change_folders(change, cx);
    }

    /// Closes the inline field and returns keyboard focus to the window's shortcuts.
    pub(super) fn end_folder_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.folder_edit.take().is_some() {
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    fn set_folder_color(
        &mut self,
        id: FolderId,
        color: Option<FolderColor>,
        cx: &mut Context<Self>,
    ) {
        self.change_folders(OrganizationChange::SetFolderColor { id, color }, cx);
    }

    pub(super) fn change_folders(&mut self, change: OrganizationChange, cx: &mut Context<Self>) {
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

/// A menu glyph spaced from its label like a sidebar row's: kit menus leave 4 points
/// between icon and label, so the icon tops that up to the row gap.
pub(super) fn menu_icon(name: &str, color: Hsla) -> Icon {
    app_icon(name).text_color(color).mr(px(ICON_GAP - 4.0))
}

/// A menu click handler that runs `action` on the view for one row.
pub(super) fn menu_action<Id: 'static>(
    view: WeakEntity<NativeView>,
    id: Id,
    action: impl Fn(&mut NativeView, &Id, &mut Window, &mut Context<NativeView>) + 'static,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    move |_, window, cx| {
        let _ = view.update(cx, |view, cx| action(view, &id, window, cx));
    }
}

fn color_name(color: FolderColor) -> &'static str {
    match color {
        FolderColor::Gray => "Gray",
        FolderColor::Red => "Red",
        FolderColor::Orange => "Orange",
        FolderColor::Yellow => "Yellow",
        FolderColor::Green => "Green",
        FolderColor::Blue => "Blue",
        FolderColor::Purple => "Purple",
        FolderColor::Pink => "Pink",
    }
}

/// Palette tones that keep contrast on the dark and light sidebar surfaces.
fn folder_tint(color: FolderColor, dark: bool) -> Hsla {
    let (dark_tone, light_tone) = match color {
        FolderColor::Gray => (0xa3a3a3, 0x737373),
        FolderColor::Red => (0xf87171, 0xdc2626),
        FolderColor::Orange => (0xfb923c, 0xea580c),
        FolderColor::Yellow => (0xfacc15, 0xca8a04),
        FolderColor::Green => (0x4ade80, 0x16a34a),
        FolderColor::Blue => (0x60a5fa, 0x2563eb),
        FolderColor::Purple => (0xc084fc, 0x9333ea),
        FolderColor::Pink => (0xf472b6, 0xdb2777),
    };
    rgb(if dark { dark_tone } else { light_tone }).into()
}
