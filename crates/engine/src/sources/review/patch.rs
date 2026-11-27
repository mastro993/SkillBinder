//! The patch an update review shows: a Git-style unified diff from the library's files to the
//! fetched ones, built in-process with `similar` so the user's Git configuration never shapes it
//! (ADR 0037). Each text file carries its whole text as context, so the window can unfold every
//! unchanged stretch it folds.

use crate::payload::{Manifest, ManifestKind};
use similar::{ChangeTag, TextDiff};
use skillbinder_proto::{FileChange, ReviewedFile};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

/// Git reads a file as binary when a NUL byte is in its first this many bytes.
const BINARY_PROBE: usize = 8000;

/// One side's files by relative path: their bytes, and whether each is executable.
pub(super) type Files<'a> = BTreeMap<&'a str, (&'a [u8], bool)>;

/// The files `manifest` lists, with their bytes from `bytes`.
pub(super) fn files<'a>(manifest: &'a Manifest, bytes: &'a BTreeMap<String, Vec<u8>>) -> Files<'a> {
    manifest
        .entries
        .iter()
        .filter(|entry| entry.kind == ManifestKind::File)
        .filter_map(|entry| {
            let content = bytes.get(&entry.path)?;
            Some((entry.path.as_str(), (content.as_slice(), entry.executable)))
        })
        .collect()
}

/// Each file that differs between `before` and `after`, sorted by path, and the patch showing
/// how.
pub(super) fn build(before: &Files, after: &Files) -> (Vec<ReviewedFile>, String) {
    let paths: BTreeSet<&str> = before.keys().chain(after.keys()).copied().collect();
    let mut patch = String::new();
    let files = paths
        .into_iter()
        .filter_map(|path| {
            let (old, new) = (before.get(path).copied(), after.get(path).copied());
            (old != new).then(|| file(path, old, new, &mut patch))
        })
        .collect();
    (files, patch)
}

/// Writes one file's entry into `patch`, as `git diff` does.
fn file(
    path: &str,
    old: Option<(&[u8], bool)>,
    new: Option<(&[u8], bool)>,
    patch: &mut String,
) -> ReviewedFile {
    let _ = writeln!(patch, "diff --git a/{path} b/{path}");
    let change = match (old, new) {
        (None, Some((_, executable))) => {
            let _ = writeln!(patch, "new file mode {}", mode(executable));
            FileChange::Added
        }
        (Some((_, executable)), None) => {
            let _ = writeln!(patch, "deleted file mode {}", mode(executable));
            FileChange::Removed
        }
        (Some((_, was)), Some((_, is))) => {
            if was != is {
                let _ = writeln!(patch, "old mode {}\nnew mode {}", mode(was), mode(is));
            }
            FileChange::Changed
        }
        (None, None) => unreachable!("a listed path is on one side at least"),
    };
    let (before, after) = (old.map(|(bytes, _)| bytes), new.map(|(bytes, _)| bytes));
    let old_name = old.map_or_else(|| "/dev/null".to_owned(), |_| format!("a/{path}"));
    let new_name = new.map_or_else(|| "/dev/null".to_owned(), |_| format!("b/{path}"));
    let (additions, deletions) = if before == after {
        // Only the executable bit changed.
        (0, 0)
    } else {
        match (text(before), text(after)) {
            (Some(before), Some(after)) => lines(before, after, &old_name, &new_name, patch),
            _ => {
                let _ = writeln!(patch, "Binary files {old_name} and {new_name} differ");
                (0, 0)
            }
        }
    };
    ReviewedFile {
        path: path.to_owned(),
        change,
        additions,
        deletions,
    }
}

/// Writes the change from `before` to `after` into `patch` as one hunk holding the whole text as
/// context, and returns the lines added and removed. Lines end only at `\n`, as Kit reads them,
/// so a stray `\r` stays inside its line.
fn lines(before: &str, after: &str, old: &str, new: &str, patch: &mut String) -> (u64, u64) {
    let (before, after): (Vec<&str>, Vec<&str>) = (
        before.split_inclusive('\n').collect(),
        after.split_inclusive('\n').collect(),
    );
    let diff = TextDiff::configure().diff_slices(&before, &after);
    let mut body = String::new();
    let (mut additions, mut deletions) = (0, 0);
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Insert => {
                additions += 1;
                '+'
            }
            ChangeTag::Delete => {
                deletions += 1;
                '-'
            }
            ChangeTag::Equal => ' ',
        };
        let line = change.value();
        body.push(sign);
        body.push_str(line);
        if !line.ends_with('\n') {
            body.push_str("\n\\ No newline at end of file\n");
        }
    }
    if additions + deletions > 0 {
        let _ = write!(
            patch,
            "--- {old}\n+++ {new}\n@@ -{} +{} @@\n{body}",
            range(before.len()),
            range(after.len())
        );
    }
    (additions, deletions)
}

/// A hunk side's range covering all `count` lines of a file: `0,0` for none.
fn range(count: usize) -> String {
    format!("{},{count}", usize::from(count > 0))
}

/// A file's text, if Git would read it as text; an absent file's is empty.
fn text(bytes: Option<&[u8]>) -> Option<&str> {
    let bytes = bytes.unwrap_or_default();
    if bytes[..bytes.len().min(BINARY_PROBE)].contains(&0) {
        return None;
    }
    std::str::from_utf8(bytes).ok()
}

fn mode(executable: bool) -> &'static str {
    if executable { "100755" } else { "100644" }
}

#[cfg(test)]
mod tests;
