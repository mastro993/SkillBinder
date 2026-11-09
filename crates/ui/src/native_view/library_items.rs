//! Library skills as rows inside one card, or as grid tiles on their own surfaces. Each shows its
//! name, its update, drift, and validation glyphs, its folder as a tag while Library lists every
//! skill, and its description, then what [`RowEnd`] says: on Library where the skill is on, on a
//! workspace's page its activation there and the agents it is applied for.

use super::controls::chip::tag;
use super::controls::{app_icon, text, tooltip::tip};
use super::library_copies::Placement;
use super::library_drift::drift_glyph;
use super::library_validation::validation_glyph;
use super::sidebar_row::{activate, with_menu};
use super::snapshot_query::SkillPlaces;
use super::{NativeView, Route};
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, tag::Tag, tooltip::Tooltip, v_flex};
use gpui_kit::{
    AnyElement, App, Context, DefiniteLength, Div, FontWeight, Hsla, InteractiveElement,
    IntoElement, ParentElement, Stateful, StatefulInteractiveElement, Styled, canvas, div,
    prelude::FluentBuilder, px,
};
use skillbinder_proto::{Overlap, Skill, SkillDetails, SkillId};
use skillbinder_theme::library::{
    CARD_RADIUS, DIVIDER_INSET, ROW_GAP, ROW_MIN_HEIGHT, ROW_PADDING_X, TILE_GAP, TILE_MIN_HEIGHT,
    TILE_PADDING, TILE_RADIUS,
};

/// The hover group of a row or tile, whose dashed `+` shows while it is hovered.
pub(super) const ITEM_GROUP: &str = "skill-item";
/// Opacity of an inactive skill's name, tag, and description.
const DIM: f32 = 0.5;
/// The size of a skill's name, whose line sets a row's and a tile's height.
pub(super) const SLUG_SIZE: f32 = 14.;

/// One Library skill with its details, folder, and copies.
pub(super) struct SkillItem<'a> {
    pub skill: &'a Skill,
    /// The skill's description and validation; `None` while the snapshot has none.
    pub details: Option<&'a SkillDetails>,
    /// The skill's folder, shown while Library lists every skill.
    pub folder: Option<&'a str>,
    /// The skill's source has a newer version.
    pub update: bool,
    /// The skill's content differs from its record, so it shows a warning glyph.
    pub drifted: bool,
    /// The open workspace holds the skill for an agent that the active preset holds it for too,
    /// so it shows a warning badge saying how that agent treats the two copies.
    pub overlap: Option<Overlap>,
    /// Where the skill's copies stand; `None` while it has none.
    pub placement: Option<&'a Placement>,
    pub end: RowEnd<'a>,
}

/// What a row or tile shows at its end. Every use matches it exhaustively, so a new kind of row
/// fails to compile until it is drawn.
pub(super) enum RowEnd<'a> {
    /// Library: where the skill is on, which opens the popover that changes it.
    Places(SkillPlaces<'a>),
    /// A workspace's page: the Active checkbox and agents button, and the agents line.
    Activation,
}

impl SkillItem<'_> {
    fn name(&self) -> String {
        self.skill.id.to_string()
    }

    fn description(&self) -> &str {
        self.details
            .map_or("", |details| details.description.as_str())
    }

    /// An inactive skill on Library draws its text dimmed.
    fn dimmed(&self) -> bool {
        match &self.end {
            RowEnd::Places(places) => !places.is_active(),
            RowEnd::Activation => false,
        }
    }

    /// The copies that are not applied, or ask, or were not removed.
    fn unsettled(&self) -> usize {
        self.placement
            .map_or(0, |placement| placement.unsettled.len())
    }
}

/// How a list or grid lays out its items.
#[derive(Clone, Copy)]
pub(super) struct Layout {
    /// Too narrow for a workspace's name, so one chip counts the workspaces.
    pub narrow: bool,
    /// The vertical padding that makes up for GPUI's shorter name line (`text::line_shortfall`),
    /// as a fraction of the parent's width, since GPUI rounds every point length to the device
    /// pixel but keeps fractions: a row's padding, half the shortfall over 10 points, or the
    /// padding around a tile's name line, half the shortfall.
    pub pad_y: DefiniteLength,
}

