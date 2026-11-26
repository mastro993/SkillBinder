//! Ignoring found skills (ADR 0033, design I2 to I11): Ignore on a row while it is hovered or
//! focused, Ignore all on a place's header, and the Ignored group closing the list, collapsed,
//! whose rows have Stop ignoring. Each is saved at once, and Cancel keeps it.

use super::super::NativeView;
use super::super::import_review::place_name;
use super::ImportDialog;
use super::list::{Group, Row};
use gpui_kit::component::{ActiveTheme, h_flex};
use gpui_kit::{
    Animation, AnimationExt, AnyElement, App, Context, InteractiveElement, IntoElement,
    ParentElement, Styled, div, prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::{color, opacity, space, text};
use skillbinder_design::{Button, CardList, ListRow, SectionHeader, Text, icon};
use skillbinder_proto::MAX_IMPORT;
use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

/// The group a row's Ignore is revealed by.
const ROW_GROUP: &str = "import-row";

/// Numbers each flash, so every one plays its animation from the start.
static FLASHES: AtomicU64 = AtomicU64::new(0);

/// What flashes once, and which flash it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_view) struct Flash {
    /// A row's key, or `None` for the Ignored group's header.
    key: Option<String>,
    id: u64,
}

impl Flash {
    fn new(key: Option<String>) -> Self {
        Self {
            key,
            id: FLASHES.fetch_add(1, Ordering::Relaxed),
        }
    }
}

/// A slug the engine knows a row by: its shown version's spelling, else its key.
fn slug(row: &Row<'_>) -> String {
    row.shown()
        .map_or_else(|| row.key().to_owned(), |version| version.slug.clone())
}

impl NativeView {
    /// Ignores `rows`, dropping their ticks; the Ignored group's header flashes once it is saved.
    fn ignore_found(&mut self, rows: Vec<(String, String)>, cx: &mut Context<Self>) {
        let (slugs, keys): (Vec<String>, Vec<String>) = rows.into_iter().unzip();
        self.edit_import(
            |dialog| {
                if let Some(ticked) = dialog.ticked.as_mut() {
                    ticked.retain(|key| !keys.contains(key));
                }
            },
            cx,
        );
        let client = self.client.clone();
        let asked = async move { client.set_ignored(slugs, true).await };
        self.ask(asked, |view, result, cx| view.ignored(result, None, cx), cx);
    }

    /// Stops ignoring the skill `slug`, `key`, which comes back ticked when `tick` and the cap
    /// allows (I7), and flashes once it is saved.
    fn stop_ignoring(&mut self, slug: String, key: String, tick: bool, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let asked = async move { client.set_ignored(vec![slug], false).await };
        self.ask(
            asked,
            move |view, result, cx| {
                if result.is_ok() && tick {
                    view.edit_import(
                        |dialog| {
                            let ticked = dialog.ticked.get_or_insert_default();
                            if ticked.len() < MAX_IMPORT {
                                ticked.insert(key.clone());
                            }
                        },
                        cx,
                    );
                }
                view.ignored(result, Some(key), cx);
            },
            cx,
        );
    }

