//! The Use SkillBinder with your agents step (spec's agents step, ADR 0027): the lead, a muted
//! line while Set up would install the command, then two columns of tiles, one per agent Connect
//! can connect whose agent is chosen, all ticked. Set up installs the command when it is missing,
//! then connects each ticked agent in turn; with every one connected, the step moves on after a
//! moment, and with a failure it stays, offering Try again. Skip, beside the primary as in the
//! prototype, Continue, or a completed Set up settles the step.

#[cfg(test)]
mod tests;

use super::super::controls::agent_logo;
use super::super::git_setup::READY_FOR;
use super::super::{Effect, NativeView};
use super::StepView;
use super::offers::Link;
use gpui_kit::component::{ActiveTheme, h_flex, v_flex};
use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled, div, px,
};
use skillbinder_design::tokens::{space, text};
use skillbinder_design::{
    AvatarStack, Button, IconSize, Notice, NoticeSize, SelectTile, Spinner, Status, Text, TextTone,
};
use skillbinder_proto::{
    AgentOption, AppResult, CommandLineState, McpClient, McpClientState, McpClientStatus,
    McpConnect,
};
use std::collections::{BTreeMap, BTreeSet};

const TITLE: &str = "Use SkillBinder with your agents";
const LEAD: &str = "Your agents can find, install, and turn on skills for you. You can change \
                    this later in Settings.";
const INSTALLS: &str = "This also installs the skillbinder command.";
/// The tiles' order: the spec's, not Settings'.
const ORDER: [McpClient; 4] = [
    McpClient::ClaudeCode,
    McpClient::Codex,
    McpClient::Cursor,
    McpClient::VsCode,
];
/// Tiles per row.
const COLUMNS: usize = 2;

/// The agents the step shows: those Connect can connect whose agent is chosen, each with whether
/// it is connected already, in [`ORDER`].
fn shown(clients: &[McpClientStatus], agents: &[AgentOption]) -> Vec<(McpClient, bool)> {
    ORDER
        .into_iter()
        .filter_map(|client| {
            let status = clients.iter().find(|status| status.client == client)?;
            let chosen = agents
                .iter()
                .any(|agent| agent.id == client.agent() && agent.chosen);
            let connected = match status.state {
                McpClientState::Connected => true,
                McpClientState::NotConnected => false,
                McpClientState::NotFound { .. } => return None,
            };
            chosen.then_some((client, connected))
        })
        .collect()
}

/// The step's primary action.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Primary {
    /// Connects these agents.
    SetUp(Vec<McpClient>),
    /// Connects these agents again, after a failure.
    TryAgain(Vec<McpClient>),
    /// Every agent shown is connected, or none is shown: settles the step.
    Continue,
}

/// What the primary does: connects the ticked agents not connected yet, or, with none left,
/// leads on.
fn primary(
    shown: &[(McpClient, bool)],
    unticked: &BTreeSet<McpClient>,
    links: &BTreeMap<McpClient, Link>,
) -> Primary {
    let connected = |client: &McpClient, read: bool| {
        read || matches!(links.get(client), Some(Link::Connected | Link::Opened))
    };
    if shown.iter().all(|(client, read)| connected(client, *read)) {
        return Primary::Continue;
    }
    let targets: Vec<McpClient> = shown
        .iter()
        .filter(|(client, read)| !connected(client, *read) && !unticked.contains(client))
        .map(|(client, _)| *client)
        .collect();
    if links.values().any(|link| matches!(link, Link::Failed(_))) {
        Primary::TryAgain(targets)
    } else {
        Primary::SetUp(targets)
    }
}

