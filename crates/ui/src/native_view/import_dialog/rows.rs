//! The dialog's list (D3 to D7): Select all, then each place's group header with its tri-state
//! checkbox, glyph, name, and count, which onboarding's list leaves out (O7), then its rows. A
//! row is ticked with its checkbox or a click anywhere on it outside its versions.

use super::super::NativeView;
use super::super::controls::{Inactive, app_icon, checkbox};
use super::super::library_filters::{empty_state, glyph};
use super::ImportDialog;
use super::list::{Count, Group, Place, Row, groups, rows, tick_all, tickable};
use gpui_kit::component::{
    ActiveTheme, Sizable, h_flex, scroll::ScrollableElement, tag::Tag, v_flex,
};
use gpui_kit::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Text, div, prelude::FluentBuilder, px,
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
        let groups = groups(found, &self.snapshot.workspaces);
        let ticked = state.ticked.clone().unwrap_or_default();
        let mut list = v_flex().child(self.select_all(&groups, &ticked, &search, cx));
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
                list = list.child(self.import_row(state, row, &ticked, cx));
            }
        }
        if !matched {
            let title = format!("No skills match '{search}'");
            list = list.child(empty_state(glyph("search", cx), title, cx));
        }
        div()
            .id("import-list")
            .flex_1()
            .min_h_0()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(px(10.))
            .when(state.refreshing(), |list| list.opacity(0.6))
            .child(list)
            .overflow_y_scrollbar()
            .into_any_element()
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
            .px(px(12.))
            .py(px(8.))
            .gap(px(10.))
            .child(
                self.ticking("import-check-all", count, &keys, cx)
                    .label("Select all"),
            )
            .when(hidden > 0, |head| {
                head.child(
                    div()
                        .text_size(px(12.))
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{hidden} selected not shown")),
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
        let (icon, name) = match &group.place {
            Place::Agents => ("robot", "Your agents".to_owned()),
            Place::Workspace(id) => (
                "code-folder",
                self.workspace(id)
                    .map_or_else(|| "Workspace".to_owned(), |w| w.name.clone()),
            ),
        };
        let count = Count::of(ticked, &group.rows);
        let keys = tickable(&group.rows);
        let theme = cx.theme();
        let label = h_flex()
            .gap(px(6.))
            .items_center()
            .child(app_icon(icon).size(px(14.)))
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(Text::new("name".into(), name.into())),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.muted_foreground)
                    .child(group.rows.len().to_string()),
            );
        h_flex()
            .id(format!("import-group-{index}"))
            .px(px(12.))
            .py(px(8.))
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.muted)
            .child(
                self.ticking(format!("import-check-group-{index}"), count, &keys, cx)
                    .child(label),
            )
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
    ) -> gpui_kit::component::checkbox::Checkbox {
        let keys = keys.to_vec();
        let weak = cx.entity().downgrade();
        checkbox(id)
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
    ) -> AnyElement {
        let key = row.key().to_owned();
        let able = row.tickable();
        let shown = row.shown();
        let slug = shown.map_or(key.clone(), |version| version.slug.clone());
        let weak = cx.entity().downgrade();
        let toggled = key.clone();
        let check = checkbox(format!("import-check-{key}"))
            .checked(ticked.contains(&key))
            .inactive(!able)
            .accessibility_label(slug.clone())
            .on_change(move |_, _, cx| {
                let key = toggled.clone();
                let _ = weak.update(cx, |view, cx| view.toggle_import(key, cx));
            });
        let versions = self.row_versions(state, row, cx);
        let theme = cx.theme();
        let blocked = shown.is_some_and(|v| v.validation == ValidationStatus::Blocked);
        let title = h_flex()
            .gap(px(6.))
            .items_center()
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(Text::new("slug".into(), slug.into())),
            )
            .when(row.invalid(), |title| {
                title.child(Tag::warning().small().child("Invalid"))
            })
            .when(blocked, |title| {
                title.child(Tag::secondary().small().child("Blocked"))
            })
            .when(state.choosing.as_deref() == Some(key.as_str()), |title| {
                title.child(Self::spinner("import-choosing"))
            });
        let description = shown
            .map(|version| version.description.clone())
            .unwrap_or_default();
        let main = v_flex()
            .min_w_0()
            .flex_1()
            .gap(px(4.))
            .child(title)
            .when(!description.is_empty(), |main| {
                main.child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.muted_foreground)
                        .truncate()
                        .child(description),
                )
            })
            .child(versions);
        let clicked = key.clone();
        h_flex()
            .id(format!("import-row-{key}"))
            .items_start()
            .gap(px(10.))
            .px(px(12.))
            .py(px(10.))
            .border_t_1()
            .border_color(theme.border)
            .when(!able, |row| row.opacity(0.6))
            .when(able, |row| {
                row.cursor_pointer()
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.toggle_import(clicked.clone(), cx);
                    }))
            })
            .child(
                div()
                    .id(format!("import-check-box-{key}"))
                    .pt(px(2.))
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(check),
            )
            .child(main)
            .into_any_element()
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