impl NativeView {
    /// The list's card: one bordered surface holding every row, divided by inset hairlines.
    pub(super) fn skill_list(
        &self,
        items: &[SkillItem<'_>],
        layout: Layout,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let last = items.len().saturating_sub(1);
        v_flex()
            .border_1()
            .rounded(px(CARD_RADIUS))
            .border_color(cx.theme().border)
            .bg(surface(cx))
            .children(
                items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| self.skill_row(item, index, index == last, layout, cx)),
            )
            .into_any_element()
    }

    /// A row of the list card: name and one description line, with its end at the trailing
    /// edge. The first and last rows round their outer corners so the hover fill follows the
    /// card's.
    fn skill_row(
        &self,
        item: &SkillItem<'_>,
        index: usize,
        last: bool,
        layout: Layout,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let owner = format!("skill-row-{}", item.skill.id);
        let hover = cx.theme().muted;
        let inner = px(CARD_RADIUS - 1.);
        let row = div()
            .id(owner.clone())
            .group(ITEM_GROUP)
            .relative()
            .flex()
            .items_center()
            .gap(px(ROW_GAP))
            .w_full()
            .min_h(px(ROW_MIN_HEIGHT))
            .px(px(ROW_PADDING_X))
            .py(layout.pad_y)
            .cursor_pointer()
            .hover(move |row| row.bg(hover))
            .when(index == 0, |row| row.rounded_t(inner))
            .when(last, |row| row.rounded_b(inner))
            .when(index > 0, |row| row.child(divider(cx)))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(2.))
                    .child(self.title_line(item, None, cx))
                    .child(description(item, 1, cx))
                    .when(matches!(item.end, RowEnd::Activation), |main| {
                        main.child(div().pt(px(4.)).child(self.agents_line(item, cx)))
                    }),
            )
            .child(self.row_end(item, false, layout, cx));
        frame(self, row, item, owner, cx)
    }

    /// A grid tile: name, two description lines, then its end in the footer.
    pub(super) fn skill_card(
        &self,
        item: &SkillItem<'_>,
        layout: Layout,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let owner = format!("skill-card-{}", item.skill.id);
        // A tile wears the list card's surface and hairline; only the hover fill differs.
        let (rest, hover, border) = (surface(cx), cx.theme().muted, cx.theme().border);
        let card = div()
            .id(owner.clone())
            .group(ITEM_GROUP)
            .relative()
            .flex()
            .flex_col()
            .gap(px(TILE_GAP))
            .w_full()
            .min_w_0()
            .min_h(px(TILE_MIN_HEIGHT))
            .p(px(TILE_PADDING))
            .rounded(px(TILE_RADIUS))
            .bg(rest)
            .border_1()
            .border_color(border)
            .cursor_pointer()
            .hover(move |card| card.bg(hover))
            .child(self.title_line(item, Some(layout.pad_y), cx))
            .child(description(item, 2, cx))
            .when(matches!(item.end, RowEnd::Activation), |card| {
                card.child(self.agents_line(item, cx))
            })
            .child(
                div()
                    .mt_auto()
                    .pt(px(6.))
                    .min_h(px(32.))
                    .flex()
                    .items_center()
                    .child(self.row_end(item, true, layout, cx)),
            );
        frame(self, card, item, owner, cx)
    }

    /// The name with its glyphs and folder tag, one line high, padded by `pad_y` in a tile.
    fn title_line(&self, item: &SkillItem<'_>, pad_y: Option<DefiniteLength>, cx: &App) -> Div {
        let dim = item.dimmed();
        let slug = text::sized(div(), SLUG_SIZE)
            .min_w_0()
            .font_weight(FontWeight::SEMIBOLD)
            .truncate()
            .when(dim, |slug| slug.opacity(DIM))
            .child(item.name());
        h_flex()
            .min_w_0()
            .gap(px(6.))
            .when_some(pad_y, Styled::py)
            .child(slug)
            .children(name_marks(item, cx))
            .when_some(item.folder, |line, folder| {
                line.child(tag(folder.to_owned(), cx).when(dim, |tag| tag.opacity(DIM)))
            })
    }

    /// The end of a row or tile: where the skill is on, or its activation in the workspace.
    fn row_end(
        &self,
        item: &SkillItem<'_>,
        grid: bool,
        layout: Layout,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match &item.end {
            RowEnd::Places(places) => {
                let unsettled = item.unsettled();
                h_flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.))
                    .when(unsettled > 0, |end| {
                        end.child(self.unsettled_glyph(&item.skill.id, unsettled, cx))
                    })
                    .child(self.place_trigger(places, grid, layout.narrow, cx))
                    .into_any_element()
            }
            RowEnd::Activation => self.activation_controls(&item.skill.id, cx),
        }
    }
}

