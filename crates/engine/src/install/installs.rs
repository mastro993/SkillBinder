//! The installs this session started, kept in memory so they run on while Explore is closed and
//! their states reach every snapshot.

use super::Failed;
use crate::storage;
use skillbinder_proto::{AppResult, InstallFailure, InstallState};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex, MutexGuard},
};
use uuid::Uuid;

#[derive(Default)]
struct Shared {
    /// Each install that is not idle, by its source's
    /// [`install_key`](skillbinder_proto::install_key), or before a skills.sh result is resolved
    /// by its [`result_key`](skillbinder_proto::result_key).
    states: BTreeMap<String, InstallState>,
    /// The ids of the running installs, whose caches are in use.
    running: BTreeSet<Uuid>,
}

/// Install states shared by the worker, the client, and the installs' tasks.
#[derive(Clone, Default)]
pub(crate) struct Installs {
    shared: Arc<Mutex<Shared>>,
}

impl Installs {
    fn lock(&self) -> AppResult<MutexGuard<'_, Shared>> {
        self.shared
            .lock()
            .map_err(storage::error("locking the install state"))
    }

    /// Each install that is not idle.
    pub(crate) fn current(&self) -> BTreeMap<String, InstallState> {
        self.lock()
            .map(|shared| shared.states.clone())
            .unwrap_or_default()
    }

    /// The ids of the running installs.
    pub(crate) fn running(&self) -> AppResult<Vec<Uuid>> {
        Ok(self.lock()?.running.iter().copied().collect())
    }

    /// Marks the skill `key` names installing under a new id, clearing its last failure; `None`
    /// while it is installing already.
    pub(crate) fn begin(&self, key: String) -> AppResult<Option<Begun>> {
        let mut shared = self.lock()?;
        if shared.states.get(&key) == Some(&InstallState::Installing) {
            return Ok(None);
        }
        let id = Uuid::new_v4();
        shared.states.insert(key.clone(), InstallState::Installing);
        shared.running.insert(id);
        Ok(Some(Begun {
            installs: Some(self.clone()),
            key,
            id,
        }))
    }
}

/// A running install. Ending it keeps a failure for Explore to show; dropping it unended, as a
/// panicking or abandoned install does, makes the source idle again.
#[must_use]
pub(crate) struct Begun {
    installs: Option<Installs>,
    key: String,
    pub(crate) id: Uuid,
}

impl Begun {
    /// Ends the install: a failure stays shown until the next install of the source; success,
    /// a slug the library came to hold, and a quit leave it idle.
    pub(crate) fn end(mut self, result: &Result<(), Failed>) {
        let failure = match result {
            Err(Failed::Failure(failure, _)) => Some(failure.clone()),
            Ok(()) | Err(Failed::Held | Failed::Cancelled) => None,
        };
        self.finish(failure);
    }

    fn finish(&mut self, failure: Option<InstallFailure>) {
        let Some(installs) = self.installs.take() else {
            return;
        };
        let Ok(mut shared) = installs.shared.lock() else {
            return;
        };
        shared.running.remove(&self.id);
        match failure {
            Some(failure) => {
                shared
                    .states
                    .insert(self.key.clone(), InstallState::Failed(failure));
            }
            None => {
                shared.states.remove(&self.key);
            }
        }
    }
}

impl Drop for Begun {
    fn drop(&mut self) {
        self.finish(None);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use skillbinder_proto::{SkillSource, install_key};

    fn source() -> SkillSource {
        SkillSource {
            url: "https://github.com/acme/skills.git".into(),
            reference: None,
            path: "skills/pdf".into(),
        }
    }

    #[test]
    fn a_source_installs_once_at_a_time_and_keeps_its_last_failure() {
        let installs = Installs::default();
        let begun = installs.begin(install_key(&source())).unwrap().unwrap();
        assert!(installs.begin(install_key(&source())).unwrap().is_none());
        assert_eq!(installs.running().unwrap(), [begun.id]);
        begun.end(&Err(InstallFailure::Access.into()));
        let key = install_key(&source());
        assert_eq!(
            installs.current()[&key],
            InstallState::Failed(InstallFailure::Access)
        );
        assert!(installs.running().unwrap().is_empty());

        // Installing again clears the failure; an abandoned install leaves the source idle.
        let again = installs.begin(install_key(&source())).unwrap().unwrap();
        assert_eq!(installs.current()[&key], InstallState::Installing);
        drop(again);
        assert!(installs.current().is_empty());
        assert!(installs.running().unwrap().is_empty());
    }
}
