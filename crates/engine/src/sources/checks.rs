//! Checks of library skills' sources, run on their own thread and kept in memory: the next check
//! after launch replaces them.

use super::{CacheUser, GONE, cache_dir, checkout, refusal};
use crate::{git::Git, logging::Logger, persistence};
use skillbinder_proto::*;
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

/// A library skill whose source is due for a check, with the content it is compared against.
pub(crate) struct Due {
    pub id: SkillId,
    pub source: SkillSource,
    pub digest: String,
}

/// One skill's last check, which holds only while its source and content are unchanged.
struct Checked {
    source: SkillSource,
    digest: String,
    at: u64,
    update: SkillUpdate,
}

#[derive(Default)]
struct State {
    checked: BTreeMap<SkillId, Checked>,
    running: bool,
}

/// Update states shared by the worker and the check thread.
#[derive(Clone, Default)]
pub(crate) struct Updates {
    state: Arc<Mutex<State>>,
}

impl Updates {
    fn lock(&self) -> AppResult<MutexGuard<'_, State>> {
        self.state.lock().map_err(|_| AppError::storage())
    }

    /// The update state of each of `skills` whose last check still describes its source and
    /// content.
    pub(crate) fn current(&self, skills: &[Skill]) -> AppResult<BTreeMap<SkillId, SkillUpdate>> {
        let state = self.lock()?;
        Ok(skills
            .iter()
            .filter_map(|skill| {
                let checked = state.checked.get(&skill.id)?;
                (skill.source.as_ref() == Some(&checked.source) && skill.digest == checked.digest)
                    .then(|| (skill.id.clone(), checked.update.clone()))
            })
            .collect())
    }

    /// Starts a check of the skills with a source not checked for their current content within
    /// `max_age` seconds. `None` while another check runs or when no skill is due.
    pub(crate) fn begin(&self, skills: &[Skill], max_age: u64) -> AppResult<Option<Vec<Due>>> {
        let mut state = self.lock()?;
        if state.running {
            return Ok(None);
        }
        let now = persistence::now();
        let due: Vec<_> = skills
            .iter()
            .filter_map(|skill| {
                let source = skill.source.as_ref().filter(|source| !source.pinned())?;
                let fresh = state.checked.get(&skill.id).is_some_and(|checked| {
                    &checked.source == source
                        && checked.digest == skill.digest
                        && now.saturating_sub(checked.at) < max_age
                });
                (!fresh).then(|| Due {
                    id: skill.id.clone(),
                    source: source.clone(),
                    digest: skill.digest.clone(),
                })
            })
            .collect();
        state.running = !due.is_empty();
        Ok(state.running.then_some(due))
    }

    /// Ends the running check, keeping each result.
    pub(crate) fn finish(&self, results: Vec<(Due, UpdateCheck)>) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.running = false;
        let at = persistence::now();
        for (due, check) in results {
            let checked = Checked {
                source: due.source,
                digest: due.digest,
                at,
                update: SkillUpdate {
                    check,
                    failure: None,
                },
            };
            state.checked.insert(due.id, checked);
        }
    }

    /// Records that `id`'s library content now matches its source.
    pub(crate) fn current_with(&self, id: SkillId, source: SkillSource, digest: String) {
        if let Ok(mut state) = self.state.lock() {
            let update = SkillUpdate {
                check: UpdateCheck::UpToDate,
                failure: None,
            };
            let at = persistence::now();
            state.checked.insert(
                id,
                Checked {
                    source,
                    digest,
                    at,
                    update,
                },
            );
        }
    }

    /// Records why updating `skill` failed, beside its last check. Without a check that still
    /// describes it, the skill is due for one, and the failure shows until then.
    pub(crate) fn failed(&self, skill: &Skill, message: String) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let current = state.checked.get(&skill.id).is_some_and(|checked| {
            skill.source.as_ref() == Some(&checked.source) && skill.digest == checked.digest
        });
        if let (false, Some(source)) = (current, &skill.source) {
            let checked = Checked {
                source: source.clone(),
                digest: skill.digest.clone(),
                at: 0,
                update: SkillUpdate {
                    check: UpdateCheck::Available,
                    failure: None,
                },
            };
            state.checked.insert(skill.id.clone(), checked);
        }
        if let Some(checked) = state.checked.get_mut(&skill.id) {
            checked.update.failure = Some(message);
        }
    }
}

/// Checks each due skill, fetching each source repository and ref once.
pub(crate) fn check(
    git: &Git,
    data_dir: &Path,
    due: Vec<Due>,
    log: &Logger,
) -> Vec<(Due, UpdateCheck)> {
    let mut groups: BTreeMap<(String, Option<String>), Vec<Due>> = BTreeMap::new();
    for skill in due {
        let key = (skill.source.url.clone(), skill.source.reference.clone());
        groups.entry(key).or_default().push(skill);
    }
    let mut results = Vec::new();
    for skills in groups.into_values() {
        let checks = check_group(git, data_dir, &skills, log);
        results.extend(skills.into_iter().zip(checks));
    }
    results
}

/// Checks skills that share a source repository and ref.
fn check_group(git: &Git, data_dir: &Path, skills: &[Due], log: &Logger) -> Vec<UpdateCheck> {
    let failed = |error: AppError| {
        log.record(format!("Source check failed: {error}"));
        UpdateCheck::Failed(format!("Couldn't check for updates: {}", error.message))
    };
    let Some(first) = skills.first() else {
        return Vec::new();
    };
    let cache = cache_dir(data_dir, CacheUser::Check, &first.source);
    if let Err(error) = git.fetch_source(&cache, &first.source) {
        return skills.iter().map(|_| failed(error.clone())).collect();
    }
    skills
        .iter()
        .map(|skill| {
            let staging = data_dir
                .join("staging")
                .join(format!("check-{}", uuid::Uuid::new_v4()));
            let fetched = checkout(git, &cache, &skill.source, &staging, skill.id.as_str());
            let _ = fs::remove_dir_all(&staging);
            match fetched {
                Ok(None) => UpdateCheck::Failed(GONE.to_owned()),
                Ok(Some(inspection)) if inspection.manifest.digest == skill.digest => {
                    UpdateCheck::UpToDate
                }
                // An update would be refused, so the skill has none to offer.
                Ok(Some(inspection)) => refusal(&inspection)
                    .map_or(UpdateCheck::Available, |why| {
                        UpdateCheck::Failed(why.to_owned())
                    }),
                Err(error) => failed(error),
            }
        })
        .collect()
}
