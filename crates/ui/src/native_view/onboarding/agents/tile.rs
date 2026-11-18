//! One tile of Choose your agents (design P3). Kit has no selectable tile, so it is a focusable
//! bordered box with the theme radius: the logo at its top left, Kit avatars as the import summary
//! draws them (Universal's first three members in a stack), Kit's checkbox box at its top right,
//! then the name and the skill folder, or Universal's agents, in muted text. A chosen tile has a
//! 1-point outline, the foreground mixed into the border, and a checked box (design P6). The tile
//! is the checkbox: click, Enter, or Space toggles it.

use super::super::super::NativeView;
use super::super::super::controls::{agent_logo, reader_logo};
use super::super::super::library_avatars::{Ring, avatar, stack_of};
use super::super::super::library_copies::{AgentRow, Reader, shown_dir};
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Role,
    StatefulInteractiveElement, Styled, Toggled, div, prelude::FluentBuilder, px,
};
use skillbinder_design::tokens::text;
use skillbinder_design::{Checkbox, Logo, Text, tooltip};

/// The tile's inner margin, inside its outline.
const PADDING: f32 = 12.;
/// How many of Universal's members its logo stack shows.
const MEMBERS: usize = 3;

impl NativeView {
    /// The tile of `row`, whose global skill directory, shown inside `home`, is chosen when
    /// `chosen`.
    pub(super) fn agent_tile(
        &self,
        row: &AgentRow,
        chosen: bool,
        home: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let (background, foreground, border) = (theme.background, theme.foreground, theme.border);
        let ring = Ring {
            rest: background,
            hovered: None,
        };
        let (logo, detail) = match &row.reader {
            Reader::Universal(names) => {
                let logos: Vec<Logo> = row
                    .ids
                    .iter()
                    .take(MEMBERS)
                    .map(|id| agent_logo(id))
                    .collect();
                (stack_of(&logos, MEMBERS, ring, cx), names.join(", "))
            }
            Reader::Agent(_) => (
                div().child(avatar(reader_logo(&row.reader), ring, cx)),
                shown_dir(&row.dir, home),
            ),
        };
        let mono = matches!(row.reader, Reader::Agent(_));
        let id = match &row.reader {
            // Not `onboarding-agent-universal`: that is the registry's own Universal agent.
            Reader::Universal(_) => "onboarding-agents-universal".to_owned(),
            Reader::Agent(id) => format!("onboarding-agent-{id}"),
        };
        let dir = row.dir.clone();
        let key_dir = dir.clone();
        div()
            .id(id.clone())
            .role(Role::CheckBox)
            .aria_label(row.name.clone())
            .aria_toggled(if chosen {
                Toggled::True
            } else {
                Toggled::False
            })
            .tab_index(0)
            .relative()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(8.))
            .p(px(PADDING))
            .border_1()
            .border_color(if chosen {
                border.blend(foreground.opacity(0.45))
            } else {
                border
            })
            .rounded(theme.radius_lg)
            .bg(background)
            .cursor_pointer()
            .focus_visible(|style| style.border_color(theme.ring))
            .child(logo)
            .child(
                v_flex()
                    .w_full()
                    .min_w_0()
                    .gap(px(2.))
                    .child(
                        Text::labeled(format!("{id}-name"), row.name.clone())
                            .level(text::TITLE_SM)
                            .truncate(),
                    )
                    .child(
                        div()
                            .id(format!("{id}-detail"))
                            .truncate()
                            .text_size(px(12.))
                            .text_color(theme.muted_foreground)
                            .when(mono, |text| {
                                text.font_family(theme.mono_font_family.clone())
                            })
                            .child(detail.clone())
                            .when(!mono, |text| text.tooltip(tooltip(detail))),
                    ),
            )
            .child(check(&id, chosen))
            .on_click(cx.listener(move |view, _, _, cx| view.toggle_agent(&dir, cx)))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    view.toggle_agent(&key_dir, cx);
                    cx.stop_propagation();
                }
            }))
            .into_any_element()
    }
}

/// The check at the tile's top right: Kit's checkbox box alone, since the tile is the checkbox.
fn check(id: &str, chosen: bool) -> impl IntoElement {
    Checkbox::new(format!("{id}-check"))
        .checked(chosen)
        .indicator()
        .absolute()
        .top(px(PADDING))
        .right(px(PADDING))
}
