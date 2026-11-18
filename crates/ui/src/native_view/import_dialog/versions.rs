//! Found skills: the not-in-library candidates grouped into one skill per key, each with its
//! versions, and how a version reads in a list.

use super::super::NativeView;
use super::super::import_review::{place_name, replaced_in};
use super::super::library_copies::shown_dir;
use super::super::library_dates as dates;
use super::ImportDialog;
use super::list::Row;
use chrono::Utc;
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, radio::Radio, tag::Tag, v_flex};
use gpui_kit::{
    AnyElement, App, Context, Hsla, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::color;
use skillbinder_design::{Button, IconSize, icon};
use skillbinder_proto::{AppSnapshot, Candidate, CandidateLocation, ValidationStatus};
use std::{collections::BTreeMap, path::Path};

/// One found skill and its versions, in the order the scan lists them.
pub(in crate::native_view) struct FoundSkill<'a> {
    pub key: &'a str,
    pub versions: Vec<&'a Candidate>,
}

/// Whether a version can be imported: it is not blocked and has files.
fn importable(version: &Candidate) -> bool {
    version.validation != ValidationStatus::Blocked && version.file_count > 0
}

/// Groups `listed` by key, keeping the order of each key's first version.
pub(in crate::native_view) fn found_skills<'a>(listed: &[&'a Candidate]) -> Vec<FoundSkill<'a>> {
    let mut skills: Vec<FoundSkill<'a>> = Vec::new();
    let mut index: BTreeMap<&str, usize> = BTreeMap::new();
    for &candidate in listed {
        match index.get(candidate.key.as_str()) {
            Some(&at) => skills[at].versions.push(candidate),
            None => {
                index.insert(&candidate.key, skills.len());
                skills.push(FoundSkill {
                    key: &candidate.key,
                    versions: vec![candidate],
                });
            }
        }
    }
    skills
}

