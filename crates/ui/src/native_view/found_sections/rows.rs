//! A found skill's row on a page (U3, U4, U7): not a link, since a found skill has no page. It
//! reads the slug with its validation glyph, the description, and a muted mono line naming where
//! it was found, and ends with Ignore while hovered or focused, or for an ignored skill, dimmed,
//! with Stop ignoring.

use super::super::NativeView;
use super::super::ignore_actions::flash_overlay;
use super::super::library_validation::found_glyph;
use super::super::snapshot_query::FoundRow;
use super::FoundPage;
use gpui_kit::component::{ActiveTheme, h_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{Button, ListRow, Text};

/// The group a row's Ignore is revealed by.
const ROW_GROUP: &str = "found-row";

impl NativeView {
    /// A Not in library row: on a workspace's page, its line names the directories holding the
    /// skill, then on every page the other places it was found in. It flashes once when it was
    /// brought back.
    pub(super) fn found_row(
        &self,
        page: &FoundPage<'_>,
        row: &FoundRow<'_>,
        cx: &mut Context<Self>,
    ) -> ListRow {
        let (slug, key) = (row.shown.slug.clone(), row.key.to_owned());
        let also = row.also_in(&self.snapshot);
        let line: Vec<String> = dirs(page, row)
            .into_iter()
            .chain((!also.is_empty()).then(|| format!("Also found in {}", also.join(", "))))
            .collect();
        let ignored = vec![(slug.clone(), key.clone())];
        let ignore = Button::new(format!("found-ignore-{key}"))
            .label("Ignore")
            .ghost()
            .small()
            .accessibility_label(format!("Ignore {slug}"))
            .tooltip("Ignore everywhere: never offered for import")
            .inactive(self.blocked())
            .opacity(0.)
            .group_hover(ROW_GROUP, |style| style.opacity(1.))
            .focus_visible(|style| style.opacity(1.))
            .on_click(cx.listener(move |view, _, _, cx| view.ignore_found(ignored.clone(), cx)));
        let flashing = self
            .found_sections
            .flash
            .as_ref()
            .filter(|flash| flash.flashes(Some(&key)));
        ListRow::new(format!("found-row-{key}"))
            .group(ROW_GROUP)
            .title(
                h_flex()
                    .gap(px(space::SM))
                    .child(Text::labeled("slug", slug))
                    .children(found_glyph(row.shown)),
            )
            .subtitle(description(row))
            .children(where_line(line))
            .trailing(ignore)
            .children(flashing.map(|flash| flash_overlay(flash, cx)))
    }

    /// An Ignored row: the slug and description dimmed, the directories holding it on a
    /// workspace's page, and Stop ignoring, which brings it back under Not in library (U7).
    pub(super) fn ignored_found_row(
        &self,
        page: &FoundPage<'_>,
        row: &FoundRow<'_>,
        cx: &mut Context<Self>,
    ) -> ListRow {
        let (slug, key) = (row.shown.slug.clone(), row.key.to_owned());
        let stop = Button::new(format!("found-stop-ignoring-{key}"))
            .label("Stop ignoring")
            .small()
            .accessibility_label(format!("Stop ignoring {slug}"))
            .inactive(self.blocked())
            .on_click(cx.listener({
                let (slug, key) = (slug.clone(), key.clone());
                move |view, _, _, cx| view.stop_ignoring(slug.clone(), key.clone(), false, cx)
            }));
        let muted = cx.theme().muted_foreground;
        ListRow::new(format!("found-ignored-row-{key}"))
            .title(div().text_color(muted).child(Text::labeled("slug", slug)))
            .subtitle(description(row))
            .children(where_line(dirs(page, row)))
            .trailing(stop)
    }
}

/// The directories holding `row` in the page's workspace, on one line; none on an agent's page.
fn dirs(page: &FoundPage<'_>, row: &FoundRow<'_>) -> Option<String> {
    page.workspace
        .map(|workspace| row.dirs_in(workspace).join(", "))
        .filter(|dirs| !dirs.is_empty())
}

/// The shown version's description, or "No description" (U3).
fn description(row: &FoundRow<'_>) -> String {
    match row.shown.description.as_str() {
        "" => "No description".to_owned(),
        description => description.to_owned(),
    }
}

/// `parts` joined by " · " in muted mono, or nothing when there are none.
fn where_line(parts: impl IntoIterator<Item = String>) -> Option<AnyElement> {
    let parts: Vec<String> = parts.into_iter().collect();
    (!parts.is_empty()).then(|| {
        Text::labeled("where", parts.join(" · "))
            .level(text::MONO_SM)
            .muted()
            .min_w_0()
            .truncate()
            .into_any_element()
    })
}
