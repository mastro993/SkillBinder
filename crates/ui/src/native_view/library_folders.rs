//! Library folders: the header's folder tabs, their inline naming, menu, and deletion, and the
//! skill menu that moves a skill between folders.

use super::controls::{Inactive, app_icon, button};
use super::folder_reorder::{FolderDrag, draggable};
use super::menu::MenuItem;
use super::sidebar_presets::menu_action;
use super::sidebar_row::with_menu;
use super::{DialogState, Effect, NativeView, Route};
use gpui_kit::component::{
    ActiveTheme, Selectable, Sizable,
    button::{Button, ButtonVariants},
    dialog::Dialog,
    h_flex,
    input::{Input, InputEvent, InputState},
};
use gpui_kit::{
    AnyElement, ClickEvent, Context, Div, Entity, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, Stateful, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use skillbinder_proto::{Folder, FolderId, OrganizationChange, Skill, SkillId};
use skillbinder_theme::FOLDER_TAB_SIZE;
use std::cell::Cell;

/// Which folder name the inline header field edits.
#[derive(Clone, PartialEq, Eq)]
pub(super) enum FolderEdit {
    Create,
    Rename(FolderId),
}

impl NativeView {
    pub(super) fn folders(&self) -> &[Folder] {
        self.snapshot
            .library
            .as_ref()
            .map_or(&[], |library| &library.library.folders)
    }

    /// The chosen folder, while Library lists every skill; otherwise none applies.
    pub(super) fn shown_folder(&self) -> Option<&Folder> {
        if self.route != Route::Library {
            return None;
        }
        let id = self.library_folder.as_ref()?;
        self.folders().iter().find(|folder| &folder.id == id)
    }

    /// The name of `skill`'s folder, shown on its item while Library lists all skills.
    pub(super) fn folder_label(&self, skill: &Skill) -> Option<&str> {
        if self.route != Route::Library || self.shown_folder().is_some() {
            return None;
        }
        let id = skill.folder_id.as_ref()?;
        self.folders()
            .iter()
            .find(|folder| &folder.id == id)
            .map(|folder| folder.name.as_str())
    }

    /// All skills, a separator, each folder, then the Add folder plus, scrolling sideways when they
    /// overflow the header.
    pub(super) fn render_folder_tabs(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let mut strip = h_flex()
            .id("folder-tabs")
            .min_w_0()
            .overflow_x_scroll()
            .gap_1()
            .child(self.tab_button(None, "All skills", cx))
            .child(
                div()
                    .flex_shrink_0()
                    .w(px(1.))
                    .h(px(16.))
                    .mx_1()
                    .bg(cx.theme().border),
            );
        if let Some(folders) = self.render_folder_run(window, cx) {
            strip = strip.child(folders);
        }
        // A created folder shows at once; a refused one disappears with its error.
        if let Some(name) = &self.creating_folder {
            strip = strip.child(
                button("folder-creating")
                    .ghost()
                    .small()
                    .flex_shrink_0()
                    .accessibility_label(name.clone())
                    .child(tab_label(name))
                    .text_color(cx.theme().muted_foreground),
            );
        }
        if self.folder_edit == Some(FolderEdit::Create) {
            strip = strip.child(self.folder_name_field(cx));
        } else {
            strip = strip.child(
                button("folder-create")
                    .inactive(self.busy)
                    .ghost()
                    .small()
                    .flex_shrink_0()
                    .icon(app_icon("plus"))
                    .accessibility_label("Add folder")
                    .tooltip("Add folder")
                    .text_color(cx.theme().muted_foreground)
                    .on_click(cx.listener(|view, _, window, cx| {
                        cx.stop_propagation();
                        view.begin_folder_edit(FolderEdit::Create, window, cx);
                    })),
            );
        }
        strip.into_any_element()
    }

    /// One tab; clicking chooses it, and double-clicking a folder renames it.
    fn tab_button(&self, id: Option<FolderId>, name: &str, cx: &mut Context<Self>) -> Button {
        let element = id
            .as_ref()
            .map_or_else(|| "folder-all".to_owned(), |id| format!("folder-{id}"));
        let chosen = id.clone();
        let selected = self.library_folder == id;
        button(element)
            .ghost()
            .small()
            .flex_shrink_0()
            .accessibility_label(name.to_owned())
            .child(tab_label(name))
            .selected(selected)
            // The chosen tab sits on a soft muted fill rather than the kit's darker selected
            // fill. Unchosen tabs are muted; hovering brightens them to the ghost text color.
            .map(|tab| {
                if selected {
                    tab.bg(cx.theme().muted).text_color(cx.theme().foreground)
                } else {
                    tab.text_color(cx.theme().muted_foreground)
                }
            })
            .on_click(cx.listener(move |view, event: &ClickEvent, window, cx| {
                // The header is the window's title bar, whose double click zooms the window.
                cx.stop_propagation();
                // Releasing a dragged tab over itself is not a click.
                if cx.has_active_drag() {
                    return;
                }
                view.library_folder = chosen.clone();
                if let Some(id) = &chosen
                    && event.click_count() > 1
                {
                    view.begin_folder_edit(FolderEdit::Rename(id.clone()), window, cx);
                }
                cx.notify();
            }))
    }

    /// A folder's tab, which drags to reorder and opens the folder menu on right-click.
    pub(super) fn folder_tab(&self, folder: &Folder, cx: &mut Context<Self>) -> Stateful<Div> {
        let id = folder.id.clone();
        let element = format!("folder-{id}");
        let tab = self.tab_button(Some(id.clone()), &folder.name, cx);
        let drag = FolderDrag {
            id: id.clone(),
            name: folder.name.clone().into(),
            grab: Cell::new(0.0),
        };
        let wrapper = draggable(div().id(format!("{element}-menu")).child(tab), drag);
        with_menu(wrapper, element, true, self, cx, move |view, _, cx| {
            view.folder_menu(&id, cx)
        })
    }

    fn folder_menu(&self, id: &FolderId, cx: &mut Context<Self>) -> Vec<MenuItem> {
        let view = cx.entity().downgrade();
        vec![
            MenuItem::new("folder-rename", "Rename")
                .icon("pencil-edit")
                .on_select(menu_action(
                    view.clone(),
                    id.clone(),
                    |view, id, window, cx| {
                        view.begin_folder_edit(FolderEdit::Rename(id.clone()), window, cx)
                    },
                )),
            MenuItem::separator(),
            MenuItem::new("folder-delete", "Delete…")
                .icon("delete")
                .danger()
                .on_select(menu_action(view, id.clone(), |view, id, _, cx| {
                    if let Some(library) = &view.snapshot.library {
                        view.dialog = DialogState::DeleteFolder {
                            id: id.clone(),
                            revision: library.revision.clone(),
                        };
                        cx.notify();
                    }
                })),
        ]
    }

    /// Move to folder, with None and each folder; the skill's current choice is checked.
    pub(super) fn skill_menu(&self, skill: &SkillId, cx: &mut Context<Self>) -> Vec<MenuItem> {
        let view = cx.entity().downgrade();
        let current = self
            .snapshot
            .library
            .iter()
            .flat_map(|library| &library.library.skills)
            .find(|record| &record.id == skill)
            .and_then(|record| record.folder_id.clone());
        let choice = |key: String, label: &str, folder: Option<FolderId>| {
            MenuItem::new(key, label.to_owned())
                .checked(current == folder)
                .on_select(menu_action(
                    view.clone(),
                    skill.clone(),
                    move |view, skill, _, cx| view.move_to_folder(skill, folder.clone(), cx),
                ))
        };
        let folders = self.folders();
        let mut choices = vec![choice("move-folder-none".to_owned(), "None", None)];
        if !folders.is_empty() {
            choices.push(MenuItem::separator());
        }
        choices.extend(folders.iter().map(|folder| {
            choice(
                format!("move-folder-{}", folder.id),
                &folder.name,
                Some(folder.id.clone()),
            )
        }));
        vec![
            MenuItem::new("move-to-folder", "Move to folder")
                .icon("folder")
                // Another operation would refuse the move, so it is unavailable meanwhile.
                .disabled(folders.is_empty() || self.busy)
                .submenu(choices),
        ]
    }

    fn move_to_folder(
        &mut self,
        skill: &SkillId,
        folder: Option<FolderId>,
        cx: &mut Context<Self>,
    ) {
        let skill = skill.clone();
        self.change_organization(OrganizationChange::MoveToFolder { skill, folder }, cx);
    }

    /// A borderless field in a tab-shaped fill: an input's own focus ring would draw outside it
    /// and be clipped by the scrolling strip.
    pub(super) fn folder_name_field(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("folder-name-field")
            .flex()
            .flex_shrink_0()
            .items_center()
            .w(px(160.))
            .h(px(24.))
            .px_2()
            .rounded(cx.theme().radius)
            .bg(cx.theme().muted)
            .child(
                Input::new(&self.folder_name)
                    .appearance(false)
                    .small()
                    .px_0()
                    .text_size(px(FOLDER_TAB_SIZE)),
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
                .folders()
                .iter()
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
        // Another operation would refuse the change, so the field stays open with its name.
        if self.busy {
            return;
        }
        let name = Self::input_text(&self.folder_name, cx);
        let Some(edit) = self.folder_edit.clone() else {
            return;
        };
        self.end_folder_edit(window, cx);
        let change = match edit {
            _ if name.is_empty() => return,
            FolderEdit::Create => {
                self.creating_folder = Some(name.clone());
                OrganizationChange::CreateFolder { name }
            }
            FolderEdit::Rename(id) => OrganizationChange::RenameFolder { id, name },
        };
        self.change_organization(change, cx);
    }

    /// Closes the inline field and returns keyboard focus to the window's shortcuts.
    fn end_folder_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.folder_edit.take().is_some() {
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    pub(super) fn delete_folder_dialog(
        &self,
        dialog: Dialog,
        id: &FolderId,
        revision: &str,
        cx: &mut Context<Self>,
    ) -> Dialog {
        let name = self
            .folders()
            .iter()
            .find(|folder| &folder.id == id)
            .map_or("this folder", |folder| folder.name.as_str());
        let skills = self.snapshot.library.as_ref().map_or(0, |library| {
            library
                .library
                .skills
                .iter()
                .filter(|skill| skill.folder_id.as_ref() == Some(id))
                .count()
        });
        let change = OrganizationChange::DeleteFolder {
            id: id.clone(),
            revision: revision.to_owned(),
        };
        dialog
            .title(self.dialog_title("Delete folder", cx))
            .child(div().child(format!(
                "Delete {name}? Its {skills} skills stay in the library."
            )))
            .footer(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button("folder-delete-cancel")
                            .label("Cancel")
                            .inactive(self.busy)
                            .on_click(
                                cx.listener(|view, _, window, cx| view.dismiss_dialog(window, cx)),
                            ),
                    )
                    .child(
                        button("folder-delete-confirm")
                            .label("Delete folder")
                            .inactive(self.busy)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let (client, change) = (view.client.clone(), change.clone());
                                view.run(
                                    async move {
                                        client
                                            .change_organization(change)
                                            .await
                                            .map(|_| Effect::CloseDialog)
                                    },
                                    cx,
                                );
                            })),
                    ),
            )
    }
}

/// A tab's text; a button's own label keeps the kit's size for its button size.
pub(super) fn tab_label(text: &str) -> impl IntoElement {
    div()
        .text_size(px(FOLDER_TAB_SIZE))
        .whitespace_nowrap()
        .child(text.to_owned())
}
