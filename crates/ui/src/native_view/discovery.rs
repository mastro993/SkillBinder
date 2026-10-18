//! Discovery: the found skills that are not in library, one row per skill with its versions, and
//! the import of the chosen ones.

use super::controls::{Inactive, app_icon, button, checkbox};
use super::discovery_versions::{FoundSkill, found_skills, importable};
use super::library_dates as dates;
use super::{Effect, NativeView};
use chrono::Utc;
use gpui_kit::component::{
    ActiveTheme, Sizable, button::ButtonVariants, h_flex, radio::Radio, tag::Tag, v_flex,
};
use gpui_kit::{AnyElement, Context, IntoElement, ParentElement, Styled, div};
use skillbinder_proto::*;

impl NativeView {
    pub(super) fn discovery_header_actions(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        vec![
            button("scan-start")
                .label("Rescan")
                .small()
                .inactive(self.busy || self.snapshot.scanning)
                .on_click(cx.listener(|view, _, _, cx| {
                    let client = view.client.clone();
                    view.run(
                        async move { client.start_scan().await.map(|_| Effect::None) },
                        cx,
                    );
                }))
                .into_any_element(),
        ]
    }

    /// Found skills that are not in library, which Discovery lists.
    pub(super) fn not_in_library(&self) -> Vec<&Candidate> {
        self.snapshot.scan.as_ref().map_or_else(Vec::new, |scan| {
            scan.candidates
                .iter()
                .filter(|candidate| candidate.in_library().is_none())
                .collect()
        })
    }

