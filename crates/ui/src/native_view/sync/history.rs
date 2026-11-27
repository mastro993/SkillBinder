//! History (Y6): the library's published changes by day, each with the device that made it, its
//! skills as chips, and its time; a change opens in place to list its skills, each with why and
//! Restore this version, Bring back, or Current version, and a merge's choices. The
//! engine reads it a page of 50 at a time (A3), again from the newest whenever the library's
//! commit moves, and the changes that move added flash once.

mod choices;
mod days;

use super::super::NativeView;
use super::super::ignore_actions::{Flash, flash_overlay};
use super::super::sidebar_row::activate;
use super::{copy, pending};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement, Role, SharedString,
    StatefulInteractiveElement, Styled, Task, div, prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::{radius, space, text};
use skillbinder_design::{
    Button, Chip, ChipVariant, IconSize, Level, Separator, Surface, Text, icon, icon_color,
};
use skillbinder_proto::{AppError, AppResult, ChangeKind, ChangedSkill, SyncChange, SyncHistory};
use std::collections::{BTreeMap, BTreeSet};

/// How many changes a page holds (A3).
const PAGE: usize = 50;

/// The pages of History read so far, and which changes are open.
#[derive(Default)]
pub(in crate::native_view) struct HistoryPages {
    /// The library commit the pages were read at, once a read for it answered.
    read_at: Option<Option<String>>,
    /// The library commit of the first page being read.
    reading: Option<Option<String>>,
    changes: Vec<SyncChange>,
    /// The commit the next page starts after, while older changes remain.
    next: Option<String>,
    /// Whether Show more is reading the next page.
    more: bool,
    /// The open changes, by commit.
    open: BTreeSet<String>,
    /// The changes the last move of the library's commit added, which flash once.
    fresh: Option<(BTreeSet<String>, Flash)>,
    task: Option<Task<()>>,
}

impl HistoryPages {
    /// Forgets the flash, so coming back to the page does not play it again.
    pub(in crate::native_view) fn leave(&mut self) {
        self.fresh = None;
    }
}

impl NativeView {
    /// Reads History's first page again when the library's commit moved since it was read.
    pub(super) fn follow_history(&mut self, cx: &mut Context<Self>) {
        let head = Some(self.snapshot.sync.head.clone());
        let pages = &self.sync_history;
        if pages.read_at != head && pages.reading != head {
            self.sync_history.reading = head;
            self.sync_history.more = false;
            self.read_history(None, cx);
        }
    }

    /// Reads the page after `before`, or the first page without it.
    fn read_history(&mut self, before: Option<String>, cx: &mut Context<Self>) {
        let client = self.client.clone();
        let after = before.clone();
        let handle = client
            .runtime()
            .spawn(async move { client.sync_history(after, PAGE).await });
        self.sync_history.task = Some(cx.spawn(async move |this, cx| {
            let result = handle.await.unwrap_or_else(|_| Err(AppError::storage()));
            let _ = this.update(cx, |view, cx| {
                view.history_read(before.is_some(), result, cx);
                cx.notify();
            });
        }));
    }

    /// Shows a page that was read: Show more's after the others, a first page in their place.
    fn history_read(&mut self, more: bool, result: AppResult<SyncHistory>, cx: &mut Context<Self>) {
        let pages = &mut self.sync_history;
        // A read answered before, even with no change, so whatever this one brings is new.
        let answered = pages.read_at.is_some();
        if more {
            pages.more = false;
        } else {
            // A failed read is not retried until the library's commit moves again.
            pages.read_at = pages.reading.take();
        }
        let page = match result {
            Ok(page) => page,
            Err(error) => {
                self.error = Some(error);
                return;
            }
        };
        if more {
            pages.changes.extend(page.changes);
        } else {
            let fresh = days::fresh(answered, &pages.changes, &page.changes);
            if !fresh.is_empty() {
                pages.fresh = Some((fresh, Flash::new(None)));
            }
            pages.changes = page.changes;
        }
        pages.next = page.next;
        cx.notify();
    }

    /// Reads the next page, as Show more does.
    fn show_more_history(&mut self, cx: &mut Context<Self>) {
        let pages = &mut self.sync_history;
        if pages.more || pages.reading.is_some() {
            return;
        }
        let Some(next) = pages.next.clone() else {
            return;
        };
        pages.more = true;
        self.read_history(Some(next), cx);
        cx.notify();
    }

    /// Opens the change `commit` in place, or closes it.
    fn toggle_change(&mut self, commit: &str, cx: &mut Context<Self>) {
        let open = &mut self.sync_history.open;
        if !open.remove(commit) {
            open.insert(commit.to_owned());
        }
        cx.notify();
    }

