//! A change that writes and removes copies in many directories publishes how far it has got as
//! [`AppSnapshot::activation`] while it runs.

use crate::Client;
use skillbinder_proto::*;
use std::{future::Future, sync::Arc, time::Duration};
use tokio::time::{Instant, MissedTickBehavior};

/// How often a tracked change publishes its progress.
const PUBLISH_INTERVAL: Duration = Duration::from_millis(100);

impl Client {
    /// Runs the engine `call` on the runtime and publishes the engine's progress record every
    /// interval until it returns, then reads the final counts and publishes `None` whatever the
    /// result. A change faster than one interval returns at once.
    pub(crate) async fn tracked<T: Send + 'static>(
        &self,
        call: impl Future<Output = AppResult<T>> + Send + 'static,
    ) -> (AppResult<T>, ActivationProgress) {
        let mut task = self.shared.runtime.spawn(call);
        let mut interval = tokio::time::interval_at(Instant::now(), PUBLISH_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let result = loop {
            tokio::select! {
                biased;
                joined = &mut task => break joined.unwrap_or_else(|_| Err(AppError::storage())),
                _ = interval.tick() => self.publish_activation(Some(self.activation_counts())),
            }
        };
        let counts = self.activation_counts();
        self.publish_activation(None);
        (result, counts)
    }

    /// The engine's counts, or none counted yet.
    fn activation_counts(&self) -> ActivationProgress {
        self.shared
            .engine
            .activation_progress()
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    /// Publishes how far the change in flight has got, or `None` once it is over.
    fn publish_activation(&self, activation: Option<ActivationProgress>) {
        self.shared.snapshots.send_if_modified(|current| {
            if current.activation == activation {
                return false;
            }
            let mut next = (**current).clone();
            next.activation = activation;
            next.generation = current.generation + 1;
            *current = Arc::new(next);
            true
        });
    }
}
