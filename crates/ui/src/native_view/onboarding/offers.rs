//! What the command and agents steps know (ADR 0027): the command's state and the agents'
//! connections, read from the engine when either step shows, and what Install and Set up did.

use super::super::NativeView;
use gpui_kit::Context;
use skillbinder_proto::{CommandLineStatus, McpClient, McpClientStatus};
use std::collections::{BTreeMap, BTreeSet};

/// What Install did in the command step.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) enum Install {
    #[default]
    Idle,
    Running,
    Failed(String),
}

/// What Set up did for one agent's tile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Link {
    Connecting,
    Connected,
    /// Cursor's install link opened; Cursor asks the user.
    Opened,
    Failed(String),
}

#[derive(Default)]
pub(in crate::native_view) struct Offers {
    /// The command's state read last.
    pub(super) command: Option<CommandLineStatus>,
    reading_command: bool,
    pub(super) install: Install,
    /// The agents Connect knows that are on this computer, read once the agents step shows.
    pub(super) clients: Option<Vec<McpClientStatus>>,
    reading_clients: bool,
    /// The tiles of the agents step left unticked; every tile starts ticked.
    pub(super) unticked: BTreeSet<McpClient>,
    pub(super) links: BTreeMap<McpClient, Link>,
}

impl NativeView {
    /// Reads the command's state once, as either step shows.
    pub(super) fn read_command(&mut self, cx: &mut Context<Self>) {
        let offers = &mut self.offers;
        if offers.command.is_some() || offers.reading_command {
            return;
        }
        offers.reading_command = true;
        let client = self.client.clone();
        self.ask(
            async move { client.command_line_status().await },
            |view, status, cx| {
                view.offers.reading_command = false;
                view.offers.command = status.ok();
                cx.notify();
            },
            cx,
        );
    }

    /// Reads the command's state again, after an action that may have installed it.
    pub(super) fn reread_command(&mut self, cx: &mut Context<Self>) {
        self.offers.command = None;
        self.read_command(cx);
    }

    /// Reads the agents' connections once, as the agents step shows.
    pub(super) fn read_clients(&mut self, cx: &mut Context<Self>) {
        let offers = &mut self.offers;
        if offers.clients.is_some() || offers.reading_clients {
            return;
        }
        offers.reading_clients = true;
        let client = self.client.clone();
        self.ask(
            async move { client.mcp_clients().await },
            |view, clients, cx| {
                view.offers.reading_clients = false;
                // Unread, the step offers no agent and leads on with Continue.
                view.offers.clients = Some(clients.unwrap_or_default());
                cx.notify();
            },
            cx,
        );
    }
}