    /// The versions the selected found skills import.
    fn chosen_versions<'a>(&self, skills: &[FoundSkill<'a>]) -> Vec<&'a Candidate> {
        skills
            .iter()
            .filter(|skill| self.selected_skills.contains(skill.key))
            .filter_map(|skill| skill.chosen(&self.version_choices))
            .collect()
    }

    pub(super) fn render_discovery(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let scan = self.snapshot.scan.as_ref();
        let finished = scan.is_some_and(|scan| scan.error.is_none());
        let listed = self.not_in_library();
        let skills = found_skills(&listed);
        let chosen = self.chosen_versions(&skills);
        let has_invalid = chosen
            .iter()
            .any(|version| version.validation == ValidationStatus::Invalid);
        let mut page = div()
            .flex()
            .flex_col()
            .gap_4()
            .child("Skills found in your agents and workspaces that are not in your library.");
        if self.snapshot.scanning {
            page = page.child("Scanning…");
        }
        let Some(scan) = scan else {
            return page;
        };
        let held: Vec<&Candidate> = scan
            .candidates
            .iter()
            .filter(|candidate| candidate.in_library().is_some())
            .collect();
        let in_library = found_skills(&held).len();
        page = page.child(format!(
            "{} found · {in_library} already in library",
            skills.len()
        ));
        if let Some(error) = &scan.error {
            page = page.child(error.clone());
        }
        let keys: Vec<String> = skills
            .iter()
            .filter(|skill| skill.chosen(&self.version_choices).is_some())
            .map(|skill| skill.key.to_owned())
            .collect();
        if finished && !keys.is_empty() {
            let all = keys.iter().all(|key| self.selected_skills.contains(key));
            let weak = cx.entity().downgrade();
            page = page.child(
                checkbox("select-discovery-all")
                    .checked(all)
                    .label("Select all")
                    .on_change(move |checked, _, cx| {
                        let _ = weak.update(cx, |view, cx| {
                            for key in &keys {
                                if *checked {
                                    view.selected_skills.insert(key.clone());
                                } else {
                                    view.selected_skills.remove(key);
                                }
                            }
                            cx.notify();
                        });
                    }),
            );
        }
        for skill in &skills {
            page = page.child(self.found_skill_row(skill, finished, cx));
        }
        if has_invalid {
            let weak = cx.entity().downgrade();
            page = page.child(
                checkbox("acknowledge-invalid")
                    .checked(self.acknowledge_invalid)
                    .label("I understand the selected invalid skills may not work as expected")
                    .on_change(move |checked, _, cx| {
                        let _ = weak.update(cx, |view, cx| {
                            view.acknowledge_invalid = *checked;
                            cx.notify();
                        });
                    }),
            );
        }
        let acknowledge = self.acknowledge_invalid;
        let ids: Vec<CandidateId> = chosen.iter().map(|version| version.id.clone()).collect();
        let too_many = ids.len() > MAX_IMPORT;
        if too_many {
            page = page.child(format!(
                "{} skills selected. Import at most {MAX_IMPORT} at a time.",
                ids.len()
            ));
        }
        page.child(
            button("prepare-import")
                .label("Review import")
                .primary()
                .inactive(
                    self.busy
                        || !finished
                        || ids.is_empty()
                        || too_many
                        || (has_invalid && !acknowledge),
                )
                .on_click(cx.listener(move |view, _, _, cx| {
                    let client = view.client.clone();
                    let ids = ids.clone();
                    view.run(
                        async move {
                            client
                                .prepare_import(ids, acknowledge)
                                .await
                                .map(Effect::Plan)
                        },
                        cx,
                    );
                })),
        )
    }

    /// One found skill: its checkbox, the chosen version's details, and its versions when it has
    /// several and they are shown.
    fn found_skill_row(
        &self,
        skill: &FoundSkill<'_>,
        finished: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let key = skill.key.to_owned();
        let chosen = skill.chosen(&self.version_choices);
        let shown = chosen.or_else(|| skill.versions.first().copied());
        let weak = cx.entity().downgrade();
        let selected = {
            let key = key.clone();
            checkbox(format!("skill-{key}"))
                .checked(self.selected_skills.contains(&key))
                .label(skill.slug(&self.version_choices).to_owned())
                .inactive(!finished || chosen.is_none())
                .on_change(move |checked, _, cx| {
                    let _ = weak.update(cx, |view, cx| {
                        if *checked {
                            view.selected_skills.insert(key.clone());
                        } else {
                            view.selected_skills.remove(&key);
                        }
                        cx.notify();
                    });
                })
        };
        let mut row = v_flex()
            .p_3()
            .gap_1()
            .border_1()
            .border_color(cx.theme().colors.border)
            .child(selected)
            .children(shown.map(|version| version.description.clone()));
        if skill.versions.len() == 1 {
            return row
                .children(shown.map(|version| version_details(version, cx)))
                .into_any_element();
        }
        let expanded = self.expanded_skills.contains(skill.key);
        let toggle = button(format!("versions-{key}"))
            .label(format!("{} versions", skill.versions.len()))
            .icon(app_icon(if expanded {
                "arrow-down"
            } else {
                "chevron-right"
            }))
            .ghost()
            .small()
            .on_click(cx.listener(move |view, _, _, cx| {
                if !view.expanded_skills.remove(&key) {
                    view.expanded_skills.insert(key.clone());
                }
                cx.notify();
            }));
        row = row.child(h_flex().child(toggle));
        if expanded {
            for &version in &skill.versions {
                row = row.child(self.version_choice(skill.key, version, chosen, finished, cx));
            }
        }
        row.into_any_element()
    }

    /// A radio choosing `version` as the one its skill imports, with its details.
    fn version_choice(
        &self,
        key: &str,
        version: &Candidate,
        chosen: Option<&Candidate>,
        finished: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let weak = cx.entity().downgrade();
        let (key, id) = (key.to_owned(), version.id.clone());
        let modified = version
            .modified
            .and_then(dates::from_unix)
            .map(|time| format!("Modified {}", dates::relative(time, Utc::now())));
        let summary = [
            Some(modified.unwrap_or_else(|| "Modified at an unknown time".to_owned())),
            (version.validation != ValidationStatus::Valid)
                .then(|| format!("{:?}", version.validation)),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
        let radio = Radio::new(format!("version-{}", version.id))
            .checked(chosen.is_some_and(|chosen| chosen.id == version.id))
            .label(summary)
            .disabled(!finished || !importable(version))
            .on_click(move |_, _, cx| {
                let _ = weak.update(cx, |view, cx| {
                    view.version_choices.insert(key.clone(), id.clone());
                    cx.notify();
                });
            });
        v_flex()
            .pl_4()
            .gap_1()
            .child(
                h_flex().gap_2().child(radio).children(
                    version
                        .matches_source
                        .then(|| Tag::secondary().small().child("Matches source")),
                ),
            )
            .child(div().pl_6().child(version_details(version, cx)))
            .into_any_element()
    }
}

/// A version's locations with their readers, size, and validation messages.
fn version_details(version: &Candidate, cx: &Context<NativeView>) -> AnyElement {
    v_flex()
        .gap_1()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .children(version.locations.iter().map(|location| {
            div().child(format!(
                "{} · {}",
                location.display_path,
                location.readers.join(", ")
            ))
        }))
        .child(format!(
            "{} files · {} bytes",
            version.file_count, version.total_bytes
        ))
        .children(
            version
                .messages
                .iter()
                .map(|message| div().child(format!("{:?}: {}", message.status, message.message))),
        )
        .into_any_element()
}
