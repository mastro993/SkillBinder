//! The sync state of the library against its remote: what waits on each side, fetched first when
//! asked, with the skills changed on both sides that Sync needs a choice for.

use super::{Git, MANAGED, REMOTE_REF, failure};
use skillbinder_proto::{AppResult, Reason, RemoteConfig, SyncSnapshot, SyncState};
use std::{
    collections::BTreeMap,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

impl Git {
    /// Classifies the state, optionally fetching, and lists what waits on each side. A skill
    /// changed on this device says why from `reasons`, by slug, else from the commits not pushed
    /// yet. Never commits or checks out files.
    pub fn status(
        &self,
        repo: &Path,
        remote: Option<&RemoteConfig>,
        fetch: bool,
        reasons: &BTreeMap<String, Reason>,
    ) -> AppResult<SyncSnapshot> {
        let porcelain = self.output(
            repo,
            &[
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
                "--ignored=matching",
                "--",
                MANAGED[0],
                MANAGED[1],
            ],
        )?;
        let changed_files = changed_files(&porcelain);
        let dirty = changed_files > 0;
        let Some(remote) = remote else {
            return Ok(SyncSnapshot {
                uncommitted: dirty,
                changed_files,
                ..Default::default()
            });
        };
        self.validate_remote(remote)?;
        if fetch {
            self.reconcile_origin(repo, remote)?;
            self.fetch(repo, remote)?;
        }
        let local = self.revision(repo, "HEAD")?;
        let incoming = self.revision(repo, REMOTE_REF)?;
        let (ahead, behind) = match (&local, &incoming) {
            (Some(local), Some(incoming)) => {
                let counts = self.output(
                    repo,
                    &[
                        "rev-list",
                        "--left-right",
                        "--count",
                        &format!("{local}...{incoming}"),
                    ],
                )?;
                let mut counts = counts.split_whitespace();
                (count(counts.next())?, count(counts.next())?)
            }
            (Some(_), None) => (1, 0),
            (None, Some(_)) => (0, 1),
            (None, None) => (0, 0),
        };
        let state = if behind > 0 && (ahead > 0 || dirty) {
            SyncState::NeedsSync
        } else if behind > 0 {
            SyncState::NeedsPull
        } else if ahead > 0 || dirty || incoming.is_none() {
            SyncState::NeedsPush
        } else {
            SyncState::Synced
        };
        let (local_skills, incoming_skills) = if state == SyncState::Synced {
            (Vec::new(), None)
        } else {
            self.pending(
                repo,
                local.as_deref(),
                incoming.as_deref(),
                &porcelain,
                reasons,
            )?
        };
        let conflicts = match (&incoming, &incoming_skills) {
            (Some(commit), Some(incoming)) if state == SyncState::NeedsSync => {
                let base = match &local {
                    Some(local) => self.merge_base(repo, local, commit)?,
                    None => None,
                };
                let here = match &local {
                    Some(local) if !dirty => local.clone(),
                    _ => self.disk_tree(repo)?,
                };
                let sides = (base.as_deref(), here.as_str(), commit.as_str());
                self.conflicts(repo, sides, &local_skills, &incoming.skills)?
            }
            _ => Vec::new(),
        };
        let state = if conflicts.is_empty() {
            state
        } else {
            SyncState::NeedsResolve
        };
        Ok(SyncSnapshot {
            remote: Some(remote.clone()),
            state,
            ahead,
            behind,
            uncommitted: dirty,
            changed_files,
            local: local_skills,
            incoming: incoming_skills,
            conflicts,
            refreshed_at: fetch.then(|| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            }),
            last_error: None,
            // The engine names the device.
            device: String::new(),
            head: local,
        })
    }
    fn reconcile_origin(&self, repo: &Path, remote: &RemoteConfig) -> AppResult<()> {
        self.output(
            repo,
            &[
                "config",
                "--local",
                "--replace-all",
                "remote.origin.url",
                &remote.url,
            ],
        )?;
        Ok(())
    }
    fn fetch(&self, repo: &Path, remote: &RemoteConfig) -> AppResult<()> {
        let branch = format!("refs/heads/{}", remote.branch);
        let refs =
            self.network_output(repo, &["ls-remote", "--heads", "--", &remote.url, &branch])?;
        if refs.trim().is_empty() {
            self.output(repo, &["update-ref", "-d", REMOTE_REF])?;
            return Ok(());
        }
        self.network_output(
            repo,
            &[
                "fetch",
                "--no-tags",
                "--no-recurse-submodules",
                "--no-write-fetch-head",
                "--",
                &remote.url,
                &branch,
            ],
        )?;
        let expected = refs
            .split_whitespace()
            .next()
            .ok_or_else(|| failure("Remote revision is unavailable."))?;
        // Use the advertised immutable object, never FETCH_HEAD from an unrelated operation.
        self.validate_tree(repo, expected)?;
        self.output(repo, &["update-ref", REMOTE_REF, expected])?;
        Ok(())
    }
}

/// Counts `git status --porcelain=v1 -z` entries; renames and copies carry a second path.
pub(super) fn changed_files(status: &str) -> u64 {
    let mut paths = status.split_terminator('\0');
    let mut count = 0;
    while let Some(entry) = paths.next() {
        if entry.get(..2).is_some_and(|xy| xy.contains(['R', 'C'])) {
            paths.next();
        }
        count += 1;
    }
    count
}
fn count(value: Option<&str>) -> AppResult<u64> {
    value
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| failure("Git returned invalid revision counts."))
}
