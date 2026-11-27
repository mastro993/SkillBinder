//! The commit Push and Sync make of the managed files (ADR 0038), named by the device and saying
//! why each skill changed.

use super::merge::Merge;
use super::note::{self, Note};
use super::{Git, MANAGED, record_of, skill_of};
use skillbinder_proto::{AppResult, ChangeKind, Reason};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// What a Push or Sync's commit asks of the library's owner.
pub(crate) trait Committer {
    /// Runs right before a commit when nothing was pulled, so files it writes join that commit
    /// and never dirty the library ahead of a Pull.
    fn prepare(&self) -> AppResult<()>;
    /// This device's name, which the commit carries.
    fn device(&self) -> AppResult<String>;
    /// Why each skill changed, by slug; a changed skill without one changed outside SkillBinder.
    fn reasons(&self) -> AppResult<BTreeMap<String, Reason>>;
    /// Runs once HEAD holds the managed files as they are, whether or not that took a commit,
    /// so no change is left for a recorded reason to explain.
    fn committed(&self) -> AppResult<()>;
    /// Records `merge` before the library's files move to it, so a launch after an interruption
    /// finishes it.
    fn merging(&self, merge: &Merge) -> AppResult<()>;
    /// Forgets the merge once HEAD is the merged commit.
    fn merged(&self) -> AppResult<()>;
}

impl Git {
    /// Commits the managed files, unless nothing changed since a first commit, with a message
    /// naming the device and why each skill changed (ADR 0038).
    pub(super) fn commit(&self, repo: &Path, committer: &dyn Committer) -> AppResult<()> {
        // A pathspec absent both from HEAD and disk is not passed to `add`.
        for path in MANAGED {
            if repo.join(path).exists() || !self.output(repo, &["ls-files", "--", path])?.is_empty()
            {
                self.output(repo, &["add", "-f", "-A", "--", path])?;
            }
        }
        // Publish only what a fetch would accept, so a pushed commit never blocks later fetches.
        let staged = self.output(repo, &["write-tree"])?;
        let staged = staged.trim();
        self.validate_tree(repo, staged)?;
        let head = self.revision(repo, "HEAD")?;
        let changed = self.output(
            repo,
            &["diff", "--cached", "--no-renames", "--name-only", "-z"],
        )?;
        // A library without skills still publishes a first, empty commit.
        if head.is_none() || !changed.is_empty() {
            self.commit_note(repo, head.as_deref(), staged, &changed, committer)?;
        }
        committer.committed()
    }
    fn commit_note(
        &self,
        repo: &Path,
        head: Option<&str>,
        staged: &str,
        changed: &str,
        committer: &dyn Committer,
    ) -> AppResult<()> {
        let kinds = self.kinds(repo, head, staged, changed)?;
        let note = Note::new(committer.device()?, kinds, &committer.reasons()?);
        let mut args = note.identity();
        args.extend(["commit", "--allow-empty", "--cleanup=whitespace"].map(str::to_owned));
        for paragraph in [note.subject(), note.body(), note.trailers()] {
            if !paragraph.is_empty() {
                args.extend(["-m".to_owned(), paragraph]);
            }
        }
        self.output(repo, &args.iter().map(String::as_str).collect::<Vec<_>>())
            .map(drop)
    }
    /// How the `-z` paths `changed` change each slug from `head` to the tree `staged`.
    pub(super) fn kinds(
        &self,
        repo: &Path,
        head: Option<&str>,
        staged: &str,
        changed: &str,
    ) -> AppResult<BTreeMap<String, ChangeKind>> {
        let before = match head {
            Some(head) => self.skill_dirs(repo, head)?,
            None => BTreeSet::new(),
        };
        let after = self.skill_dirs(repo, staged)?;
        Ok(changed
            .split('\0')
            .filter_map(|path| skill_of(path).or_else(|| record_of(path)))
            .map(|slug| {
                let kind = note::kind(before.contains(slug), after.contains(slug));
                (slug.to_owned(), kind)
            })
            .collect())
    }
    /// The skill directories in `tree`.
    pub(super) fn skill_dirs(&self, repo: &Path, tree: &str) -> AppResult<BTreeSet<String>> {
        Ok(self
            .output(
                repo,
                &["ls-tree", "--name-only", "-z", tree, "--", "skills/"],
            )?
            .split('\0')
            .filter_map(|path| path.strip_prefix("skills/"))
            .filter(|slug| !slug.is_empty())
            .map(str::to_owned)
            .collect())
    }
}
