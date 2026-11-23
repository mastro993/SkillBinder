//! skills.sh search (design T3, T4): one GET at a time on a thread of its own, never the worker
//! or the Git pool, and an anticorruption layer for its undocumented payload, which never leaves
//! this module. An answer to an older query is dropped.

use super::fetch::{Answer, CatalogSource, Fetcher};
use crate::{git::validate_url, library::normalized};
use serde::Deserialize;
use skillbinder_proto::{
    AppError, AppResult, SkillsShResult, SkillsShSearch, SkillsShState, result_key,
};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::sync::oneshot;

/// skills.sh's origin; the search is `GET /api/search?q=<query>&limit=<n>`.
const SKILLS_SH: &str = "https://skills.sh/";
/// The most results asked for.
const RESULTS: usize = 20;
/// The largest answer read.
const SEARCH_LIMIT: u64 = 256 * 1024;
/// skills.sh refuses a shorter query.
const MIN_QUERY: usize = 2;

#[derive(Default)]
struct Shared {
    search: SkillsShSearch,
    /// Counts the queries asked; an answer to any but the last is dropped.
    asked: u64,
    running: bool,
    /// Callers waiting for the search to settle.
    waiting: Vec<oneshot::Sender<()>>,
}

impl Shared {
    /// Ends the running search with `state`, waking every caller.
    fn settle(&mut self, state: Option<SkillsShState>) {
        if let Some(state) = state {
            self.search.state = state;
        }
        self.running = false;
        for waiting in self.waiting.drain(..) {
            let _ = waiting.send(());
        }
    }
}

/// The skills.sh search shared by the worker's snapshots, the client, and the search thread.
#[derive(Clone)]
pub(crate) struct Searches {
    shared: Arc<Mutex<Shared>>,
    source: CatalogSource,
}

impl Searches {
    pub(crate) fn new(source: CatalogSource) -> Self {
        Self {
            shared: Arc::default(),
            source,
        }
    }

    fn lock(&self) -> MutexGuard<'_, Shared> {
        // Nothing panics while holding it, so a poisoned lock still holds consistent data.
        self.shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The search for the last query asked.
    pub(crate) fn current(&self) -> SkillsShSearch {
        self.lock().search.clone()
    }

    /// Asks skills.sh for `query`, trimmed and lowercased, in place of any earlier query; a
    /// query under two characters searches nothing. The receiver answers once the search for it,
    /// or for a newer query, ends; `None` when no request is needed.
    pub(crate) fn search(&self, query: &str) -> AppResult<Option<oneshot::Receiver<()>>> {
        let query = query.trim().to_lowercase();
        let mut shared = self.lock();
        let asked = &shared.search;
        if asked.query == query
            && matches!(
                asked.state,
                SkillsShState::Searching | SkillsShState::Results(_)
            )
        {
            return Ok(None);
        }
        shared.asked += 1;
        let state = if query.chars().count() < MIN_QUERY {
            SkillsShState::Idle
        } else {
            SkillsShState::Searching
        };
        let idle = state == SkillsShState::Idle;
        shared.search = SkillsShSearch { query, state };
        if idle {
            return Ok(None);
        }
        let (done, settled) = oneshot::channel();
        shared.waiting.push(done);
        let start = !std::mem::replace(&mut shared.running, true);
        drop(shared);
        if start {
            let running = Running(Some(self.clone()));
            // A thread that cannot start drops `running`, which ends the search as unavailable.
            std::thread::Builder::new()
                .name("skillbinder-search".into())
                .spawn(move || running.run())
                .map_err(|_| AppError::storage())?;
        }
        Ok(Some(settled))
    }

    /// Asks skills.sh for `query`, trimmed and lowercased, on a thread of its own, leaving the
    /// search Explore shows as it is, as the `skillbinder` command's search does. The receiver
    /// answers with what skills.sh answered; `None` for a query under two characters, which asks
    /// nothing.
    pub(crate) fn ask(&self, query: &str) -> AppResult<Option<oneshot::Receiver<SkillsShState>>> {
        let query = query.trim().to_lowercase();
        if query.chars().count() < MIN_QUERY {
            return Ok(None);
        }
        let source = self.source.clone();
        let (done, answered) = oneshot::channel();
        std::thread::Builder::new()
            .name("skillbinder-search".into())
            .spawn(move || {
                let _ = done.send(answer(&source, &query));
            })
            .map_err(|_| AppError::storage())?;
        Ok(Some(answered))
    }

