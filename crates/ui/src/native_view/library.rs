use super::controls::{Inactive, button};
use super::{DialogState, Effect, LibraryFilter, NativeView};
use gpui_kit::component::{ActiveTheme, checkbox::Checkbox, input::Input};
use gpui_kit::{Context, IntoElement, ParentElement, Styled, div};
use skillbinder_proto::*;

impl NativeView {
    pub(super) fn render_library(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let Some(library) = &self.snapshot.library else {
            return div().child("Finish onboarding to open your library.");
        };
        let title = match self.library_filter() {
            LibraryFilter::All => "Library".to_owned(),
            LibraryFilter::Folder(id) => library
                .library
                .folders
                .iter()
                .find(|folder| &folder.id == id)
                .map_or_else(|| "Library".to_owned(), |folder| folder.name.clone()),
        };
        let mut page = div()
            .flex()
            .flex_col()
            .gap_4()
            .child(title)
            .child(Input::new(&self.search));
        let query = self.search.read(cx).value().trim().to_lowercase();
        let folders = &library.library.folders;
        let selected_skills: Vec<_> = self.selected_skills.iter().cloned().collect();
        if !selected_skills.is_empty() {
            let mut assignments = div()
                .flex()
                .gap_2()
                .child(format!("{} selected", selected_skills.len()));
            let client = self.client.clone();
            let ids = selected_skills.clone();
            assignments = assignments.child(
                button("assign-none")
                    .label("Remove from folder")
                    .inactive(self.busy)
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
            for folder in folders {
                let id = folder.id.clone();
                let ids = selected_skills.clone();
                assignments = assignments.child(
                    button(format!("assign-{id}"))
                        .label(format!("Move to {}", folder.name))
                        .inactive(self.busy)
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
                (match self.library_filter() {
                    LibraryFilter::All => true,
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
                        Checkbox::new(format!("skill-select-{id}"))
                            .checked(checked)
                            .label("Select")
                            .on_change({
                                let weak = cx.entity().downgrade();
                                let id = id.clone();
                                move |checked, _, cx| {
                                    let _ = weak.update(cx, |view, cx| {
                                        if *checked {
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
                        button(format!("skill-open-{id}"))
                            .label(
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
                    );
            }
            let targets = self.deployment_targets(skill_id);
            detail = detail.child(if targets.is_empty() {
                "Not deployed".to_owned()
            } else {
                format!("Deployed to {}", targets.join(", "))
            });
            if let Some(preview) = &self.preview
                && &preview.skill_id == skill_id
            {
                for entry in &preview.entries {
                    let id = skill_id.clone();
                    let path = entry.path.clone();
                    detail = detail.child(
                        button(format!("file-{}", entry.path))
                            .label(format!(
                                "{} {}",
                                if entry.directory { "Folder" } else { "File" },
                                entry.path
                            ))
                            .inactive(entry.directory || self.busy)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                let client = view.client.clone();
                                let id = id.clone();
                                let path = Some(path.clone());
                                view.run(
                                    async move {
                                        client.preview_skill(id, path).await.map(Effect::Preview)
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
                button(format!("conflict-{slug}"))
                    .label(format!("Resolve shared name: {slug}"))
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
