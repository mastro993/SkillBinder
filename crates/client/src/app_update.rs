//! App update checks (spec rules 1, 5, 16, 24): Check now always checks; while automatic checks
//! are on, the app also checks at launch, every hour, after a channel change, and when they are
//! turned back on. Checks, and the downloads they start, run on the engine's own thread, so their
//! state, a download's percent included, publishes from the client's 250-millisecond poll without
//! waiting on the worker. Restart is a request the engine acts on at quit.

use crate::{Client, Shared};
use skillbinder_proto::*;
use std::{sync::Arc, time::Duration};
use tokio::time::MissedTickBehavior;

/// How often a running app checks while automatic checks are on.
const CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);

#[cfg(test)]
mod tests;

/// What asks for a check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Trigger {
    Launch,
    Hourly,
    ChannelChanged,
    AutoCheckTurnedOn,
    CheckNow,
}

impl Trigger {
    /// Whether this trigger checks: Check now and turning automatic checks on always do, the
    /// others only while automatic checks are on (rules 1, 16, 24).
    fn checks(self, auto_check: bool) -> bool {
        match self {
            Self::CheckNow | Self::AutoCheckTurnedOn => true,
            Self::Launch | Self::Hourly | Self::ChannelChanged => auto_check,
        }
    }

    /// Only Check now shows its failure.
    fn manual(self) -> bool {
        self == Self::CheckNow
    }
}

impl Client {
    /// Checks now, then every hour while the client lives, whenever automatic checks are on.
    pub(crate) fn start_app_update_checks(&self) {
        let weak = Arc::downgrade(&self.shared);
        self.shared.runtime.spawn(async move {
            let mut interval = tokio::time::interval(CHECK_INTERVAL);
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
            let mut trigger = Trigger::Launch;
            loop {
                interval.tick().await;
                let Some(shared) = weak.upgrade() else {
                    break;
                };
                Self { shared }.check_on(trigger);
                trigger = Trigger::Hourly;
            }
        });
    }
    /// Checks for a newer SkillBinder now, as Check now does; its failure shows in Settings.
    pub fn check_app_update(&self) {
        self.check_on(Trigger::CheckNow);
    }
    /// Follows `channel` from now on, and checks it while automatic checks are on.
    pub async fn set_update_channel(&self, channel: Channel) -> AppResult<()> {
        self.shared.engine.set_update_channel(channel).await?;
        self.shared.publish_app_update();
        self.check_on(Trigger::ChannelChanged);
        Ok(())
    }
    /// Turns automatic checks on or off; turning them on checks at once.
    pub async fn set_update_auto_check(&self, on: bool) -> AppResult<()> {
        self.shared.engine.set_update_auto_check(on).await?;
        self.shared.publish_app_update();
        if on {
            self.check_on(Trigger::AutoCheckTurnedOn);
        }
        Ok(())
    }
    /// Moves SkillBinder into Applications (spec rule 21). The app then quits, and the desktop
    /// starts the moved copy.
    pub async fn move_to_applications(&self) -> AppResult<()> {
        self.shared.engine.move_to_applications().await
    }
    /// Answers the offer to add SkillBinder to the applications menu once (spec rule 23): `add`
    /// writes the menu entry.
    pub async fn answer_applications_menu(&self, add: bool) -> AppResult<()> {
        self.shared.engine.answer_applications_menu(add).await?;
        self.shared.publish_app_update();
        Ok(())
    }
    /// Checks in the background when `trigger` checks; the progress poll shows it checking, and
    /// its result is published as soon as it ends.
    fn check_on(&self, trigger: Trigger) {
        let auto_check = self
            .shared
            .engine
            .current_app_update()
            .is_ok_and(|update| update.auto_check);
        if !trigger.checks(auto_check) {
            return;
        }
        let manual = trigger.manual();
        let client = self.clone();
        self.shared.runtime.spawn(async move {
            let checked = client.shared.engine.check_app_update(manual).await;
            client.shared.publish_app_update();
            if let Err(error) = checked {
                client.background_error(error);
            }
        });
    }
    /// Restart: once the app quits, the ready app update installs and the app opens again.
    pub fn request_restart(&self) -> AppResult<()> {
        self.shared.engine.request_restart()
    }
}

impl Shared {
    /// Publishes the app update state without waiting on the worker.
    pub(crate) fn publish_app_update(&self) {
        let Ok(update) = self.engine.current_app_update() else {
            return;
        };
        self.snapshots.send_if_modified(|current| {
            if current.app_update == update {
                return false;
            }
            let mut next = (**current).clone();
            next.app_update = update;
            next.generation = current.generation + 1;
            *current = Arc::new(next);
            true
        });
    }
}