    /// Asks for the last query until no newer one arrived while its request ran.
    fn ask_latest(&self) {
        loop {
            let (asked, query) = {
                let mut shared = self.lock();
                if shared.search.state != SkillsShState::Searching {
                    // The search was cleared while a request ran.
                    shared.settle(None);
                    return;
                }
                (shared.asked, shared.search.query.clone())
            };
            let state = answer(&self.source, &query);
            let mut shared = self.lock();
            if shared.asked == asked {
                shared.settle(Some(state));
                return;
            }
        }
    }
}

/// The running search thread. Dropping it unfinished, as a panicking thread does, ends the search
/// as unavailable so a later one can start.
struct Running(Option<Searches>);

impl Running {
    fn run(mut self) {
        if let Some(searches) = self.0.take() {
            searches.ask_latest();
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(searches) = self.0.take() {
            searches.lock().settle(Some(SkillsShState::Unavailable));
        }
    }
}

/// What skills.sh answered for `query`.
fn answer(source: &CatalogSource, query: &str) -> SkillsShState {
    let fetcher = match source {
        CatalogSource::Http(_) => match url::Url::parse(SKILLS_SH) {
            Ok(base) => Fetcher::http(base),
            Err(_) => return SkillsShState::Unavailable,
        },
        CatalogSource::Directory(root) => Fetcher::Directory(root.join("search")),
    };
    state(&fetcher, query)
}

/// What `fetcher` answered for `query`, as Explore shows it.
fn state(fetcher: &Fetcher, query: &str) -> SkillsShState {
    let path = match fetcher {
        Fetcher::Http { .. } => format!(
            "api/search?{}",
            url::form_urlencoded::Serializer::new(String::new())
                .append_pair("q", query)
                .append_pair("limit", &RESULTS.to_string())
                .finish()
        ),
        Fetcher::Directory(_) => format!(
            "{}.json",
            url::form_urlencoded::byte_serialize(query.as_bytes()).collect::<String>()
        ),
    };
    match fetcher.get(&path, None, SEARCH_LIMIT) {
        Answer::Body { bytes, .. } => {
            translate(&bytes).map_or(SkillsShState::Unavailable, SkillsShState::Results)
        }
        Answer::Offline => SkillsShState::NeedsConnection,
        // Rate limiting (429) and every other answer but the results.
        Answer::NotModified | Answer::NotFound | Answer::Failed => SkillsShState::Unavailable,
    }
}

/// skills.sh's answer: `{skills: [{id, source: "owner/repo", skillId, name, installs}], …}`.
/// Other fields are ignored.
#[derive(Deserialize)]
struct Payload {
    skills: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    source: String,
    skill_id: String,
    name: String,
    installs: u64,
}

/// The results in skills.sh's order, each repository and `skillId` once; `None` when the answer
/// is not that shape. An entry that breaks it, or whose source is not `owner/repo`, is dropped.
fn translate(bytes: &[u8]) -> Option<Vec<SkillsShResult>> {
    let payload: Payload = serde_json::from_slice(bytes).ok()?;
    let mut seen = BTreeSet::new();
    Some(
        payload
            .skills
            .into_iter()
            .filter_map(|entry| serde_json::from_value(entry).ok())
            .filter_map(result)
            .filter(|result| seen.insert(result_key(result)))
            .take(RESULTS)
            .collect(),
    )
}

/// The result of `entry`, its repository on github.com.
fn result(entry: Entry) -> Option<SkillsShResult> {
    let (owner, repository) = entry.source.split_once('/')?;
    let part = |part: &str| {
        !matches!(part, "" | "." | "..")
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    let skill_id = entry.skill_id.trim();
    if !part(owner) || !part(repository) || skill_id.is_empty() {
        return None;
    }
    let url = format!("https://github.com/{owner}/{repository}.git");
    validate_url(&url).ok()?;
    let name = entry.name.trim();
    Some(SkillsShResult {
        key: normalized(skill_id),
        name: if name.is_empty() { skill_id } else { name }.to_owned(),
        skill_id: skill_id.to_owned(),
        url,
        installs: entry.installs,
    })
}

#[cfg(test)]
mod tests;
