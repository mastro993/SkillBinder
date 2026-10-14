//! Library skills as list rows or grid cards, each showing when it was imported and changed and
//! where the last scan found it.

use super::controls::{agent_avatar, app_icon, glyph_avatar};
use super::library_dates::{exact, relative};
use super::library_presence::{Place, Reader};
use super::{NativeView, Route};
use chrono::{DateTime, Utc};
use gpui_kit::component::{
    ActiveTheme, Sizable, Size,
    avatar::{Avatar, AvatarGroup},
    h_flex,
    tag::Tag,
    tooltip::Tooltip,
    v_flex,
};
use gpui_kit::{
    AnyElement, App, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::{Skill, WorkspaceRef};

/// Workspace chips shown before the rest collapse into `+N`.
const SHOWN_PLACES: usize = 3;
/// Agent avatars shown in a chip before the rest collapse into `+N`.
const SHOWN_READERS: usize = 3;

/// One Library skill with its description, dates, and presence.
pub(super) struct SkillItem<'a> {
    pub skill: &'a Skill,
    pub description: &'a str,
    /// Absent for skills imported before import times were recorded.
    pub imported: Option<DateTime<Utc>>,
    /// The newest managed file's change time; absent when the payload could not be read.
    pub modified: Option<DateTime<Utc>>,
    /// `None` until the first scan completes.
    pub places: Option<&'a [Place]>,
}

impl SkillItem<'_> {
    fn name(&self) -> String {
        self.skill
            .display_name
            .clone()
            .unwrap_or_else(|| self.skill.slug.clone())
    }
}

impl NativeView {
    /// A full-width row: name, one description line, and dates, then presence.
    pub(super) fn skill_row(&self, item: &SkillItem<'_>, cx: &mut Context<Self>) -> AnyElement {
        skill_frame("skill-row", item, cx)
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .truncate()
                            .child(item.name()),
                    )
                    .child(description(item.description, 1, cx))
                    .child(dates(item, cx)),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .max_w(px(420.))
                    .child(presence(item, cx)),
            )
            .into_any_element()
    }

    /// A card: name, two description lines, dates, then presence.
    pub(super) fn skill_card(&self, item: &SkillItem<'_>, cx: &mut Context<Self>) -> AnyElement {
        skill_frame("skill-card", item, cx)
            .flex_col()
            .gap_2()
            .p_3()
            .min_h(px(132.))
            .child(
                div()
                    .min_w_0()
                    .font_weight(FontWeight::MEDIUM)
                    .truncate()
                    .child(item.name()),
            )
            .child(div().flex_1().child(description(item.description, 2, cx)))
            .child(dates(item, cx))
            .child(presence(item, cx))
            .into_any_element()
    }
}

/// The bordered frame of a row or card; skill details are not available yet, so it has no action.
fn skill_frame(kind: &str, item: &SkillItem<'_>, cx: &App) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(format!("{kind}-{}", item.skill.id))
        .flex()
        .min_w_0()
        .border_1()
        .rounded(cx.theme().radius)
        .border_color(cx.theme().colors.border)
}

/// "Imported 3 days ago · Modified 2 hours ago"; each part's tooltip shows the exact time.
fn dates(item: &SkillItem<'_>, cx: &App) -> impl IntoElement {
    let now = Utc::now();
    let parts = [
        ("imported", "Imported", item.imported),
        ("modified", "Modified", item.modified),
    ];
    let mut row = h_flex()
        .gap_1()
        .text_xs()
        .text_color(cx.theme().muted_foreground);
    for (index, (key, label, time)) in parts
        .into_iter()
        .filter_map(|(key, label, time)| Some((key, label, time?)))
        .enumerate()
    {
        let tooltip = exact(time);
        row = row.when(index > 0, |row| row.child("·")).child(
            div()
                .id(format!("{key}-{}", item.skill.id))
                .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                .child(format!("{label} {}", relative(time, now))),
        );
    }
    row
}

