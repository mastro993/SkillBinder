//! A found skill's row on a page (U3, U4, U7): not a link, since a found skill has no page. It
//! reads the slug with its validation glyph, and the Ignored tag while its slug is ignored, the
//! description, and a muted mono line naming where it was found. It ends with Ignore, or Stop
//! ignoring for an ignored skill, shown while the row is hovered or the button holds focus.

use super::super::NativeView;
use super::super::ignore_actions::flash_overlay;
use super::super::library_validation::found_glyph;
use super::super::snapshot_query::FoundRow;
use super::FoundPage;
use gpui_kit::component::{Sizable, h_flex, tag::Tag};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled,
    prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{Button, ListRow, Text};

/// The group a row's button is revealed by.
const ROW_GROUP: &str = "found-row";

impl NativeView {
    /// A Not in library row: on a workspace's page, its line names the directories holding the
    /// skill, then on every page the other places it was found in. It flashes once when Stop
    /// ignoring brought it back.
    pub(super) fn found_row(
        &self,
        page: &FoundPage<'_>,
        row: &FoundRow<'_>,
        cx: &mut Context<Self>,
    ) -> ListRow {
        let key = row.key;
        let also = row.also_in(&self.snapshot);
        let line: Vec<String> = dirs(page, row)
            .into_iter()
            .chain((!also.is_empty()).then(|| format!("Also found in {}", also.join(", "))))
            .collect();
        let flashing = self
            .found_section
            .flash
            .as_ref()
            .filter(|flash| flash.flashes(Some(key)));
        ListRow::new(format!("found-row-{key}"))
            .group(ROW_GROUP)
            .title(
                h_flex()
                    .gap(px(space::SM))
                    .child(Text::labeled("slug", row.shown.slug.clone()))
                    .children(found_glyph(row.shown))
                    .when(row.ignored, |title| {
                        title.child(Tag::secondary().small().child("Ignored"))
                    }),
            )
            .subtitle(description(row))
            .children(where_line(line))
            .trailing(self.row_action(row, cx))
            .children(flashing.map(|flash| flash_overlay(flash, cx)))
    }

    /// Ignore (U4), or Stop ignoring for an ignored skill (U7), revealed while the row is
    /// hovered or the button holds keyboard focus.
    fn row_action(&self, row: &FoundRow<'_>, cx: &mut Context<Self>) -> Button {
        let (slug, key) = (row.shown.slug.clone(), row.key.to_owned());
        let button = if row.ignored {
            Button::new(format!("found-stop-ignoring-{key}"))
                .label("Stop ignoring")
                .accessibility_label(format!("Stop ignoring {slug}"))
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.stop_ignoring(slug.clone(), key.clone(), false, cx);
                }))
        } else {
            Button::new(format!("found-ignore-{key}"))
                .label("Ignore")
                .ghost()
                .accessibility_label(format!("Ignore {slug}"))
                .tooltip("Ignore everywhere: never offered for import")
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.ignore_found(vec![(slug.clone(), key.clone())], cx);
                }))
        };
        button
            .small()
            .inactive(self.blocked())
            .opacity(0.)
            .group_hover(ROW_GROUP, |style| style.opacity(1.))
            .focus_visible(|style| style.opacity(1.))
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