    /// The section, once the library has a change to show.
    pub(super) fn sync_history(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pages = &self.sync_history;
        if pages.changes.is_empty() {
            return None;
        }
        let now = chrono::Local::now();
        let newest = days::newest(&pages.changes);
        let days = days::days(&pages.changes, &now);
        let more = pages.next.as_ref().map(|_| {
            Button::new("sync-history-more")
                .label("Show more")
                .small()
                .loading(pages.more)
                .inactive(self.blocked() || self.snapshot.syncing.is_some())
                .on_click(cx.listener(|view, _, _, cx| view.show_more_history(cx)))
        });
        let mut section = v_flex()
            .id("sync-history")
            // The column's gap, then as much again, as Not synced yet is set off.
            .mt(px(space::MD))
            .gap(px(space::MD))
            .child(
                div()
                    .px(px(space::XXS))
                    .child(Text::labeled("sync-history-title", "History").level(text::TITLE_SM)),
            );
        for (index, day) in days.into_iter().enumerate() {
            let mut card = Surface::new(Level::Flat)
                .radius(radius::LG)
                .apply(v_flex().overflow_hidden(), cx.theme());
            for (row, change) in day.changes.into_iter().enumerate() {
                if row > 0 {
                    card = card.child(Separator::horizontal());
                }
                card = card.child(self.change(change, &newest, &now, cx));
            }
            section = section.child(
                v_flex()
                    .id(SharedString::from(format!("sync-day-{index}")))
                    .gap(px(space::XS))
                    .child(
                        div().px(px(space::XXS)).child(
                            Text::labeled(
                                SharedString::from(format!("sync-day-{index}-heading")),
                                day.heading,
                            )
                            .level(text::CAPTION)
                            .muted(),
                        ),
                    )
                    .child(card),
            );
        }
        Some(
            section
                .children(more.map(|more| h_flex().child(more)))
                .into_any_element(),
        )
    }

    /// One change: its device over its chips, then its time; opened, a row per skill.
    fn change(
        &self,
        change: &SyncChange,
        newest: &BTreeMap<&str, &str>,
        now: &chrono::DateTime<chrono::Local>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pages = &self.sync_history;
        let id: String = change.commit.chars().take(12).collect();
        let id = format!("sync-change-{id}");
        let open = pages.open.contains(&change.commit);
        let time = days::time(change, now);
        let this = change.device.as_deref() == Some(self.snapshot.sync.device.as_str());
        let who = change
            .device
            .clone()
            .unwrap_or_else(|| "Before device names".to_owned());
        let (shown, more) = days::chips(&change.skills);
        let chips = h_flex()
            .flex_wrap()
            .gap(px(space::XS))
            .children(shown.iter().map(|skill| chip(&id, skill, cx)))
            .when(more > 0, |chips| {
                chips.child(
                    Text::labeled(
                        SharedString::from(format!("{id}-more")),
                        format!("+{more} more"),
                    )
                    .level(text::CAPTION)
                    .muted(),
                )
            });
        let commit = change.commit.clone();
        let hover = cx.theme().muted;
        let head = h_flex()
            .id(SharedString::from(format!("{id}-head")))
            .gap(px(space::MD))
            .px(px(space::LG))
            .py(px(space::MD))
            .cursor_pointer()
            .hover(move |head| head.bg(hover))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(space::SM))
                    .child(pending::device(
                        format!("{id}-device").into(),
                        change.device.clone(),
                        this,
                        cx,
                    ))
                    .child(chips),
            )
            .child(
                Text::labeled(SharedString::from(format!("{id}-time")), time.clone())
                    .level(text::CAPTION)
                    .muted()
                    .tabular(),
            )
            .child(
                icon(if open { "arrow-down" } else { "chevron-right" })
                    .with_size(IconSize::Xs)
                    .text_color(icon_color(cx.theme())),
            );
        let label = format!("{who}, {time}, {}", copy::skills(change.skills.len()));
        let head = activate(head, label, cx, move |view, _, cx| {
            view.toggle_change(&commit, cx)
        })
        .role(Role::Button)
        .aria_expanded(open);
        let flash = pages
            .fresh
            .as_ref()
            .filter(|(fresh, _)| fresh.contains(&change.commit))
            .map(|(_, flash)| flash_overlay(flash, cx));
        let mut skills = Vec::new();
        for skill in change.skills.iter().filter(|_| open) {
            let current = newest.get(skill.slug.as_str()) == Some(&change.commit.as_str())
                && skill.kind != ChangeKind::Removed;
            let end = if current {
                Text::labeled(
                    SharedString::from(format!("{id}-current-{}", skill.slug)),
                    "Current version",
                )
                .level(text::CAPTION)
                .muted()
                .into_any_element()
            } else {
                let when = (days::day(change, now), time.clone());
                self.restore_button(change, skill, when, cx)
            };
            skills.push(pending::row(&id, skill, false, Some(end), cx).into_any_element());
        }
        if open {
            let when = (days::day(change, now), time.clone());
            skills.extend(self.choice_rows(change, &id, &when, cx));
        }
        v_flex()
            .id(SharedString::from(id.clone()))
            .relative()
            .child(head)
            .when(open, |row| {
                row.child(Separator::horizontal())
                    .child(v_flex().py(px(space::XS)).children(skills))
            })
            .children(flash)
            .into_any_element()
    }
}

/// A skill chip: its kind mark and its slug, outlined, since a muted fill would vanish into the
/// row's hover.
fn chip(change: &str, skill: &ChangedSkill, cx: &App) -> impl IntoElement {
    Chip::new(ChipVariant::Outline).child(
        h_flex()
            .gap(px(space::XS))
            .child(pending::mark(&format!("{change}-chip"), skill, cx))
            .child(Text::labeled(
                SharedString::from(format!("{change}-chip-{}", skill.slug)),
                skill.slug.clone(),
            )),
    )
}
