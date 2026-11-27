//! Not synced yet (Y5): what a Push sends, under this device, and what a Pull brings, under the
//! devices that changed it, each skill with its kind and why. Hidden while nothing waits.

use super::copy;
use super::status::tinted;
use chrono::{DateTime, Utc};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::{icon as icon_size, radius, space, text};
use skillbinder_design::{
    Chip, ChipVariant, IconSize, Level, Separator, Surface, Text, icon, icon_color, tooltip,
};
use skillbinder_proto::{ChangedSkill, SyncSnapshot};

/// The section, while anything waits on either side.
pub(super) fn pending(sync: &SyncSnapshot, now: DateTime<Utc>, cx: &App) -> Option<AnyElement> {
    let incoming = sync
        .incoming
        .as_ref()
        .filter(|incoming| !incoming.skills.is_empty());
    let mut groups = Vec::new();
    if !sync.local.is_empty() {
        let head = device("sync-local-device", Some(sync.device.clone()), true, cx);
        groups.push(group(
            "sync-local",
            head,
            "Not pushed".to_owned(),
            &sync.local,
            cx,
        ));
    }
    if let Some(incoming) = incoming {
        let head = device("sync-incoming-device", copy::devices(incoming), false, cx);
        let end = copy::not_pulled(incoming, now);
        groups.push(group("sync-incoming", head, end, &incoming.skills, cx));
    }
    if groups.is_empty() {
        return None;
    }
    let mut card = Surface::new(Level::Flat)
        .radius(radius::LG)
        .apply(v_flex().id("sync-pending").overflow_hidden(), cx.theme());
    for (index, group) in groups.into_iter().enumerate() {
        if index > 0 {
            card = card.child(Separator::horizontal());
        }
        card = card.child(group);
    }
    Some(
        // The column's gap, then as much again, sets the section off from the facts above it.
        v_flex()
            .mt(px(space::MD))
            .gap(px(space::SM))
            .child(
                div().px(px(space::XXS)).child(
                    Text::labeled("sync-pending-title", "Not synced yet").level(text::TITLE_SM),
                ),
            )
            .child(card)
            .into_any_element(),
    )
}

/// One side: its device and, at its end, `end`, over a row per skill.
fn group(
    id: &'static str,
    head: AnyElement,
    end: String,
    skills: &[ChangedSkill],
    cx: &App,
) -> impl IntoElement {
    let head = h_flex()
        .justify_between()
        .gap(px(space::MD))
        .px(px(space::LG))
        .pt(px(space::MD))
        .pb(px(space::XS))
        .child(head)
        .child(
            Text::labeled(SharedString::from(format!("{id}-end")), end)
                .level(text::CAPTION)
                .muted(),
        );
    v_flex()
        .id(id)
        .pb(px(space::XS))
        .child(head)
        .children(skills.iter().map(|skill| row(id, skill, cx)))
}

/// A device: the laptop glyph and its name in bold, with an outline "This device" tag for this
/// one; a commit made before device names reads "Before device names" after a clock glyph.
fn device(id: &'static str, name: Option<String>, this: bool, cx: &App) -> AnyElement {
    let (glyph, name) = match name {
        Some(name) => ("laptop", name),
        None => ("clock", "Before device names".to_owned()),
    };
    h_flex()
        .min_w_0()
        .gap(px(space::SM))
        .child(
            icon(glyph)
                .with_size(IconSize::Sm)
                .text_color(icon_color(cx.theme())),
        )
        .child(Text::labeled(id, name).level(text::LABEL_MD))
        .when(this, |device| {
            device.child(Chip::new(ChipVariant::Outline).child(Text::labeled(
                SharedString::from(format!("{id}-tag")),
                "This device",
            )))
        })
        .into_any_element()
}

/// A skill: its kind mark, its slug in bold, and why it changed in muted text.
fn row(group: &str, skill: &ChangedSkill, cx: &App) -> impl IntoElement {
    let slug = &skill.slug;
    let theme = cx.theme();
    h_flex()
        .id(SharedString::from(format!("{group}-{slug}")))
        .min_h(px(space::CONTROL_MD))
        .gap(px(space::MD))
        .px(px(space::LG))
        .hover(|row| row.bg(theme.muted))
        .child(mark(group, skill, cx))
        .child(
            div().flex_none().child(
                Text::labeled(
                    SharedString::from(format!("{group}-slug-{slug}")),
                    slug.clone(),
                )
                .level(text::LABEL_MD),
            ),
        )
        .children(copy::why(skill).map(|why| {
            div().min_w_0().truncate().child(
                Text::labeled(SharedString::from(format!("{group}-why-{slug}")), why)
                    .level(text::CAPTION)
                    .muted(),
            )
        }))
}

/// The 16-point kind mark, tinted by the kind and named by its tooltip.
fn mark(group: &str, skill: &ChangedSkill, cx: &App) -> impl IntoElement {
    let (glyph, label, tint) = copy::kind(skill.kind);
    let (fill, ink) = tinted(tint, cx.theme());
    div()
        .id(SharedString::from(format!("{group}-kind-{}", skill.slug)))
        .flex_none()
        .size(px(icon_size::MD))
        .rounded(px(radius::XS))
        .bg(fill)
        .text_color(ink)
        .flex()
        .items_center()
        .justify_center()
        .aria_label(label)
        .tooltip(tooltip(label))
        .child(Text::new(glyph).level(text::MONO_SM))
}
