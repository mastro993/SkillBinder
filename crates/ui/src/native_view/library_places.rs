//! Where a library skill is on, at a Library row's trailing edge or in a grid tile's footer: one
//! button holding your agents' logo stack, a hairline, the workspaces' chips, and a chevron, or a
//! dashed `+` circle that shows while an inactive skill's row is hovered. Either opens the skill's
//! popover, except while the app is blocked, when it is inert. The preset chip and the pill are
//! shared with the popover and the other pages.

use super::NativeView;
use super::controls::avatar::{self, Fill, Ring};
use super::controls::chip::{chip, dot};
use super::controls::exact_text::{exact_text, sized};
use super::controls::logo::{Ink, Logo};
use super::controls::{pill, tooltip::tip};
use super::library_items::{ITEM_GROUP, surface};
use super::preset_colors::preset_tint;
use super::snapshot_query::{SkillPlaces, WorkspacePlace};
use super::workspace_icons::sized_icon;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{
    AnyElement, App, Context, Div, ElementId, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, Role, SharedString, Stateful, StatefulInteractiveElement, Styled, Toggled,
    canvas, div, prelude::FluentBuilder, px,
};
use skillbinder_proto::Preset;
use skillbinder_theme::MEDIUM;
use skillbinder_theme::menu::HIGHLIGHT_INK;

/// The hover group of the placement button.
const PLACE_GROUP: &str = "place";
/// The widest a workspace's name shows in its chip.
const CHIP_NAME_MAX: f32 = 140.;

impl NativeView {
    /// The skill's placement button; clicking it, or Enter or Space while it has focus, opens or
    /// closes the popover and never the skill's page. While a change runs or the progress strip
    /// is up it does nothing, and shows no hover or hand cursor.
    pub(super) fn place_trigger(
        &self,
        places: &SkillPlaces<'_>,
        grid: bool,
        narrow: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let skill = places.skill.clone();
        let open = self.popover_open(&skill);
        let highlight = cx.theme().foreground.alpha(HIGHLIGHT_INK);
        let toggle = skill.clone();
        let key = skill.clone();
        let button = div()
            .id(format!("place-{skill}"))
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .min_h(px(26.))
            .rounded(px(6.))
            .tab_index(0)
            .role(Role::Button)
            .aria_expanded(open)
            .when(open, |button| button.bg(highlight))
            .when_some(self.open_popover_cells(&skill), |button, cells| {
                button.child(
                    canvas(
                        move |bounds, _, _| cells.trigger.set(Some(bounds)),
                        |_, (), _, _| {},
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                )
            })
            // Inert, it still takes the click, so the row behind it does not open the skill.
            .on_click(cx.listener(move |view, _, window, cx| {
                cx.stop_propagation();
                view.toggle_place_popover(&toggle, grid, window, cx);
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    cx.stop_propagation();
                    view.toggle_place_popover(&key, grid, window, cx);
                }
            }))
            .when(!self.blocked(), |button| {
                button
                    .group(PLACE_GROUP)
                    .cursor_pointer()
                    .hover(move |style| style.bg(highlight))
            });
        if places.is_active() {
            active_button(button, places, open, narrow, cx)
        } else {
            turn_on_button(button, &skill.to_string(), open, cx)
        }
    }
}

