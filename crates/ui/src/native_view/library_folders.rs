//! Library folders: the header's folder tabs, their inline naming, menu, and deletion, and the
//! skill menu that moves a skill between folders.

use super::controls::{Inactive, app_icon, button};
use super::menu::MenuItem;
use super::sidebar_presets::menu_action;
use super::sidebar_row::with_menu;
use super::{DialogState, Effect, LibraryFilter, NativeView, Route};
use gpui_kit::component::{
    ActiveTheme, Selectable, Sizable,
    button::ButtonVariants,
    dialog::Dialog,
    h_flex,
    input::{Input, InputEvent, InputState},
};
use gpui_kit::{
    AnyElement, ClickEvent, Context, Entity, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::{Folder, FolderId, OrganizationChange, Skill, SkillId};

/// Which folder name the inline header field edits.
#[derive(Clone, PartialEq, Eq)]
pub(super) enum FolderEdit {
    Create,
    Rename(FolderId),
}

impl NativeView {
    fn folders(&self) -> &[Folder] {
        self.snapshot
            .library
            .as_ref()
            .map_or(&[], |library| &library.library.folders)
    }

    /// The chosen folder, while Library lists every skill; otherwise none applies.
    pub(super) fn shown_folder(&self) -> Option<&Folder> {
        if self.route != Route::Library(LibraryFilter::All) {
            return None;
        }
        let id = self.library_folder.as_ref()?;
        self.folders().iter().find(|folder| &folder.id == id)
    }

    /// The name of `skill`'s folder, shown on its item while Library lists all skills.
    pub(super) fn folder_label(&self, skill: &Skill) -> Option<&str> {
        if self.route != Route::Library(LibraryFilter::All) || self.shown_folder().is_some() {
            return None;
        }
        let id = skill.folder_id.as_ref()?;
        self.folders()
            .iter()
            .find(|folder| &folder.id == id)
            .map(|folder| folder.name.as_str())
    }

    /// All skills, a separator, each folder, then Add folder, scrolling sideways when they
    /// overflow the header.
    pub(super) fn render_folder_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut strip = h_flex()
            .id("folder-tabs")
            .min_w_0()
            .overflow_x_scroll()
            .gap_1()
            .child(self.folder_tab(None, "All skills", cx))
            .child(
                div()
                    .flex_shrink_0()
                    .w(px(1.))
                    .h(px(16.))
                    .mx_1()
                    .bg(cx.theme().border),
            );
        for folder in self.folders() {
            strip = if self.folder_edit == Some(FolderEdit::Rename(folder.id.clone())) {
                strip.child(self.folder_name_field(cx))
            } else {
                strip.child(self.folder_tab(Some(folder), &folder.name, cx))
            };
        }
        if self.folder_edit == Some(FolderEdit::Create) {
            strip = strip.child(self.folder_name_field(cx));
        } else {
            strip = strip.child(
                button("folder-create")
                    .ghost()
                    .small()
                    .flex_shrink_0()
                    .icon(app_icon("plus"))
                    .label("Add folder")
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
    fn folder_tab(
        &self,
        folder: Option<&Folder>,
        name: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = folder.map(|folder| folder.id.clone());
        let element = id
            .as_ref()
            .map_or_else(|| "folder-all".to_owned(), |id| format!("folder-{id}"));
        let chosen = id.clone();
        let selected = self.library_folder == id;
        let tab = button(element.clone())
            .ghost()
            .small()
            .flex_shrink_0()
            .label(name.to_owned())
            .selected(selected)
            // Unchosen tabs are muted; hovering brightens them to the ghost button's text color.
            .when(!selected, |tab| tab.text_color(cx.theme().muted_foreground))
            .on_click(cx.listener(move |view, event: &ClickEvent, window, cx| {
                // The header is the window's title bar, whose double click zooms the window.
                cx.stop_propagation();
                view.library_folder = chosen.clone();
                if let Some(id) = &chosen
                    && event.click_count() > 1
                {
                    view.begin_folder_edit(FolderEdit::Rename(id.clone()), window, cx);
                }
                cx.notify();
            }));
        let Some(id) = id else {
            return tab.into_any_element();
        };
        let wrapper = div()
            .id(format!("{element}-menu"))
            .flex_shrink_0()
            .child(tab);
        with_menu(wrapper, element, true, self, cx, move |view, _, cx| {
            view.folder_menu(&id, cx)
        })
        .into_any_element()
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
                .disabled(folders.is_empty())
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

    fn folder_name_field(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("folder-name-field")
            .flex_shrink_0()
            .w(px(160.))
            .child(Input::new(&self.folder_name).small())
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