/// A version's radio label: when it was modified, then Invalid or Blocked (D4).
fn version_label(version: &Candidate) -> String {
    let modified = version
        .modified
        .and_then(dates::from_unix)
        .map(|time| format!("Modified {}", dates::relative(time, Utc::now())));
    let status = match version.validation {
        ValidationStatus::Valid | ValidationStatus::Warning => None,
        ValidationStatus::Invalid => Some("Invalid".to_owned()),
        ValidationStatus::Blocked => Some("Blocked".to_owned()),
    };
    [
        Some(modified.unwrap_or_else(|| "Modified at an unknown time".to_owned())),
        status,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ")
}

/// A version's locations as "directory · readers", its size, and its validation messages.
fn version_details(version: &Candidate, snapshot: &AppSnapshot, cx: &App) -> AnyElement {
    v_flex()
        .min_w_0()
        .gap(px(2.))
        .text_size(px(12.))
        .text_color(cx.theme().muted_foreground)
        .child(
            h_flex()
                .min_w_0()
                .flex_wrap()
                .gap_x(px(12.))
                .children(version.locations.iter().map(|location| {
                    div()
                        .min_w_0()
                        .truncate()
                        .child(location_label(location, snapshot))
                }))
                .child(format!(
                    "{} files · {} bytes",
                    version.file_count, version.total_bytes
                )),
        )
        .children(version.messages.iter().map(|message| {
            let color = if message.status == ValidationStatus::Blocked {
                color::DANGER_TEXT.of(cx.theme())
            } else {
                color::WARNING_TEXT.of(cx.theme())
            };
            warning(message.message.clone(), color)
        }))
        .into_any_element()
}

/// Where a version was found: its skill directory with home as `~`, then the Agents rows or the
/// workspace that read it, as the app names places.
fn location_label(location: &CandidateLocation, snapshot: &AppSnapshot) -> String {
    let directory = Path::new(&location.display_path)
        .parent()
        .map_or_else(String::new, |parent| parent.display().to_string());
    let shown = shown_dir(&directory, &snapshot.home);
    let place = place_name(
        &directory,
        &snapshot.agents,
        &snapshot.workspaces,
        &snapshot.home,
    );
    if place == shown {
        shown
    } else {
        format!("{shown} · {place}")
    }
}

/// A line of `text` after the alert glyph, both in `color`, wrapping beside the glyph.
fn warning(text: String, color: Hsla) -> AnyElement {
    h_flex()
        .items_start()
        .gap(px(6.))
        .text_size(px(12.))
        .text_color(color)
        .child(
            icon("alert-circle")
                .with_size(IconSize::Xs)
                .flex_none()
                .mt(px(2.)),
        )
        .child(div().min_w_0().flex_1().child(text))
        .into_any_element()
}

impl NativeView {
    /// A row's versions: one version's details and replacements, or the "N versions" disclosure
    /// that opens a radio per version.
    pub(super) fn row_versions(
        &self,
        state: &ImportDialog,
        row: &Row<'_>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let snapshot = &self.snapshot;
        let versions = &row.skill.versions;
        let shown = row.shown();
        let warn = color::WARNING_TEXT.of(cx.theme());
        let replaced = self.replacements(row);
        let replaced = || replaced.iter().map(move |line| warning(line.clone(), warn));
        if versions.len() == 1 {
            return v_flex()
                .gap(px(2.))
                .children(shown.map(|version| version_details(version, snapshot, cx)))
                .children(replaced())
                .into_any_element();
        }
        let key = row.key().to_owned();
        let expanded = state.expanded.contains(&key);
        let toggled = key.clone();
        let toggle = Button::new(format!("import-versions-{key}"))
            .label(format!("{} versions", versions.len()))
            .icon(icon(if expanded {
                "arrow-down"
            } else {
                "chevron-right"
            }))
            .ghost()
            .small()
            .on_click(cx.listener(move |view, _, _, cx| {
                cx.stop_propagation();
                let key = toggled.clone();
                view.edit_import(
                    |dialog| {
                        if !dialog.expanded.remove(&key) {
                            dialog.expanded.insert(key);
                        }
                    },
                    cx,
                );
            }));
        let mut list = v_flex()
            .id(format!("import-versions-list-{key}"))
            .gap(px(6.))
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(h_flex().child(toggle));
        if !expanded {
            let messages = shown.map_or(Vec::new(), |version| {
                let messages = version.messages.iter();
                messages.map(|m| warning(m.message.clone(), warn)).collect()
            });
            return list
                .children(messages)
                .children(replaced())
                .into_any_element();
        }
        let chosen = state
            .choices
            .get(&key)
            .or(row.item.map(|item| &item.candidate_id));
        let locked = state.choosing.is_some() || state.refreshing();
        for &version in versions {
            let weak = cx.entity().downgrade();
            let (key, id) = (key.clone(), version.id.clone());
            let radio = Radio::new(format!("import-version-{}", version.id))
                .checked(chosen == Some(&version.id))
                .label(version_label(version))
                .small()
                .disabled(locked || !importable(version))
                .on_click(move |_, _, cx| {
                    let (key, id) = (key.clone(), id.clone());
                    let _ = weak.update(cx, |view, cx| view.choose_import_version(key, id, cx));
                });
            let planned = row.item.is_some_and(|item| item.candidate_id == version.id);
            list = list.child(
                v_flex()
                    .gap(px(2.))
                    .child(
                        h_flex().gap(px(8.)).child(radio).children(
                            version
                                .matches_source
                                .then(|| Tag::secondary().small().child("Matches source")),
                        ),
                    )
                    .child(
                        v_flex()
                            .pl(px(24.))
                            .gap(px(2.))
                            .child(version_details(version, snapshot, cx))
                            .when(planned, |details| details.children(replaced())),
                    ),
            );
        }
        list.into_any_element()
    }

    /// "Replaces the copy in <place>." for each copy the planned version replaces.
    fn replacements(&self, row: &Row<'_>) -> Vec<String> {
        let snapshot = &self.snapshot;
        row.item.map_or(Vec::new(), |item| {
            item.replaces
                .iter()
                .map(|directory| {
                    let place = replaced_in(
                        directory,
                        &snapshot.agents,
                        &snapshot.workspaces,
                        &snapshot.home,
                    );
                    format!("Replaces the copy in {place}.")
                })
                .collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_proto::CandidateId;

    fn version(id: &str, key: &str, preferred: bool, validation: ValidationStatus) -> Candidate {
        Candidate {
            id: CandidateId(id.to_owned()),
            key: key.to_owned(),
            preferred,
            matches_source: false,
            modified: None,
            skill_id: None,
            held: None,
            slug: key.to_owned(),
            description: String::new(),
            locations: Vec::new(),
            validation,
            messages: Vec::new(),
            total_bytes: 1,
            file_count: 1,
        }
    }

    #[test]
    fn versions_group_by_key_in_the_order_the_scan_lists_them() {
        let a1 = version("a1", "a", false, ValidationStatus::Valid);
        let b = version("b", "b", true, ValidationStatus::Valid);
        let a2 = version("a2", "a", true, ValidationStatus::Valid);
        let skills = found_skills(&[&a1, &b, &a2]);
        let keys: Vec<_> = skills.iter().map(|skill| skill.key).collect();
        assert_eq!(keys, ["a", "b"]);
        let ids: Vec<_> = skills[0].versions.iter().map(|v| v.id.0.as_str()).collect();
        assert_eq!(ids, ["a1", "a2"]);
    }
}