impl NativeView {
    /// The step; gone back to once settled, it offers no Skip, and the frame's Continue is the
    /// primary.
    pub(super) fn connect_step(&mut self, settled: bool, cx: &mut Context<Self>) -> StepView {
        self.read_command(cx);
        self.read_clients(cx);
        let muted = cx.theme().muted_foreground;
        let mut body = vec![div().text_color(muted).child(LEAD).into_any_element()];
        let missing = self
            .offers
            .command
            .as_ref()
            .is_some_and(|status| status.state == CommandLineState::NotInstalled);
        if missing {
            body.push(
                Text::labeled("onboarding-connect-installs", INSTALLS)
                    .level(text::BODY_SM)
                    .tone(TextTone::Muted)
                    .into_any_element(),
            );
        }
        let Some(clients) = &self.offers.clients else {
            body.push(
                Notice::new("onboarding-connect-reading", "Checking your agents…")
                    .size(NoticeSize::Md)
                    .turning("loading")
                    .into_any_element(),
            );
            return StepView {
                title: TITLE,
                wide: false,
                body,
                leading: None,
                actions: Vec::new(),
            };
        };
        let shown = shown(clients, &self.snapshot.agents);
        body.push(self.connect_grid(&shown, cx));
        let skip = (!settled).then(|| {
            Button::new("onboarding-skip-connect")
                .label("Skip")
                .ghost()
                .inactive(self.blocked())
                .on_click(cx.listener(|view, _, _, cx| view.settle_connect_step(cx)))
                .into_any_element()
        });
        let primary = primary(&shown, &self.offers.unticked, &self.offers.links);
        let button = match &primary {
            Primary::SetUp(targets) => Button::new("onboarding-set-up")
                .label("Set up")
                .inactive(targets.is_empty()),
            Primary::TryAgain(targets) => Button::new("onboarding-set-up")
                .label("Try again")
                .inactive(targets.is_empty()),
            // Gone back to, the frame's Continue leads on.
            Primary::Continue if settled => {
                return StepView {
                    title: TITLE,
                    wide: false,
                    body,
                    leading: None,
                    actions: Vec::new(),
                };
            }
            Primary::Continue => Button::new("onboarding-connect-continue").label("Continue"),
        };
        let button = if settled { button } else { button.primary() };
        let action =
            button
                .inactive(self.blocked())
                .on_click(cx.listener(move |view, _, _, cx| match &primary {
                    Primary::SetUp(targets) | Primary::TryAgain(targets) => {
                        view.set_up(targets.clone(), cx);
                    }
                    Primary::Continue => view.settle_connect_step(cx),
                }));
        StepView {
            title: TITLE,
            wide: false,
            body,
            leading: None,
            actions: skip
                .into_iter()
                .chain([action.into_any_element()])
                .collect(),
        }
    }

    /// [`COLUMNS`] columns of tiles; the frame scrolls the step when they do not fit.
    fn connect_grid(&self, shown: &[(McpClient, bool)], cx: &mut Context<Self>) -> AnyElement {
        let mut lines = Vec::new();
        for chunk in shown.chunks(COLUMNS) {
            let line = h_flex()
                .gap(px(space::MD))
                .items_stretch()
                .children(chunk.iter().map(|&(client, connected)| {
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .child(self.connect_tile(client, connected, cx))
                }))
                .children((chunk.len()..COLUMNS).map(|_| div().flex_1()));
            lines.push(line);
        }
        v_flex()
            .id("onboarding-connect-agents")
            .gap(px(space::MD))
            .children(lines)
            .into_any_element()
    }

    /// `client`'s tile: ticked unless unticked, or "Connected" in place of the check once it is.
    fn connect_tile(
        &self,
        client: McpClient,
        connected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = format!("onboarding-connect-{}", client.id());
        let link = self.offers.links.get(&client);
        let detail = |text: String, tone: TextTone| {
            Text::labeled(format!("{id}-detail"), text)
                .level(text::BODY_SM)
                .tone(tone)
                .into_any_element()
        };
        let (detail, corner) = match link {
            _ if connected => (None, Some(connected_mark(&id))),
            Some(Link::Connected) => (None, Some(connected_mark(&id))),
            Some(Link::Connecting) => (
                Some(detail("Connecting…".to_owned(), TextTone::Muted)),
                Some(Spinner::new(IconSize::Sm).into_any_element()),
            ),
            Some(Link::Opened) => (
                Some(detail(
                    format!("Finish in {}.", client.name()),
                    TextTone::Muted,
                )),
                None,
            ),
            Some(Link::Failed(error)) => (
                Some(detail(
                    error.clone(),
                    TextTone::Status(Status::Danger.text()),
                )),
                None,
            ),
            None => (None, None),
        };
        let toggles = !connected && matches!(link, None | Some(Link::Failed(_)));
        let mut tile = SelectTile::checkbox(id, client.name())
            .media(AvatarStack::new([agent_logo(client.agent())]))
            .selected(connected || !self.offers.unticked.contains(&client));
        if let Some(detail) = detail {
            tile = tile.detail(detail);
        }
        if let Some(corner) = corner {
            tile = tile.corner(corner);
        }
        if toggles {
            let view = cx.entity().downgrade();
            tile = tile.on_select(move |_, cx| {
                let _ = view.update(cx, |view, cx| view.toggle_connect(client, cx));
            });
        }
        tile.into_any_element()
    }