/// The button of an active skill: your agents' stack, a hairline, the workspaces' chips, and a
/// chevron.
fn active_button(
    button: Stateful<Div>,
    places: &SkillPlaces<'_>,
    open: bool,
    narrow: bool,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let highlight = theme.foreground.alpha(HIGHLIGHT_INK);
    let (rest, hover) = (surface(cx), theme.muted);
    // The rings take the fill behind them: the row's, with the highlight over it while the
    // button is hovered or open.
    let ring = Ring {
        fill: Fill {
            rest,
            hover: Some((ITEM_GROUP.into(), hover)),
        },
        tint: Some(Fill {
            rest: if open {
                highlight
            } else {
                gpui_kit::transparent_black()
            },
            hover: Some((PLACE_GROUP.into(), highlight)),
        }),
    };
    let held: Vec<_> = places.held_agents().collect();
    let workspaces: Vec<_> = places.holding_workspaces().collect();
    let stack = (!held.is_empty()).then(|| {
        let logos: Vec<Logo> = held
            .iter()
            .map(|entry| Logo::reader(&entry.row.reader))
            .collect();
        let names = if places.all_agents() {
            "All agents".to_owned()
        } else {
            held.iter()
                .map(|entry| entry.row.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let stack = div()
            .id(format!("place-agents-{}", places.skill))
            .child(avatar::stack(&logos, &ring, cx));
        tip(stack, format!("Your agents\n{names}"))
    });
    let chevron = Ink {
        rest: if open {
            theme.foreground
        } else {
            theme.muted_foreground
        },
        hover: Some((PLACE_GROUP, theme.foreground)),
    };
    button
        .gap(px(6.))
        .pl(px(6.))
        .pr(px(4.))
        .aria_label(format!("Change where {} is on", places.skill))
        .children(stack)
        .when(!held.is_empty() && !workspaces.is_empty(), |button| {
            button.child(div().flex_none().w(px(1.)).h(px(14.)).bg(theme.border))
        })
        .children(workspace_chip(places, &workspaces, narrow, cx))
        .child(
            div()
                .flex_none()
                .opacity(0.7)
                .child(Logo::Glyph("icons/arrow-down.svg").element(12., chevron, false)),
        )
        .into_any_element()
}

/// One chip naming the only workspace that holds the skill, or one counting the workspaces when
/// several do or the list is narrow.
fn workspace_chip(
    places: &SkillPlaces<'_>,
    workspaces: &[&WorkspacePlace<'_>],
    narrow: bool,
    cx: &App,
) -> Option<Stateful<Div>> {
    let muted = cx.theme().muted_foreground;
    let base = || {
        chip(format!("place-workspaces-{}", places.skill), 20., cx)
            .gap(px(4.))
            .pl(px(5.))
            .pr(px(7.))
    };
    match workspaces {
        [] => None,
        [only] if !narrow => Some(tip(
            base().child(sized_icon(only.workspace, 13., cx)).child(
                div()
                    .flex()
                    .min_w_0()
                    .max_w(px(CHIP_NAME_MAX))
                    .child(exact_text("name", only.workspace.name.clone()).truncate()),
            ),
            format!("{}\n{}", only.workspace.name, only.held_names()),
        )),
        several => Some(tip(
            base()
                .child(Logo::Glyph("icons/code-folder.svg").element(13., muted.into(), false))
                .child(exact_text("count", several.len().to_string())),
            several
                .iter()
                .map(|place| format!("{} · {}", place.workspace.name, place.held_names()))
                .collect::<Vec<_>>()
                .join("\n"),
        )),
    }
}

/// The 26 px dashed `+` circle of an inactive skill, shown while its row or tile is hovered, it
/// has keyboard focus, or its popover is open.
fn turn_on_button(button: Stateful<Div>, slug: &str, open: bool, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let plus = Ink {
        rest: if open {
            theme.foreground
        } else {
            theme.muted_foreground
        },
        hover: Some((PLACE_GROUP, theme.foreground)),
    };
    let button = pill::dashed(button, cx)
        .justify_center()
        .size(px(26.))
        .rounded_full()
        .aria_label(format!("Turn on {slug}"))
        .opacity(if open { 1. } else { 0. })
        .group_hover(ITEM_GROUP, |style| style.opacity(1.))
        .focus_visible(|style| style.opacity(1.))
        .child(Logo::Glyph("icons/plus.svg").element(14., plus, false));
    tip(button, "Turn on").into_any_element()
}

/// The active preset's chip: its color dot, muted without a color, then its name. It says where
/// changes for your agents are saved and is not clickable.
pub(super) fn preset_chip(preset: Option<&Preset>, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let tint = preset
        .and_then(|preset| preset.color)
        .map_or(theme.muted_foreground, |color| {
            preset_tint(color, theme.is_dark())
        });
    let name = preset.map_or_else(|| "Default".to_owned(), |preset| preset.name.clone());
    let chip = chip("place-preset", 22., cx)
        .px(px(8.))
        .gap(px(5.))
        .child(dot(tint))
        .child(exact_text("name", name.clone()));
    tip(chip, format!("Saved to {name}")).into_any_element()
}

/// A 26 by 24 pill toggling one directory, with its tooltip and accessible name; the caller adds
/// its click, unless the pill is `inert`.
pub(super) fn place_pill(
    id: impl Into<ElementId>,
    logo: Logo,
    on: bool,
    inert: bool,
    tooltip: impl Into<SharedString>,
    cx: &App,
) -> Stateful<Div> {
    let tooltip = tooltip.into();
    let label = tooltip.lines().next().unwrap_or_default().to_owned();
    tip(
        pill::pill(id, logo, on, inert, cx)
            .role(Role::Button)
            .aria_toggled(if on { Toggled::True } else { Toggled::False })
            .aria_label(label),
        tooltip,
    )
}

/// An 11.5 px medium muted section label, its line `id`.
pub(super) fn section_label(id: &'static str, label: &'static str, cx: &App) -> Div {
    sized(div(), 11.5)
        .font_weight(MEDIUM)
        .text_color(cx.theme().muted_foreground)
        .child(exact_text(id, label))
}