/// Opens the skill's page on click or Enter, and its menu on right-click. While the skill's
/// popover is open, the row records where it was painted, so the popover can open above it.
fn frame(
    view: &NativeView,
    element: Stateful<Div>,
    item: &SkillItem<'_>,
    owner: String,
    cx: &mut Context<NativeView>,
) -> AnyElement {
    let id = item.skill.id.clone();
    let route = Route::Skill(id.clone());
    let element = element.when_some(view.open_popover_cells(&id), |element, cells| {
        element.child(
            canvas(
                move |bounds, _, _| cells.item.set(Some(bounds)),
                |_, (), _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    });
    let element = activate(element, item.name(), cx, move |view, window, cx| {
        view.navigate(route.clone(), window, cx);
    });
    with_menu(element, owner, true, view, cx, move |view, _, cx| {
        view.skill_menu(&id, cx)
    })
    // The wrapper is the grid item, so it must take the column's width rather than the text's.
    .w_full()
    .min_w_0()
    .into_any_element()
}

/// A hairline at the top of every row but the first, inset from both card edges so it starts
/// where the name starts.
fn divider(cx: &App) -> AnyElement {
    div()
        .absolute()
        .left(px(DIVIDER_INSET))
        .right(px(DIVIDER_INSET))
        .top(px(0.))
        .h(px(1.))
        .bg(cx.theme().border)
        .into_any_element()
}

/// The list card's surface, which the agent avatars ring themselves in.
pub(super) fn surface(cx: &App) -> Hsla {
    skillbinder_theme::library::surface(cx.theme())
}

/// The glyphs after a skill's name, for its update, drift, and validation, then its overlap badge.
fn name_marks(item: &SkillItem<'_>, cx: &App) -> Vec<AnyElement> {
    let id = &item.skill.id;
    // Redesigned surfaces show updates in the info color; the workspace page keeps warning.
    let tone = match item.end {
        RowEnd::Places(_) => cx.theme().info,
        RowEnd::Activation => cx.theme().warning,
    };
    [
        item.update.then(|| update_glyph(id, tone)),
        item.drifted.then(|| drift_glyph(id, cx)),
        validation_glyph(id, item.details, cx),
        item.overlap.map(|overlap| overlap_badge(item, overlap)),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The warning badge of a skill the open workspace holds for an agent that the active preset holds
/// it for too: the agent may load it twice, or it keeps one copy, which the reviewed order names
/// for every agent but the ones no source ranks.
fn overlap_badge(item: &SkillItem<'_>, overlap: Overlap) -> AnyElement {
    let (label, explanation) = match overlap {
        Overlap::Twice => (
            "Also in your agents",
            "An agent reads this skill both here and in your agents, so it may load it twice.",
        ),
        Overlap::Hidden => (
            "Hidden by your agents",
            "An agent reads this skill in both places but keeps your agents' copy, so this copy is ignored.",
        ),
        Overlap::Overrides => (
            "Overrides your agents",
            "An agent reads this skill in both places but loads this copy, so your agents' copy is ignored.",
        ),
        Overlap::OneCopy => (
            "One copy is used",
            "An agent reads this skill in both places but keeps one copy, and no source says which.",
        ),
    };
    div()
        .id(format!("overlap-{}", item.skill.id))
        .flex_shrink_0()
        .child(Tag::warning().small().child(label))
        .tooltip(move |window, cx| Tooltip::new(explanation).build(window, cx))
        .into_any_element()
}

/// The 14 px glyph of a skill whose source has an update, in `tone`. It keeps full opacity on a
/// dimmed row.
fn update_glyph(id: &SkillId, tone: Hsla) -> AnyElement {
    let glyph = div()
        .id(format!("update-{id}"))
        .flex_shrink_0()
        .child(app_icon("upload-circle").small().text_color(tone));
    tip(glyph, "Update available").into_any_element()
}

/// The description, muted, on one line in a row or two in a tile, dimmed on an inactive skill.
fn description(item: &SkillItem<'_>, lines: usize, cx: &App) -> impl IntoElement {
    text::sized(div(), 12.5)
        .min_w_0()
        .text_color(cx.theme().muted_foreground)
        .when(item.dimmed(), |text| text.opacity(DIM))
        .map(|text| {
            if lines == 1 {
                text.truncate()
            } else {
                text.line_clamp(lines).text_ellipsis()
            }
        })
        .child(item.description().to_owned())
}
