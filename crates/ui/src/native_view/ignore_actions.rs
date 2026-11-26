//! Ignoring found skills, the one seam around [`skillbinder_client::Client::set_ignored`] that
//! the Import skills dialog and the workspace, agent, and Universal pages share (ADR 0033, ADR
//! 0036). An ignore names a slug everywhere and is saved at once, with no toast: the Ignored group
//! or section is its undo. Once saved, the Ignored header, or a skill brought back, flashes once
//! in the dialog and on the page (I4, I7, U5, U7).

use super::NativeView;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{Animation, AnimationExt, AnyElement, App, Context, IntoElement, Styled, div};
use skillbinder_design::tokens::{color, opacity};
use skillbinder_proto::{AppResult, MAX_IMPORT};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};

/// Numbers each flash, so every one plays its animation from the start.
static FLASHES: AtomicU64 = AtomicU64::new(0);

/// What flashes once, and which flash it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Flash {
    /// A skill's key, or `None` for the Ignored header.
    pub key: Option<String>,
    id: u64,
}

impl Flash {
    fn new(key: Option<String>) -> Self {
        Self {
            key,
            id: FLASHES.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Whether it flashes the skill `key`, or the Ignored header for `None`.
    pub(super) fn flashes(&self, key: Option<&str>) -> bool {
        self.key.as_deref() == key
    }
}

impl NativeView {
    /// Ignores `rows`, each a slug and its key, dropping their ticks in the import list.
    pub(super) fn ignore_found(&mut self, rows: Vec<(String, String)>, cx: &mut Context<Self>) {
        let (slugs, keys): (Vec<String>, Vec<String>) = rows.into_iter().unzip();
        self.edit_import(|dialog| untick(&mut dialog.ticked, &keys), cx);
        let client = self.client.clone();
        let asked = async move { client.set_ignored(slugs, true).await };
        self.ask(asked, |view, result, cx| view.ignored(result, None, cx), cx);
    }

    /// Stops ignoring the skill `slug`, `key`, which comes back ticked in the import list when
    /// `tick` and the cap allow (I7).
    pub(super) fn stop_ignoring(
        &mut self,
        slug: String,
        key: String,
        tick: bool,
        cx: &mut Context<Self>,
    ) {
        let client = self.client.clone();
        let asked = async move { client.set_ignored(vec![slug], false).await };
        self.ask(
            asked,
            move |view, result, cx| {
                if result.is_ok() && tick {
                    view.edit_import(|dialog| retick(&mut dialog.ticked, &key), cx);
                }
                view.ignored(result, Some(key), cx);
            },
            cx,
        );
    }

    /// Shows the snapshot an ignore or a stop saved, flashing the Ignored header or the skill
    /// `key`, or the reason it failed.
    fn ignored(&mut self, result: AppResult<()>, key: Option<String>, cx: &mut Context<Self>) {
        match result {
            Ok(()) => {
                self.receive(self.client.snapshot(), cx);
                let flash = Flash::new(key);
                self.found_sections.flash = Some(flash.clone());
                self.edit_import(|dialog| dialog.flash = Some(flash), cx);
            }
            Err(error) => self.error = Some(error),
        }
        cx.notify();
    }
}

/// Drops the ignored `keys` from the import list's ticks.
fn untick(ticked: &mut Option<BTreeSet<String>>, keys: &[String]) {
    if let Some(ticked) = ticked.as_mut() {
        ticked.retain(|key| !keys.contains(key));
    }
}

/// Ticks `key` again in the import list while the cap allows.
fn retick(ticked: &mut Option<BTreeSet<String>>, key: &str) {
    let ticked = ticked.get_or_insert_default();
    if ticked.len() < MAX_IMPORT {
        ticked.insert(key.to_owned());
    }
}

/// A see-through fill over its parent in the accent's soft tone that fades out once
/// (scan-import D11's flash); the parent sets the position.
pub(super) fn flash_overlay(flash: &Flash, cx: &App) -> AnyElement {
    let tone = color::ACCENT_SOFT.of(cx.theme());
    div()
        .absolute()
        .inset_0()
        .bg(tone)
        .with_animation(
            ("ignore-flash", flash.id),
            Animation::new(cx.theme().motion.duration_slow),
            |fill, delta| fill.opacity(opacity::INACTIVE * (1. - delta)),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(keys: &[&str]) -> BTreeSet<String> {
        keys.iter().map(|key| (*key).to_owned()).collect()
    }

    #[test]
    fn ignoring_drops_the_ticks_of_the_ignored_skills_alone() {
        let mut ticked = Some(keys(&["a", "b", "c"]));
        untick(&mut ticked, &["a".to_owned(), "c".to_owned()]);
        assert_eq!(ticked, Some(keys(&["b"])));
        // A list that has not ticked its defaults yet stays so.
        let mut none = None;
        untick(&mut none, &["a".to_owned()]);
        assert_eq!(none, None);
    }

    #[test]
    fn stopping_ticks_the_skill_again_while_the_cap_allows() {
        let mut ticked = None;
        retick(&mut ticked, "a");
        assert_eq!(ticked, Some(keys(&["a"])));
        let mut full = Some((0..MAX_IMPORT).map(|index| index.to_string()).collect());
        retick(&mut full, "a");
        assert!(full.is_some_and(|ticked| !ticked.contains("a")));
    }
}
