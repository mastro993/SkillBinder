//! One tile of Choose your agents (design P3). Kit has no selectable tile, so it is a focusable
//! bordered box with the theme radius: the logo at its top left, an avatar stack as the import
//! summary draws it (Universal's first three members), Kit's checkbox box at its top right,
//! then the name and the skill folder, or Universal's agents, in muted text. A chosen tile has a
//! 1-point outline, the foreground mixed into the border, and a checked box (design P6). The tile
//! is the checkbox: click, Enter, or Space toggles it.

use super::super::super::NativeView;
use super::super::super::controls::{agent_detail, agent_logo, reader_logo};
use super::super::super::library_copies::{AgentRow, Reader, shown_dir};
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Role,
    StatefulInteractiveElement, Styled, Toggled, div, px,
};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{AvatarStack, Checkbox, Logo, Text};

/// The tile's inner margin, inside its outline.
const PADDING: f32 = space::LG;
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
        let (logos, detail) = match &row.reader {
            Reader::Universal(names) => {
                let logos: Vec<Logo> = row
                    .ids
                    .iter()
                    .take(MEMBERS)
                    .map(|id| agent_logo(id))
                    .collect();
                (logos, names.join(", "))
            }
            Reader::Agent(_) => (vec![reader_logo(&row.reader)], shown_dir(&row.dir, home)),
        };
        let logo = AvatarStack::new(logos).limit(MEMBERS).ring(background);
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
            .gap(px(space::MD))
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
                    .gap(px(space::XXS))
                    .child(
                        Text::labeled(format!("{id}-name"), row.name.clone())
                            .level(text::TITLE_SM)
                            .truncate(),
                    )
                    .child(agent_detail(format!("{id}-detail"), detail, mono, cx)),
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
