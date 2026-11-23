//! One tile of Choose your agents (design P3): the logo, or an avatar stack as the import summary
//! draws it (Universal's first three members), then the name and the skill folder, or
//! Universal's agents, in muted text.

use super::super::super::NativeView;
use super::super::super::controls::{agent_detail, agent_logo, reader_logo};
use super::super::super::library_copies::{AgentRow, Reader, shown_dir};
use super::super::tile::Tile;
use gpui_kit::component::ActiveTheme;
use gpui_kit::{AnyElement, Context, IntoElement};
use skillbinder_design::{AvatarStack, Logo};
use std::rc::Rc;

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
        let logo = AvatarStack::new(logos)
            .limit(MEMBERS)
            .ring(cx.theme().background);
        let mono = matches!(row.reader, Reader::Agent(_));
        let id = match &row.reader {
            // Not `onboarding-agent-universal`: that is the registry's own Universal agent.
            Reader::Universal(_) => "onboarding-agents-universal".to_owned(),
            Reader::Agent(id) => format!("onboarding-agent-{id}"),
        };
        let dir = row.dir.clone();
        let tile = Tile {
            detail: Some(agent_detail(format!("{id}-detail"), detail, mono, cx)),
            id,
            name: row.name.clone().into(),
            logo: logo.into_any_element(),
            chosen,
            corner: None,
        };
        self.tile(
            tile,
            Some(Rc::new(move |view, cx| view.toggle_agent(&dir, cx))),
            cx,
        )
    }
}