    /// Shows the snapshot an ignore or a stop saved, flashing the header or row `key`, or the
    /// reason it failed.
    fn ignored(
        &mut self,
        result: skillbinder_proto::AppResult<()>,
        key: Option<String>,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(()) => {
                self.receive(self.client.snapshot(), cx);
                self.edit_import(|dialog| dialog.flash = Some(Flash::new(key)), cx);
            }
            Err(error) => self.error = Some(error),
        }
        cx.notify();
    }

    /// A row's Ignore, shown while the row is hovered or the button has keyboard focus (I2).
    pub(super) fn ignore_button(&self, row: &Row<'_>, cx: &mut Context<Self>) -> Button {
        let (slug, key) = (slug(row), row.key().to_owned());
        Button::new(format!("import-ignore-{key}"))
            .label("Ignore")
            .ghost()
            .small()
            .accessibility_label(format!("Ignore {slug}"))
            .tooltip("Don't offer this skill again")
            .opacity(0.)
            .group_hover(ROW_GROUP, |style| style.opacity(1.))
            .focus_visible(|style| style.opacity(1.))
            .on_click(cx.listener(move |view, _, _, cx| {
                view.ignore_found(vec![(slug.clone(), key.clone())], cx);
            }))
    }

    /// A place's Ignore all, which ignores every skill the group lists, those the search hides
    /// and those that cannot be ticked included (I3).
    pub(super) fn ignore_all_button(
        &self,
        index: usize,
        group: &Group<'_>,
        cx: &mut Context<Self>,
    ) -> Button {
        let rows: Vec<(String, String)> = group
            .rows
            .iter()
            .map(|row| (slug(row), row.key().to_owned()))
            .collect();
        let tooltip = match rows.len() {
            1 => "Don't offer this skill again".to_owned(),
            count => format!("Don't offer these {count} skills again"),
        };
        Button::new(format!("import-ignore-group-{index}"))
            .label("Ignore all")
            .ghost()
            .small()
            .tooltip(tooltip)
            .on_click(cx.listener(move |view, _, _, cx| view.ignore_found(rows.clone(), cx)))
    }

    /// Ends `list` with the Ignored group, when any found skill is ignored: its header, then
    /// while it is open, or the search matches some, those of `rows` the search matches (I5, I9).
    pub(super) fn ignored_group(
        &self,
        mut list: CardList,
        state: &ImportDialog,
        rows: &[Row<'_>],
        search: &str,
        cx: &mut Context<Self>,
    ) -> CardList {
        if rows.is_empty() {
            return list;
        }
        let matched: Vec<&Row<'_>> = rows.iter().filter(|row| row.matches(search)).collect();
        let searching = !search.is_empty() && !matched.is_empty();
        let open = state.ignored_open || searching;
        let toggle = Button::new("import-ignored-toggle")
            .label("Ignored")
            .icon(icon(if open { "arrow-down" } else { "chevron-right" }))
            .ghost()
            .small()
            .on_click(cx.listener(|view, _, _, cx| {
                view.edit_import(|dialog| dialog.ignored_open = !dialog.ignored_open, cx);
            }));
        let flashing = state.flash.as_ref().filter(|flash| flash.key.is_none());
        let theme = cx.theme();
        let head = h_flex()
            .id("import-ignored")
            .relative()
            .px(px(space::LG))
            .py(px(space::XS))
            .gap(px(space::MD))
            .bg(theme.muted)
            .child(SectionHeader::custom(toggle).count(rows.len()))
            .child(
                Text::labeled("ignored-hint", "Not offered or counted")
                    .level(text::CAPTION)
                    .muted(),
            )
            .children(flashing.map(|flash| flash_overlay(flash, cx)));
        list = list.child(head);
        if open {
            for row in matched {
                list = list.row(self.ignored_row(row, cx));
            }
        }
        list
    }

    /// An ignored row: no checkbox but its space, the slug and description dimmed, where it was
    /// found, and Stop ignoring (I6).
    fn ignored_row(&self, row: &Row<'_>, cx: &mut Context<Self>) -> ListRow {
        let (slug, key) = (slug(row), row.key().to_owned());
        let tick = row.tickable() && !row.invalid();
        let description = row
            .shown()
            .map(|version| version.description.clone())
            .unwrap_or_default();
        let stop = Button::new(format!("import-stop-ignoring-{key}"))
            .label("Stop ignoring")
            .small()
            .accessibility_label(format!("Stop ignoring {slug}"))
            .on_click(cx.listener({
                let (slug, key) = (slug.clone(), key.clone());
                move |view, _, _, cx| view.stop_ignoring(slug.clone(), key.clone(), tick, cx)
            }));
        let muted = cx.theme().muted_foreground;
        ListRow::new(format!("import-ignored-row-{key}"))
            .items_start()
            .leading(div().size(px(space::XL)))
            .title(div().text_color(muted).child(Text::labeled("slug", slug)))
            .when(!description.is_empty(), |row| row.subtitle(description))
            .child(
                text::CAPTION
                    .apply(div())
                    .text_color(muted)
                    .child(self.found_in(row)),
            )
            .trailing(stop)
    }

    /// "Found in <places>", from every location of every version, each place once.
    fn found_in(&self, row: &Row<'_>) -> String {
        let snapshot = &self.snapshot;
        let mut places: Vec<String> = Vec::new();
        for location in row
            .skill
            .versions
            .iter()
            .flat_map(|version| &version.locations)
        {
            let directory = Path::new(&location.display_path)
                .parent()
                .map_or_else(String::new, |parent| parent.display().to_string());
            let place = place_name(
                &directory,
                &snapshot.agents,
                &snapshot.workspaces,
                &snapshot.home,
            );
            if !places.contains(&place) {
                places.push(place);
            }
        }
        format!("Found in {}", places.join(", "))
    }

    /// Edits `row` to reveal its Ignore while hovered, and to flash once when it was brought
    /// back (I7).
    pub(super) fn offered_row(
        &self,
        row: ListRow,
        key: &str,
        state: &ImportDialog,
        cx: &App,
    ) -> ListRow {
        let flashing = state
            .flash
            .as_ref()
            .filter(|flash| flash.key.as_deref() == Some(key));
        row.group(ROW_GROUP)
            .children(flashing.map(|flash| flash_overlay(flash, cx)))
    }
}

/// A see-through fill over its parent in the accent's soft tone that fades out once
/// (scan-import D11's flash); the parent sets the position.
fn flash_overlay(flash: &Flash, cx: &App) -> AnyElement {
    let tone = color::ACCENT_SOFT.of(cx.theme());
    div()
        .absolute()
        .inset_0()
        .bg(tone)
        .with_animation(
            ("import-flash", flash.id),
            Animation::new(cx.theme().motion.duration_slow),
            |fill, delta| fill.opacity(opacity::INACTIVE * (1. - delta)),
        )
        .into_any_element()
}