    fn toggle_connect(&mut self, client: McpClient, cx: &mut Context<Self>) {
        if self.blocked() {
            return;
        }
        let unticked = &mut self.offers.unticked;
        if !unticked.remove(&client) {
            unticked.insert(client);
        }
        cx.notify();
    }

    /// Skip or Continue: saves that the step was asked, and moves on.
    fn settle_connect_step(&mut self, cx: &mut Context<Self>) {
        let client = self.client.clone();
        self.run(
            async move { client.settle_connect_agents().await.map(|_| Effect::None) },
            cx,
        );
    }

    /// Connects `targets` in turn; the first installs the command when it is missing.
    fn set_up(&mut self, targets: Vec<McpClient>, cx: &mut Context<Self>) {
        if self.blocked() || targets.is_empty() {
            return;
        }
        self.set_busy(true, cx);
        for target in &targets {
            self.offers.links.remove(target);
        }
        self.connect_next(targets, cx);
    }

    fn connect_next(&mut self, mut targets: Vec<McpClient>, cx: &mut Context<Self>) {
        if targets.is_empty() {
            self.set_up_ended(cx);
            return;
        }
        let target = targets.remove(0);
        self.offers.links.insert(target, Link::Connecting);
        let client = self.client.clone();
        self.ask(
            async move { client.connect_mcp(target).await },
            move |view, connected, cx| view.connected(target, connected, targets, cx),
            cx,
        );
    }

    /// Shows what connecting `target` did, then connects the `rest`. Cancelling the
    /// administrator prompt stops Set up, leaving the step as it was.
    fn connected(
        &mut self,
        target: McpClient,
        connected: AppResult<McpConnect>,
        rest: Vec<McpClient>,
        cx: &mut Context<Self>,
    ) {
        let link = match connected {
            Ok(McpConnect::Connected) => Link::Connected,
            Ok(McpConnect::Open(link)) => {
                cx.open_url(&link);
                Link::Opened
            }
            Ok(McpConnect::CommandMissing) => {
                self.offers.links.remove(&target);
                self.set_busy(false, cx);
                cx.notify();
                return;
            }
            Err(error) => Link::Failed(error.message),
        };
        self.offers.links.insert(target, link);
        cx.notify();
        self.connect_next(rest, cx);
    }

    /// With every agent connected, the step moves on after a moment; with a failure it stays.
    fn set_up_ended(&mut self, cx: &mut Context<Self>) {
        self.reread_command(cx);
        let failed = self
            .offers
            .links
            .values()
            .any(|link| matches!(link, Link::Failed(_)));
        if failed {
            self.set_busy(false, cx);
            cx.notify();
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(READY_FOR).await;
            let _ = this.update(cx, |view, cx| {
                view.set_busy(false, cx);
                view.settle_connect_step(cx);
            });
        })
        .detach();
    }
}

/// "Connected" in the success color, in place of a tile's check.
fn connected_mark(id: &str) -> AnyElement {
    Text::labeled(format!("{id}-connected"), "Connected")
        .level(text::BODY_SM)
        .tone(TextTone::Status(Status::Success.text()))
        .into_any_element()
}
