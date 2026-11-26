//! The dialog's list (D3 to D7, I10): Select all, or "No new skills found." when every found
//! skill is ignored, then each place's group header with its tri-state checkbox, glyph, name,
//! count, and Ignore all, which onboarding's list leaves out (O7), then its rows, then the Ignored
//! group. A row is ticked with its checkbox or a click anywhere on it outside its versions.

use super::super::NativeView;
use super::super::library_updates::spinner;
use super::ImportDialog;
use super::list::{Count, Group, Place, Row, groups, ignored_rows, rows, tick_all, tickable};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, tag::Tag};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::{opacity, space, text};
use skillbinder_design::{
    CardList, Checkbox, EmptyState, IconSize, ListRow, Notice, SectionHeader, Text,
};
use skillbinder_proto::{FoundImport, ValidationStatus};
use std::collections::BTreeSet;

impl NativeView {
    /// The bordered, scrolling list of the rows the search matches.
    pub(super) fn import_list(
        &self,
        state: &ImportDialog,
        found: &FoundImport,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let search = state.search.read(cx).value().trim().to_lowercase();
        let snapshot = self.snapshot.clone();
        let groups = groups(found, &snapshot.workspaces, &snapshot.ignored);
        let ignored = ignored_rows(found, &snapshot.ignored);
        let ticked = state.ticked.clone().unwrap_or_default();
        let head = if groups.is_empty() {
            h_flex()
                .px(px(space::LG))
                .py(px(space::MD))
                .child(Notice::new("import-nothing", "No new skills found."))
                .into_any_element()
        } else {
            self.select_all(&groups, &ticked, &search, cx)
        };
        let mut list = CardList::new("import-list")
            .flex_1()
            .min_h_0()
            .scrolls()
            .when(state.refreshing(), |list| list.opacity(opacity::INACTIVE))
            .child(head);
        let mut matched = false;
        for (index, group) in groups.iter().enumerate() {
            let shown: Vec<&Row<'_>> = group
                .rows
                .iter()
                .filter(|row| row.matches(&search))
                .collect();
            if shown.is_empty() {
                continue;
            }
            matched = true;
            if state.end.names_places() {
                list = list.child(self.group_head(index, group, &ticked, cx));
            }
            for row in shown {
                list = list.row(self.import_row(state, row, &ticked, cx));
            }
        }
        let ignored_matched = ignored.iter().any(|row| row.matches(&search));
        list = self.ignored_group(list, state, &ignored, &search, cx);
        if !search.is_empty() && !matched && !ignored_matched {
            let title = format!("No skills match '{search}'");
            list = list.child(EmptyState::new("import-empty", title).icon("search"));
        }
        list.into_any_element()
    }

