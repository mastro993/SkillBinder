//! A merge's choices in History (Y6): each skill changed on both sides as "Kept <device>'s
//! <slug>", and where the remote's version won, Restore <device>'s version, which restores the
//! merging device's version from the merge.

use super::super::super::NativeView;
use super::super::copy;
use super::super::restore::ROW;
use gpui_kit::component::{ActiveTheme, Sizable, h_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, SharedString, Styled, div,
    px,
};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{IconSize, Text, icon, icon_color};
use skillbinder_proto::{Choice, SyncChange};

impl NativeView {
    /// A row per choice of the merge `change`, made at `when`.
    pub(super) fn choice_rows(
        &self,
        change: &SyncChange,
        id: &str,
        when: &(String, String),
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        (change.choices.iter())
            .map(|choice| self.choice_row(change, choice, id, when.clone(), cx))
            .collect()
    }

    fn choice_row(
        &self,
        change: &SyncChange,
        choice: &Choice,
        id: &str,
        when: (String, String),
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let slug = &choice.slug;
        let end = (choice.unpicked.as_deref())
            .map(|unpicked| self.restore_unpicked(change, (slug, unpicked), when, cx));
        let hover = cx.theme().muted;
        h_flex()
            .id(SharedString::from(format!("{id}-choice-{slug}")))
            .group(ROW)
            .min_h(px(space::CONTROL_MD))
            .gap(px(space::MD))
            .px(px(space::LG))
            .hover(move |row| row.bg(hover))
            .child(
                icon("git-merge")
                    .with_size(IconSize::Sm)
                    .text_color(icon_color(cx.theme())),
            )
            .child(
                div().min_w_0().truncate().child(
                    Text::labeled(
                        SharedString::from(format!("{id}-kept-{slug}")),
                        copy::kept(choice.device.as_deref(), slug),
                    )
                    .level(text::BODY_SM),
                ),
            )
            .children(end.map(|end| div().ml_auto().flex_none().child(end)))
            .into_any_element()
    }
}
