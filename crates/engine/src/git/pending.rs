//! How many skills wait on each side: the slugs this device changed, and those the remote
//! changed, since the commit both share.

use super::{Git, failure, process, record_of, skill_of};
use skillbinder_proto::AppResult;
use std::{collections::BTreeSet, path::Path, time::Duration};

impl Git {
    /// The number of slugs changed on this device and on the remote since their merge base, or
    /// since nothing when they share none. `status` is the managed paths' porcelain status, whose
    /// changes count on this device's side.
    pub(super) fn pending_skills(
        &self,
        repo: &Path,
        local: Option<&str>,
        incoming: Option<&str>,
        status: &str,
    ) -> AppResult<(u64, u64)> {
        let base = match (local, incoming) {
            (Some(local), Some(incoming)) => self.merge_base(repo, local, incoming)?,
            _ => None,
        };
        let mut here = changed_paths(status);
        if let Some(local) = local {
            here.extend(self.changed_skills(repo, base.as_deref(), local)?);
        }
        let there = match incoming {
            Some(incoming) => self.changed_skills(repo, base.as_deref(), incoming)?,
            None => BTreeSet::new(),
        };
        Ok((count(&here), count(&there)))
    }

    /// The best common ancestor of two commits, if they share one.
    fn merge_base(&self, repo: &Path, one: &str, other: &str) -> AppResult<Option<String>> {
        let args = ["merge-base", one, other];
        let out = process::run(
            &self.program,
            Some(repo),
            &args,
            Duration::from_secs(30),
            None,
        )?;
        match out.status.code() {
            Some(0) => Ok(String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .map(str::to_owned)),
            // Unrelated histories share no commit.
            Some(1) => Ok(None),
            _ => Err(failure("Git history is unavailable.").because(process::exited(&args, &out))),
        }
    }
}

/// The slugs of `git status --porcelain=v1 -z` entries; renames and copies carry a second path.
fn changed_paths(status: &str) -> BTreeSet<String> {
    let mut entries = status.split_terminator('\0');
    let mut paths = Vec::new();
    while let Some(entry) = entries.next() {
        if entry.get(..2).is_some_and(|xy| xy.contains(['R', 'C'])) {
            paths.extend(entries.next());
        }
        paths.extend(entry.get(3..));
    }
    paths
        .into_iter()
        .filter_map(|path| skill_of(path).or_else(|| record_of(path)))
        .map(str::to_owned)
        .collect()
}

fn count(slugs: &BTreeSet<String>) -> u64 {
    u64::try_from(slugs.len()).unwrap_or(u64::MAX)
}