    /// Select all, with how many ticked skills the search hides.
    fn select_all(
        &self,
        groups: &[Group<'_>],
        ticked: &BTreeSet<String>,
        search: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let count = Count::of(ticked, rows(groups));
        let hidden = rows(groups)
            .filter(|row| row.tickable() && ticked.contains(row.key()) && !row.matches(search))
            .count();
        let keys = tickable(rows(groups));
        h_flex()
            .id("import-select-all")
            .px(px(space::LG))
            .py(px(space::MD))
            .gap(px(space::LG))
            .child(
                self.ticking("import-check-all", count, &keys, cx)
                    .label("Select all"),
            )
            .when(hidden > 0, |head| {
                head.child(
                    Text::labeled("hidden", format!("{hidden} selected not shown"))
                        .level(text::CAPTION)
                        .muted(),
                )
            })
            .into_any_element()
    }

    /// A group's header: its checkbox with the place's glyph, name, and count of skills.
    fn group_head(
        &self,
        index: usize,
        group: &Group<'_>,
        ticked: &BTreeSet<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let icon = match &group.place {
            Place::Agents => "robot",
            Place::Workspace(_) => "code-folder",
        };
        let name = group.place.name(&self.snapshot.workspaces);
        let count = Count::of(ticked, &group.rows);
        let keys = tickable(&group.rows);
        let theme = cx.theme();
        let label = SectionHeader::custom(
            h_flex()
                .gap(px(space::SM))
                .child(skillbinder_design::icon(icon).with_size(IconSize::Sm))
                .child(Text::new(name)),
        )
        .count(group.rows.len());
        h_flex()
            .id(format!("import-group-{index}"))
            .justify_between()
            .gap(px(space::LG))
            .px(px(space::LG))
            .py(px(space::XS))
            .bg(theme.muted)
            .child(
                self.ticking(format!("import-check-group-{index}"), count, &keys, cx)
                    .child(label),
            )
            .child(self.ignore_all_button(index, group, cx))
            .into_any_element()
    }

    /// A checkbox ticking or unticking the skills `keys`: ticked once all of them are, partly
    /// ticked while only some are, when a click ticks them all.
    fn ticking(
        &self,
        id: impl Into<gpui_kit::ElementId>,
        count: Count,
        keys: &[String],
        cx: &mut Context<Self>,
    ) -> Checkbox {
        let keys = keys.to_vec();
        let weak = cx.entity().downgrade();
        Checkbox::new(id)
            .checked(count.full())
            .indeterminate(count.partial())
            .inactive(count.able == 0)
            .on_change(move |on, _, cx| {
                let on = *on;
                let _ = weak.update(cx, |view, cx| {
                    view.edit_import(|dialog| tick_all(ticks(dialog), &keys, on), cx);
                });
            })
    }

    /// One found skill: its checkbox, slug, tags, description, and versions or details.
    fn import_row(
        &self,
        state: &ImportDialog,
        row: &Row<'_>,
        ticked: &BTreeSet<String>,
        cx: &mut Context<Self>,
    ) -> ListRow {
        let key = row.key().to_owned();
        let able = row.tickable();
        let shown = row.shown();
        let slug = shown.map_or(key.clone(), |version| version.slug.clone());
        let weak = cx.entity().downgrade();
        let toggled = key.clone();
        let check = Checkbox::new(format!("import-check-{key}"))
            .checked(ticked.contains(&key))
            .inactive(!able)
            .accessibility_label(slug.clone())
            .on_change(move |_, _, cx| {
                let key = toggled.clone();
                let _ = weak.update(cx, |view, cx| view.toggle_import(key, cx));
            });
        let versions = self.row_versions(state, row, cx);
        let blocked = shown.is_some_and(|v| v.validation == ValidationStatus::Blocked);
        let title = h_flex()
            .gap(px(space::SM))
            .items_center()
            .child(Text::labeled("slug", slug))
            .when(row.invalid(), |title| {
                title.child(Tag::warning().small().child("Invalid"))
            })
            .when(blocked, |title| {
                title.child(Tag::secondary().small().child("Blocked"))
            })
            .when(state.choosing.as_deref() == Some(key.as_str()), |title| {
                title.child(spinner())
            });
        let description = shown
            .map(|version| version.description.clone())
            .unwrap_or_default();
        let clicked = key.clone();
        let row_element = ListRow::new(format!("import-row-{key}"))
            .items_start()
            .leading(
                div()
                    .id(format!("import-check-box-{key}"))
                    .pt(px(space::XXS))
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(check),
            )
            .title(title)
            .when(!description.is_empty(), |row| row.subtitle(description))
            .child(versions)
            .trailing(self.ignore_button(row, cx))
            .dimmed(!able)
            .when(able, |row| {
                row.cursor_pointer()
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.toggle_import(clicked.clone(), cx);
                    }))
            });
        self.offered_row(row_element, &key, state, cx)
    }

    fn toggle_import(&mut self, key: String, cx: &mut Context<Self>) {
        self.edit_import(
            |dialog| {
                let ticked = ticks(dialog);
                if !ticked.remove(&key) {
                    ticked.insert(key);
                }
            },
            cx,
        );
    }

    /// Runs `edit` on the open Import skills dialog.
    pub(super) fn edit_import(
        &mut self,
        edit: impl FnOnce(&mut ImportDialog),
        cx: &mut Context<Self>,
    ) {
        if let Some(dialog) = self.import_dialog_mut() {
            edit(dialog);
        }
        cx.notify();
    }
}

/// The dialog's ticks, which a shown list always has.
fn ticks(dialog: &mut ImportDialog) -> &mut BTreeSet<String> {
    dialog.ticked.get_or_insert_with(BTreeSet::new)
}
