//! The engine's side of app updates: checks and downloads on their own thread, the channel, the
//! automatic-check switch, and the staged update saved through the worker, and the install at
//! quit.

use super::{Engine, action::Action};
use crate::{
    QuitCause, Relaunch,
    app_update::Ended,
    logging::{Area, Level},
    storage,
};
use skillbinder_proto::*;

impl Engine {
    /// Checks the channel's manifest and waits for the check to end, unless the build never
    /// checks or a check is running; where the app can replace itself, that includes downloading
    /// and staging a newer version. Only a `manual` check keeps its failure. A check the channel
    /// changed under is followed by one of the new channel.
    pub async fn check_app_update(&self, manual: bool) -> AppResult<()> {
        self.action(Action::CheckAppUpdate, Vec::new(), |engine| async move {
            engine.check_app_update_now(manual).await
        })
        .await
    }
    /// Checks as [`Self::check_app_update`] does, writing one summary for a check that ran.
    async fn check_app_update_now(&self, manual: bool) -> AppResult<()> {
        let started = std::time::Instant::now();
        loop {
            let Some(finished) = self.owner.app_updates.check(manual)? else {
                return Ok(());
            };
            match finished
                .await
                .map_err(storage::error("waiting for the app update check"))?
            {
                Ended::Checked { check, ready } => {
                    let offered = check
                        .offered()
                        .map_or_else(|| "none".to_owned(), ToString::to_string);
                    self.log
                        .event(Level::Info, Area::AppUpdate, "check finished")
                        .field("offered", offered)
                        .took(started);
                    return self
                        .call(move |state| state.save_update_check(&check, ready.as_ref()))
                        .await;
                }
                Ended::Superseded => {}
                Ended::Failed => {
                    self.log
                        .event(Level::Info, Area::AppUpdate, "check finished")
                        .field("offered", "unknown")
                        .took(started);
                    return Ok(());
                }
            }
        }
    }
    /// The running version, the channel, and what the last check found, without the worker.
    pub fn current_app_update(&self) -> AppResult<AppUpdateSnapshot> {
        self.owner.app_updates.current()
    }
    /// Saves the channel app update checks read.
    pub async fn set_update_channel(&self, channel: Channel) -> AppResult<()> {
        let subject = vec![("channel", channel.name().to_owned())];
        self.action(Action::SetUpdateChannel, subject, |engine| async move {
            engine
                .call(move |state| state.save_update_channel(channel))
                .await?;
            engine.owner.app_updates.set_channel(channel)
        })
        .await
    }
    /// Saves whether the app checks by itself.
    pub async fn set_update_auto_check(&self, on: bool) -> AppResult<()> {
        let subject = vec![("on", on.to_string())];
        self.action(Action::SetUpdateAutoCheck, subject, |engine| async move {
            engine
                .call(move |state| state.save_update_auto_check(on))
                .await?;
            engine.owner.app_updates.set_auto_check(on)
        })
        .await
    }
    /// Moves the app into Applications on a thread of its own (spec rule 21). The app then
    /// quits, and the desktop starts the moved copy through [`Self::finish_app_update`].
    pub async fn move_to_applications(&self) -> AppResult<()> {
        let updates = self.owner.app_updates.clone();
        self.action(Action::MoveToApplications, Vec::new(), |_| {
            super::command_line::on_own_thread("skillbinder-move", move || {
                updates.move_to_applications()
            })
        })
        .await
    }
    /// Answers the offer to add SkillBinder to the applications menu (spec rule 23): `add` writes
    /// the menu entry. Either answer is remembered.
    pub async fn answer_applications_menu(&self, add: bool) -> AppResult<()> {
        let subject = vec![("add", add.to_string())];
        self.action(
            Action::AnswerApplicationsMenu,
            subject,
            |engine| async move {
                engine
                    .call(move |state| state.answer_applications_menu(add))
                    .await
            },
        )
        .await
    }
    /// Restart: opens the app again once it quits and installs the ready app update.
    pub fn request_restart(&self) -> AppResult<()> {
        self.owner.app_updates.request_restart()
    }
    /// Call once the app has quit for `cause` and the engine drained: installs a ready app
    /// update (spec rule 12), and says what to start, the moved copy after Move to Applications
    /// or the app again after Restart, once the instance lock is released. On Windows it starts
    /// the setup program instead, unless the session is ending.
    pub fn finish_app_update(&self, cause: QuitCause) -> Option<Relaunch> {
        self.owner.app_updates.finish(cause)
    }
}
