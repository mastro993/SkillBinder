//! The sidebar Folders section: Unfiled, user-ordered folders, inline naming, row menus,
//! colors, and drag reordering.

use std::collections::BTreeMap;

use super::controls::{app_icon, button};
use super::sidebar_row::{activate, count, row};
use super::{Effect, LibraryFilter, NativeView, Route};
use gpui_kit::component::{
    ActiveTheme, Sizable,
    button::ButtonVariants,
    input::{Input, InputEvent, InputState},
    menu::{ContextMenuExt, PopupMenuItem},
};
use gpui_kit::{
    AnyElement, App, AppContext, ClickEvent, Context, Entity, Hsla, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, WeakEntity, Window, div, prelude::FluentBuilder, px, rgb,
};
use skillbinder_proto::{Folder, FolderColor, FolderId, LibrarySnapshot, OrganizationChange};
use skillbinder_theme::sidebar::{
    DETAIL_SIZE, ICON_GAP, ICON_SIZE, LABEL_SIZE, RADIUS, ROW_GAP, ROW_HEIGHT, ROW_PADDING,
    SECTION_GAP, SECTION_HEIGHT,
};

/// Which folder name the inline sidebar field edits.
#[derive(Clone, PartialEq, Eq)]
pub(super) enum FolderEdit {
    Create,
    Rename(FolderId),
}

/// Drag payload for reordering folder rows.
struct FolderDrag {
    id: FolderId,
    label: SharedString,
}

/// Floating label under the pointer while a folder row is dragged.
struct FolderDragPreview(SharedString);

impl Render for FolderDragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .px(px(ROW_PADDING))
            .py_1()
            .rounded(px(RADIUS))
            .bg(theme.sidebar_accent)
            .text_color(theme.sidebar_accent_foreground)
            .child(self.0.clone())
    }
}

impl NativeView {
    pub(super) fn render_folders(
        &self,
        library: &LibrarySnapshot,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mut counts = BTreeMap::<Option<&FolderId>, usize>::new();
        for skill in &library.library.skills {
            *counts.entry(skill.folder_id.as_ref()).or_default() += 1;
        }
        let folders = &library.library.folders;
        // Other routes report all skills, which no folder row represents.
        let selected = |candidate: LibraryFilter| self.library_filter() == &candidate;
        let unfiled = activate(
            row(
                "folder-unfiled",
                app_icon("folder"),
                None,
                "Unfiled",
                selected(LibraryFilter::Unfiled),
                cx,
            ),
            "Unfiled",
            cx,
            |view, window, cx| view.navigate(Route::Library(LibraryFilter::Unfiled), window, cx),
        )
        .child(count(counts.get(&None).copied().unwrap_or(0), cx));
        let rows: Vec<AnyElement> = folders
            .iter()
            .enumerate()
            .map(|(index, folder)| {
                if self.folder_edit == Some(FolderEdit::Rename(folder.id.clone())) {
                    return self.render_folder_name_field(folder.color, cx);
                }
                let skills = counts.get(&Some(&folder.id)).copied().unwrap_or(0);
                let selected = selected(LibraryFilter::Folder(folder.id.clone()));
                self.render_folder_row(folder, index, folders.len(), skills, selected, cx)
            })
            .collect();
        let creating = self.folder_edit == Some(FolderEdit::Create);
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(self.render_folders_header(cx))
            .child(
                div()
                    .id("sidebar-folders")
                    .flex()
                    .flex_col()
                    .gap(px(ROW_GAP))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(unfiled)
                    .children(rows)
                    .when(creating, |list| {
                        list.child(self.render_folder_name_field(None, cx))
                    }),
            )
    }

    fn render_folders_header(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let theme = cx.theme();
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .h(px(SECTION_HEIGHT))
            .mt(px(SECTION_GAP))
            .pl(px(ROW_PADDING))
            // Centres the button's glyph on the rows' trailing column.
            .pr(px(ROW_PADDING - (ROW_HEIGHT - 8.0 - ICON_SIZE) / 2.0))
            .text_size(px(DETAIL_SIZE))
            .text_color(theme.muted_foreground.opacity(0.8))
            .child(div().flex_1().child("Folders"))
            .child(
                button("folder-create")
                    .ghost()
                    .xsmall()
                    .icon(
                        app_icon("plus")
                            .size(px(ICON_SIZE))
                            .text_color(theme.muted_foreground),
                    )
                    .accessibility_label("New folder")
                    .on_click(cx.listener(|view, _, window, cx| {
                        view.begin_folder_edit(FolderEdit::Create, window, cx)
                    })),
            )
    }

