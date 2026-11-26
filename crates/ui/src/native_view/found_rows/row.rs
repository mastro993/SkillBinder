//! A found skill's row in a page's list (U3, U4, U7): the Library row's frame, but not a link,
//! since a found skill has no page. It reads the slug with its validation glyph and a Not in
//! library chip, or for an ignored skill an Ignored chip with the slug and description dimmed;
//! then the description and a muted mono line naming where it was found. It ends with Ignore and
//! Import…, or Stop ignoring, shown while the row is hovered or a button holds focus.

use super::super::NativeView;
use super::super::ignore_actions::flash_overlay;
use super::super::library_items::row_frame;
use super::super::library_validation::found_glyph;
use super::super::snapshot_query::FoundRow;
use super::{FoundPage, ROW_GROUP};
use gpui_kit::component::{ActiveTheme, h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled, div,
    prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::{opacity, space, text};
use skillbinder_design::{Button, Chip, ChipVariant, Text};
use std::collections::BTreeSet;

impl NativeView {
    /// `row` at `index` of the card: on a workspace's page its line names the directories holding
    /// the skill, then on every page the other places it was found in. It flashes once when Stop
    /// ignoring brought it back.
    pub(super) fn found_row(
        &self,
        page: &FoundPage<'_>,
        row: &FoundRow<'_>,
        index: usize,
        last: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let key = row.key;
        let dim =
            |element: gpui_kit::Div| element.when(row.ignored, |el| el.opacity(opacity::INACTIVE));
        let (label, variant) = if row.ignored {
            ("Ignored", ChipVariant::Neutral)
        } else {
            ("Not in library", ChipVariant::Outline)
        };
        let title = h_flex()
            .min_w_0()
            .gap(px(space::SM))
            .child(
                dim(text::TITLE_SM.apply(div()).min_w_0().truncate()).child(row.shown.slug.clone()),
            )
            .children(found_glyph(row.shown))
            .child(Chip::new(variant).child(Text::labeled("tag", label)));
        let also = row.also_in(&self.snapshot);
        let line: Vec<String> = page
            .workspace
            .map(|workspace| row.dirs_in(workspace).join(", "))
            .filter(|dirs| !dirs.is_empty())
            .into_iter()
            .chain((!also.is_empty()).then(|| format!("Also found in {}", also.join(", "))))
            .collect();
        let description = match row.shown.description.as_str() {
            "" => "No description".to_owned(),
            description => description.to_owned(),
        };
        let flashing = self
            .found_flash
            .as_ref()
            .filter(|flash| flash.flashes(Some(key)));
        row_frame(format!("found-row-{key}"), ROW_GROUP, index, last)
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(space::XXS))
                    .child(title)
                    .child(
                        dim(text::CAPTION.apply(div()).min_w_0().truncate())
                            .text_color(cx.theme().muted_foreground)
                            .child(description),
                    )
                    .when(!line.is_empty(), |body| {
                        body.child(
                            Text::labeled("where", line.join(" · "))
                                .level(text::MONO_SM)
                                .muted()
                                .min_w_0()
                                .truncate(),
                        )
                    }),
            )
            .child(
                h_flex()
                    .flex_none()
                    .gap(px(space::XS))
                    .children(self.found_actions(row, cx)),
            )
            .children(flashing.map(|flash| flash_overlay(flash, cx)))
            .into_any_element()
    }

    /// Ignore and Import… (U4), or Stop ignoring for an ignored skill (U7), each revealed while
    /// the row is hovered or it holds keyboard focus.
    fn found_actions(&self, row: &FoundRow<'_>, cx: &mut Context<Self>) -> Vec<Button> {
        let (slug, key) = (row.shown.slug.clone(), row.key.to_owned());
        let buttons = if row.ignored {
            vec![
                Button::new(format!("found-stop-ignoring-{key}"))
                    .label("Stop ignoring")
                    .accessibility_label(format!("Stop ignoring {slug}"))
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.stop_ignoring(slug.clone(), key.clone(), false, cx);
                    })),
            ]
        } else {
            let only = BTreeSet::from([key.clone()]);
            vec![
                Button::new(format!("found-ignore-{key}"))
                    .label("Ignore")
                    .ghost()
                    .accessibility_label(format!("Ignore {slug}"))
                    .tooltip("Ignore everywhere: never offered for import")
                    .on_click(cx.listener({
                        let (slug, key) = (slug.clone(), key.clone());
                        move |view, _, _, cx| {
                            view.ignore_found(vec![(slug.clone(), key.clone())], cx);
                        }
                    })),
                Button::new(format!("found-import-{key}"))
                    .label("Import…")
                    .accessibility_label(format!("Import {slug}"))
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.open_import_dialog_for(only.clone(), window, cx);
                    })),
            ]
        };
        buttons
            .into_iter()
            .map(|button| {
                button
                    .small()
                    .inactive(self.blocked())
                    .opacity(0.)
                    .group_hover(ROW_GROUP, |style| style.opacity(1.))
                    .focus_visible(|style| style.opacity(1.))
            })
            .collect()
    }
}