fn description(text: &str, lines: usize, cx: &App) -> impl IntoElement {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .map(|text| {
            if lines == 1 {
                text.truncate()
            } else {
                text.line_clamp(lines)
            }
        })
        .child(text.to_owned())
}

/// Workspace chips, each holding the avatars of the agents that read the copy there.
fn presence(item: &SkillItem<'_>, cx: &mut Context<NativeView>) -> AnyElement {
    let muted = |text: &'static str| {
        div()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(text)
            .into_any_element()
    };
    let places = match item.places {
        None => return muted("Not scanned yet"),
        Some([]) => return muted("Not in any workspace"),
        Some(places) => places,
    };
    let hidden = &places[places.len().min(SHOWN_PLACES)..];
    h_flex()
        .flex_wrap()
        .gap_1()
        .children(
            places
                .iter()
                .take(SHOWN_PLACES)
                .map(|place| place_chip(item, place, cx)),
        )
        .when(!hidden.is_empty(), |row| {
            let names = hidden
                .iter()
                .map(|place| place.name.clone())
                .collect::<Vec<_>>()
                .join("\n");
            row.child(
                div()
                    .id(format!("places-more-{}", item.skill.id))
                    .flex_shrink_0()
                    .tooltip(move |window, cx| Tooltip::new(names.clone()).build(window, cx))
                    .child(Tag::secondary().small().child(format!("+{}", hidden.len()))),
            )
        })
        .into_any_element()
}

fn place_chip(item: &SkillItem<'_>, place: &Place, cx: &mut Context<NativeView>) -> AnyElement {
    let route = match &place.workspace {
        WorkspaceRef::System => Route::System,
        WorkspaceRef::Added(id) => Route::Workspace(id.clone()),
    };
    let path = place.path.clone();
    let key = match &place.workspace {
        WorkspaceRef::System => format!("{}-system", item.skill.id),
        WorkspaceRef::Added(id) => format!("{}-{id}", item.skill.id),
    };
    div()
        .id(format!("place-{key}"))
        .flex_shrink_0()
        .whitespace_nowrap()
        .cursor_pointer()
        .tooltip(move |window, cx| Tooltip::new(path.clone()).build(window, cx))
        .on_click(cx.listener(move |view, _, window, cx| {
            cx.stop_propagation();
            view.navigate(route.clone(), window, cx);
        }))
        .child(
            Tag::secondary()
                .small()
                .gap_1()
                .child(div().flex_shrink_0().child(place.name.clone()))
                .child(readers(&key, place, cx)),
        )
        .into_any_element()
}

/// Agent avatars, Universal first, with a tooltip naming every agent.
fn readers(key: &str, place: &Place, cx: &App) -> impl IntoElement {
    let names = place
        .readers
        .iter()
        .map(|reader| match reader {
            Reader::Universal(names) => format!("Universal: {}", names.join(", ")),
            Reader::Agent { name, .. } => name.clone(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let extra = place.readers.len().saturating_sub(SHOWN_READERS);
    h_flex()
        .id(format!("readers-{key}"))
        .gap_0p5()
        .tooltip(move |window, cx| Tooltip::new(names.clone()).build(window, cx))
        .child(
            AvatarGroup::new()
                .with_size(Size::XSmall)
                .limit(SHOWN_READERS)
                // The group lays out from its right edge; its own width leaves out the overlap.
                .w(px(group_width(place.readers.len())))
                .flex_shrink_0()
                .children(place.readers.iter().map(|reader| avatar(reader, cx))),
        )
        .when(extra > 0, |row| row.child(format!("+{extra}")))
}

/// The width of up to `SHOWN_READERS` 16px avatars, each overlapping the previous by 30%.
fn group_width(readers: usize) -> f32 {
    let shown = readers.clamp(1, SHOWN_READERS) as f32;
    16. + (shown - 1.) * 16. * 0.7
}

fn avatar(reader: &Reader, cx: &App) -> Avatar {
    match reader {
        Reader::Universal(_) => glyph_avatar(app_icon("layers"), cx),
        Reader::Agent { id, .. } => agent_avatar(id, cx),
    }
}
