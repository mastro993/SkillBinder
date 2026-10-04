use super::{DialogState, Effect, LibraryFilter, NativeView};
use ely_gpui_component::{
    buttons::Button,
    forms::{Checkbox, Input},
    theme::ActiveTheme,
};
use gpui::{Context, IntoElement, ParentElement, Styled, div};
use skillbinder_proto::*;

impl NativeView {
    pub(super) fn render_library(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let mut page = div()
            .flex()
            .flex_col()
            .gap_4()
            .child("Library")
            .child(Input::new(&self.search));
        let Some(library) = &self.snapshot.library else {
            return page.child("Finish onboarding to open your library.");
        };
        let query = self.search.read(cx).text().trim().to_lowercase();
        let mut folders: Vec<_> = library.library.folders.iter().collect();
        folders.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.id.cmp(&b.id))
        });
        page = page.child(
            div()
                .flex()
                .gap_2()
                .child(Input::new(&self.folder_name))
                .child(
                    Button::new("create-folder", "Create folder")
                        .disabled(self.busy)
                        .on_click(cx.listener(|view, _, _, cx| {
                            let name = Self::input_text(&view.folder_name, cx);
                            if name.is_empty() {
                                return;
                            }
                            let client = view.client.clone();
                            view.run(
                                async move {
                                    client
                                        .change_organization(OrganizationChange::CreateFolder {
                                            name,
                                        })
                                        .await
                                        .map(|_| Effect::None)
                                },
                                cx,
                            );
                        })),
                ),
        );
        let selected_folder = match &self.library_filter {
            LibraryFilter::Folder(id) => Some(id.clone()),
            _ => None,
        };
        let selected_skills: Vec<_> = self.selected_skills.iter().cloned().collect();
        let mut folder_row = div()
            .flex()
            .gap_2()
            .child(
                Button::new(
                    "folder-all",
                    format!("All ({})", library.library.skills.len()),
                )
                .on_click(cx.listener(|view, _, _, cx| {
                    view.library_filter = LibraryFilter::All;
                    view.selected_skills.clear();
                    view.selected_skill = None;
                    view.preview = None;
                    cx.notify();
                })),
            )
            .child(
                Button::new(
                    "folder-unfiled",
                    format!(
                        "Unfiled ({})",
                        library
                            .library
                            .skills
                            .iter()
                            .filter(|skill| skill.folder_id.is_none())
                            .count()
                    ),
                )
                .on_click(cx.listener(|view, _, _, cx| {
                    view.library_filter = LibraryFilter::Unfiled;
                    view.selected_skills.clear();
                    view.selected_skill = None;
                    view.preview = None;
                    cx.notify();
                })),
            );
        for folder in &folders {
            let id = folder.id.clone();
            folder_row = folder_row.child(
                Button::new(
                    format!("folder-{id}"),
                    format!(
                        "{} ({})",
                        folder.name,
                        library
                            .library
                            .skills
                            .iter()
                            .filter(|skill| skill.folder_id.as_ref() == Some(&id))
                            .count()
                    ),
                )
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.library_filter = LibraryFilter::Folder(id.clone());
                    view.selected_skills.clear();
                    view.selected_skill = None;
                    view.preview = None;
                    cx.notify();
                })),
            );
        }
        page = page.child(folder_row);
        if let Some(folder_id) = &selected_folder {
            let folder_id = folder_id.clone();
            page = page.child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("rename-folder", "Rename selected folder")
                            .disabled(self.busy)
                            .on_click(cx.listener({
                                let folder_id = folder_id.clone();
                                move |view, _, _, cx| {
                                    let name = Self::input_text(&view.folder_name, cx);
                                    if name.is_empty() {
                                        return;
                                    }
                                    let client = view.client.clone();
                                    let id = folder_id.clone();
                                    view.run(
                                        async move {
                                            client
                                                .change_organization(
                                                    OrganizationChange::RenameFolder { id, name },
                                                )
                                                .await
                                                .map(|_| Effect::None)
                                        },
                                        cx,
                                    );
                                }
                            })),
                    )
                    .child(
                        Button::new("delete-folder", "Delete selected folder")
                            .disabled(self.busy)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let client = view.client.clone();
                                let id = folder_id.clone();
                                view.run(
                                    async move {
                                        client.preview_delete(id).await.map(Effect::DeletePreview)
                                    },
                                    cx,
                                );
                            })),
                    ),
            );
        }
        if !selected_skills.is_empty() {
            let mut assignments = div()
                .flex()
                .gap_2()
                .child(format!("{} selected", selected_skills.len()));
            let client = self.client.clone();
            let ids = selected_skills.clone();
            assignments = assignments.child(
                Button::new("assign-unfiled", "Move to Unfiled")
                    .disabled(self.busy)
                    .on_click(cx.listener(move |view, _, _, cx| {
                        let client = client.clone();
                        let skills = ids.clone();
                        view.run(
                            async move {
                                client
                                    .change_organization(OrganizationChange::Assign {
                                        skills,
                                        folder: None,
                                    })
                                    .await
                                    .map(|_| Effect::None)
                            },
                            cx,
                        );
                    })),
            );
            for folder in &folders {
                let id = folder.id.clone();
                let ids = selected_skills.clone();
                assignments = assignments.child(
                    Button::new(format!("assign-{id}"), format!("Move to {}", folder.name))
                        .disabled(self.busy)
                        .on_click(cx.listener(move |view, _, _, cx| {
                            let client = view.client.clone();
                            let skills = ids.clone();
                            let folder = Some(id.clone());
                            view.run(
                                async move {
                                    client
                                        .change_organization(OrganizationChange::Assign {
                                            skills,
                                            folder,
                                        })
                                        .await
                                        .map(|_| Effect::None)
                                },
                                cx,
                            );
                        })),
                );
            }
            page = page.child(assignments);
        }
        let visible: Vec<_> = library
            .library
            .skills
            .iter()
            .filter(|skill| {
                (match &self.library_filter {
                    LibraryFilter::All => true,
                    LibraryFilter::Unfiled => skill.folder_id.is_none(),
                    LibraryFilter::Folder(id) => skill.folder_id.as_ref() == Some(id),
                }) && (query.is_empty()
                    || skill.slug.to_lowercase().contains(&query)
                    || skill
                        .display_name
                        .as_ref()
                        .is_some_and(|name| name.to_lowercase().contains(&query))
                    || library
                        .details
                        .get(&skill.id)
                        .is_some_and(|details| details.description.to_lowercase().contains(&query)))
            })
            .collect();
        page = page.child(format!("{} skills", visible.len()));
        for skill in visible {
            let id = skill.id.clone();
            let checked = self.selected_skills.contains(&id);
            let details = library.details.get(&id);
            page = page.child(
                div()
                    .p_3()
                    .border_1()
                    .border_color(cx.theme().colors.border)
                    .flex()
                    .gap_2()
                    .child(
                        Checkbox::new(format!("skill-select-{id}"), checked)
                            .label("Select")
                            .on_change({
                                let weak = cx.entity().downgrade();
                                let id = id.clone();
                                move |checked, _, cx| {
                                    let _ = weak.update(cx, |view, cx| {
                                        if checked {
                                            view.selected_skills.insert(id.clone());
                                        } else {
                                            view.selected_skills.remove(&id);
                                        }
                                        cx.notify();
                                    });
                                }
                            }),
                    )
                    .child(
                        Button::new(
                            format!("skill-open-{id}"),
                            skill
                                .display_name
                                .clone()
                                .unwrap_or_else(|| skill.slug.clone()),
                        )
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.selected_skill = Some(id.clone());
                            let client = view.client.clone();
                            let skill = id.clone();
                            view.run(
                                async move {
                                    client.preview_skill(skill, None).await.map(Effect::Preview)
                                },
                                cx,
                            );
                        })),
                    )
                    .child(details.map(|d| d.description.clone()).unwrap_or_default()),
            );
        }
        if let Some(skill_id) = &self.selected_skill
            && let Some(skill) = library
                .library
                .skills
                .iter()
                .find(|skill| &skill.id == skill_id)
        {
            let mut detail = div().p_4().flex().flex_col().gap_2().child(format!(
                "{} · {} files · {} bytes",
                skill.slug, skill.file_count, skill.total_bytes
            ));
            if let Some(info) = library.details.get(skill_id) {
                detail = detail
                    .child(info.description.clone())
                    .child(format!("Validation: {:?}", info.validation))
                    .children(
                        info.messages
                            .iter()
                            .map(|message| div().child(message.message.clone())),
                    )
                    .children(
                        info.sources
                            .iter()
                            .map(|source| div().child(source.clone())),
                    );
            }
            if let Some(preview) = &self.preview
                && &preview.skill_id == skill_id
            {
                for entry in &preview.entries {
                    let id = skill_id.clone();
                    let path = entry.path.clone();
                    detail = detail.child(
                        Button::new(
                            format!("file-{}", entry.path),
                            format!(
                                "{} {}",
                                if entry.directory { "Folder" } else { "File" },
                                entry.path
                            ),
                        )
                        .disabled(entry.directory || self.busy)
                        .on_click(cx.listener(move |view, _, _, cx| {
                            let client = view.client.clone();
                            let id = id.clone();
                            let path = Some(path.clone());
                            view.run(
                                            async move {
                                                client
                                                    .preview_skill(id, path)
                                                    .await
                                                    .map(Effect::Preview)
                                            },
                                            cx,
                                        );
                        })),
                    );
                }
                if let Some(text) = &preview.text {
                    detail = detail.child(text.clone());
                }
                if let Some(reason) = &preview.unavailable {
                    detail = detail.child(reason.clone());
                }
            }
            page = page.child(detail);
        }
        let mut conflicts = std::collections::BTreeMap::<String, Vec<SkillId>>::new();
        for skill in &library.library.skills {
            conflicts
                .entry(skill.slug.clone())
                .or_default()
                .push(skill.id.clone());
        }
        for (slug, ids) in conflicts.into_iter().filter(|(_, ids)| ids.len() > 1) {
            let revision = library.revision.clone();
            page = page.child(
                Button::new(
                    format!("conflict-{slug}"),
                    format!("Resolve shared name: {slug}"),
                )
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.dialog = DialogState::Conflict {
                        slug: slug.clone(),
                        ids: ids.clone(),
                        revision: revision.clone(),
                        selected: ids.first().cloned(),
                    };
                    cx.notify();
                })),
            );
        }
        page
    }
}
