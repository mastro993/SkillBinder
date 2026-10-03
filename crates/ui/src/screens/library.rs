use crate::state::Operation;
use crate::{
    components::{review::Review, *},
    state::conflict_groups,
    workspace::Workspace,
};
use gpui::{prelude::*, *};
use gpui_component::Disableable;

impl Workspace {
    pub(crate) fn library(&mut self, _: &mut Window, cx: &mut Context<Self>) -> Div {
        let mut content = column()
            .child(heading("Library", "Canonical collection", cx))
            .child(
                row()
                    .child(button("new-skill", "New skill").disabled(true))
                    .child(
                        button("refresh-library", "Refresh")
                            .disabled(self.state.pending.contains(&Operation::Library))
                            .on_click(cx.listener(|s, _, w, cx| s.load_library(w, cx))),
                    ),
            );
        let Some(data) = &self.state.library else {
            return content.child(loading(cx));
        };
        if data.has_uncommitted_changes {
            content = content.child(notice("Library changes are not committed yet.", cx));
        }
        if data.pending_resolution {
            content = content.child(notice(
                "A previous resolution was interrupted; the next attempt finishes it.",
                cx,
            ));
        }
        for (slug, skills) in conflict_groups(&data.skills) {
            content = content.child(
                panel(cx)
                    .child(format!("{slug} · {} copies share this slug", skills.len()))
                    .child(
                        button(
                            SharedString::from(format!("resolve-slug-{}", slug)),
                            "Choose which to keep",
                        )
                        .disabled(self.busy())
                        .on_click(cx.listener(move |s, _, w, cx| {
                            Review::open_conflict(
                                skills.clone(),
                                s.service.clone(),
                                cx.weak_entity(),
                                w,
                                cx,
                            );
                        })),
                    ),
            );
        }
        if data.skills.is_empty() {
            return content.child(notice("Your library is ready. No skills imported yet. Visit Discovery to inspect local skill folders.", cx));
        }
        content.children(data.skills.iter().map(|skill| {
            panel(cx)
                .child(
                    div().text_lg().font_weight(FontWeight::SEMIBOLD).child(
                        skill
                            .display_name
                            .clone()
                            .unwrap_or_else(|| skill.slug.clone()),
                    ),
                )
                .child(muted(
                    format!("{} · {}", skill.slug, skill.payload_directory),
                    cx,
                ))
                .child(muted(
                    skill
                        .description
                        .clone()
                        .unwrap_or_else(|| "No description".into()),
                    cx,
                ))
                .child(validation(&skill.validation, cx))
                .child(muted(
                    format!("{} files · {} bytes", skill.file_count, skill.total_bytes),
                    cx,
                ))
                .children(skill.sources.iter().map(|source| {
                    column()
                        .gap_1()
                        .child(muted(source.display_path.clone(), cx))
                        .child(muted(source.reader_agent_ids.join(", "), cx))
                }))
        }))
    }
}