    fn render_folder_row(
        &self,
        folder: &Folder,
        index: usize,
        len: usize,
        skills: usize,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let dark = cx.theme().mode.is_dark();
        let drop_target = cx.theme().sidebar_accent;
        let id = folder.id.clone();
        let label: SharedString = folder.name.clone().into();
        let route = Route::Library(LibraryFilter::Folder(id.clone()));
        let tint = folder.color.map(|color| folder_tint(color, dark));
        let menu = self.folder_menu(folder, index, len, cx);
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
        .on_drag(
            FolderDrag {
                id: id.clone(),
                label,
            },
            |drag, _, _, cx| cx.new(|_| FolderDragPreview(drag.label.clone())),
        )
        .drag_over::<FolderDrag>(move |style, _, _, _| style.bg(drop_target))
        .on_drop(
            cx.listener(move |view, drag: &FolderDrag, _, cx| {
                view.move_folder(&drag.id, index, cx)
            }),
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
            menu.item(PopupMenuItem::new("Rename").on_click(menu_action(
                view.clone(),
                id.clone(),
                |view, id, window, cx| {
                    view.begin_folder_edit(FolderEdit::Rename(id.clone()), window, cx)
                },
            )))
            .submenu("Color", window, cx, move |menu, _, _| {
                let choices = std::iter::once(None).chain(FolderColor::ALL.map(Some));
                choices.fold(menu, |menu, color| {
                    let (view, id) = (color_view.clone(), color_id.clone());
                    menu.item(
                        PopupMenuItem::new(color.map_or("None", color_name))
                            .checked(current == color)
                            .on_click(move |_, _, cx| {
                                let _ = view.update(cx, |view, cx| {
                                    view.set_folder_color(id.clone(), color, cx)
                                });
                            }),
                    )
                })
            })
            .separator()
            .item(
                PopupMenuItem::new("Move Up")
                    .disabled(index == 0)
                    .on_click(menu_action(
                        view.clone(),
                        id.clone(),
                        move |view, id, _, cx| view.move_folder(id, index.saturating_sub(1), cx),
                    )),
            )
            .item(
                PopupMenuItem::new("Move Down")
                    .disabled(index + 1 >= len)
                    .on_click(menu_action(
                        view.clone(),
                        id.clone(),
                        move |view, id, _, cx| view.move_folder(id, index + 1, cx),
                    )),
            )
            .separator()
            .item(PopupMenuItem::new("Delete…").on_click(menu_action(
                view.clone(),
                id.clone(),
                |view, id, _, cx| {
                    let (client, id) = (view.client.clone(), id.clone());
                    view.run(
                        async move { client.preview_delete(id).await.map(Effect::DeletePreview) },
                        cx,
                    );
                },
            )))
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
        div()
            .id("folder-name-field")
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(px(ICON_GAP))
            .h(px(ROW_HEIGHT))
            .px(px(ROW_PADDING))
            .rounded(px(RADIUS))
            .bg(theme.sidebar_accent)
            .child(
                app_icon("folder")
                    .size(px(ICON_SIZE))
                    .text_color(tint.unwrap_or(theme.muted_foreground)),
            )
            .child(
                div().flex_1().min_w_0().child(
                    // Unpadded, so the edited name stays aligned with row labels.
                    Input::new(&self.folder_name)
                        .appearance(false)
                        .small()
                        .px_0()
                        .text_size(px(LABEL_SIZE)),
                ),
            )
            .capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    view.end_folder_edit(window, cx);
                    cx.stop_propagation();
                }
            }))
            .into_any_element()
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
    fn end_folder_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    /// Moves `id` to `target`, the index of the row it was dropped on or stepped to.
    fn move_folder(&mut self, id: &FolderId, target: usize, cx: &mut Context<Self>) {
        let Some(library) = &self.snapshot.library else {
            return;
        };
        let order: Vec<_> = library
            .library
            .folders
            .iter()
            .map(|f| f.id.clone())
            .collect();
        if let Some(order) = moved(order, id, target) {
            self.change_folders(OrganizationChange::ReorderFolders { order }, cx);
        }
    }

    fn change_folders(&mut self, change: OrganizationChange, cx: &mut Context<Self>) {
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

/// A menu click handler that runs `action` on the view for one folder.
fn menu_action(
    view: WeakEntity<NativeView>,
    id: FolderId,
    action: impl Fn(&mut NativeView, &FolderId, &mut Window, &mut Context<NativeView>) + 'static,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    move |_, window, cx| {
        let _ = view.update(cx, |view, cx| action(view, &id, window, cx));
    }
}

/// `order` with `id` moved to `target`, or `None` when nothing would change.
fn moved(mut order: Vec<FolderId>, id: &FolderId, target: usize) -> Option<Vec<FolderId>> {
    let from = order.iter().position(|candidate| candidate == id)?;
    let target = target.min(order.len() - 1);
    if from == target {
        return None;
    }
    let id = order.remove(from);
    order.insert(target, id);
    Some(order)
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

#[cfg(test)]
mod tests {
    use super::moved;
    use skillbinder_proto::FolderId;

    fn ids(names: &[&str]) -> Vec<FolderId> {
        names.iter().map(|name| FolderId((*name).into())).collect()
    }

    #[test]
    fn dropping_on_a_row_takes_its_place() {
        let order = ids(&["a", "b", "c", "d"]);
        assert_eq!(
            moved(order.clone(), &FolderId("a".into()), 2),
            Some(ids(&["b", "c", "a", "d"]))
        );
        assert_eq!(
            moved(order.clone(), &FolderId("d".into()), 1),
            Some(ids(&["a", "d", "b", "c"]))
        );
        assert_eq!(
            moved(order.clone(), &FolderId("b".into()), 9),
            Some(ids(&["a", "c", "d", "b"]))
        );
        assert_eq!(moved(order.clone(), &FolderId("c".into()), 2), None);
        assert_eq!(moved(order, &FolderId("x".into()), 0), None);
    }
}
