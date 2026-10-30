//! What one global skill directory needs for one slug, decided from what the active preset holds
//! there, what SkillBinder recorded, and what the directory holds now.

use serde::{Deserialize, Serialize};

/// What SkillBinder recorded for a slug in a directory.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Row {
    /// The manifest digest of the managed copy when it was written or adopted; none while the
    /// directory holds no copy SkillBinder manages.
    pub digest: Option<String>,
    /// What holds the directory back from what the active preset holds there.
    pub issue: Option<Issue>,
}

/// Why a directory does not hold what the active preset holds there.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(super) enum Issue {
    /// Writing the copy failed, for the safe reason given.
    Failed(String),
    /// A copy of the slug with other content waits for the user to replace or keep it.
    Conflict,
    /// The user kept their own copy of the slug.
    Skipped,
    /// The managed copy was edited in place and waits for the user to replace or keep it.
    Edited,
    /// Removing the managed copy failed, for the safe reason given.
    NotRemoved(String),
}

/// What the directory holds under the slug now.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Disk {
    /// Nothing.
    Absent,
    /// A skill directory with this manifest digest.
    Copy(String),
    /// Something SkillBinder never adopts or removes unasked: a link, a file, a Git checkout, or a
    /// directory it cannot read or that cannot safely be copied.
    Other,
}

/// The step that brings a directory in line.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum Step {
    /// Write a copy where there is none.
    Write,
    /// Manage the copy already there, which matches the library.
    Adopt,
    /// Replace an unedited managed copy with the library's current content.
    Rewrite,
    /// Remove an unedited managed copy of a skill no longer active there.
    Remove,
    /// Forget the directory: nothing there is managed or wanted.
    Forget,
    /// Leave the directory as it is and record this.
    Hold(Row),
}

/// The step for one directory and slug. `wanted` tells whether the active preset holds the skill
/// there, and `library` is the library skill's current digest, if the library holds it.
pub(super) fn decide(wanted: bool, row: Option<&Row>, disk: &Disk, library: Option<&str>) -> Step {
    let managed = row.and_then(|row| row.digest.as_deref());
    let held = match disk {
        Disk::Copy(digest) => Some(digest.as_str()),
        Disk::Absent | Disk::Other => None,
    };
    if wanted {
        return match disk {
            Disk::Absent => Step::Write,
            _ if held.is_some() && held == library => Step::Adopt,
            _ if held.is_some() && held == managed => Step::Rewrite,
            _ if managed.is_some() => hold(managed, Issue::Edited),
            _ if row.and_then(|row| row.issue.as_ref()) == Some(&Issue::Skipped) => {
                hold(None, Issue::Skipped)
            }
            _ => hold(None, Issue::Conflict),
        };
    }
    match (managed, disk) {
        (None, _) | (Some(_), Disk::Absent) => Step::Forget,
        (Some(_), Disk::Copy(held)) if removable(row, library).contains(&held.as_str()) => {
            Step::Remove
        }
        _ => hold(managed, Issue::Edited),
    }
}

/// The contents a managed copy of a skill no longer active there may hold to be removed without
/// asking: what SkillBinder wrote, or the library's current content unless the user was already
/// asked about an edit there.
pub(super) fn removable<'a>(row: Option<&'a Row>, library: Option<&'a str>) -> Vec<&'a str> {
    let Some(row) = row else {
        return Vec::new();
    };
    let mut digests: Vec<&str> = row.digest.as_deref().into_iter().collect();
    if row.issue != Some(Issue::Edited) {
        digests.extend(library);
    }
    digests
}

fn hold(digest: Option<&str>, issue: Issue) -> Step {
    Step::Hold(Row {
        digest: digest.map(str::to_owned),
        issue: Some(issue),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn managed(digest: &str) -> Row {
        Row {
            digest: Some(digest.to_owned()),
            issue: None,
        }
    }

    fn copy(digest: &str) -> Disk {
        Disk::Copy(digest.to_owned())
    }

    #[test]
    fn an_active_skill_is_written_adopted_or_rewritten() {
        assert_eq!(decide(true, None, &Disk::Absent, Some("new")), Step::Write);
        assert_eq!(decide(true, None, &copy("new"), Some("new")), Step::Adopt);
        let old = managed("old");
        assert_eq!(
            decide(true, Some(&old), &Disk::Absent, Some("new")),
            Step::Write
        );
        assert_eq!(
            decide(true, Some(&old), &copy("old"), Some("new")),
            Step::Rewrite
        );
        assert_eq!(
            decide(true, Some(&old), &copy("new"), Some("new")),
            Step::Adopt
        );
    }

    #[test]
    fn other_content_is_never_replaced_without_asking() {
        assert_eq!(
            decide(true, None, &copy("theirs"), Some("new")),
            hold(None, Issue::Conflict)
        );
        assert_eq!(
            decide(true, None, &Disk::Other, Some("new")),
            hold(None, Issue::Conflict)
        );
        let skipped = Row {
            digest: None,
            issue: Some(Issue::Skipped),
        };
        assert_eq!(
            decide(true, Some(&skipped), &copy("theirs"), Some("new")),
            hold(None, Issue::Skipped)
        );
        assert_eq!(
            decide(true, Some(&skipped), &Disk::Absent, Some("new")),
            Step::Write
        );
        assert_eq!(
            decide(true, Some(&managed("old")), &copy("edited"), Some("new")),
            hold(Some("old"), Issue::Edited)
        );
    }

    #[test]
    fn only_unedited_managed_copies_are_removed() {
        let old = managed("old");
        assert_eq!(decide(false, Some(&old), &copy("old"), None), Step::Remove);
        assert_eq!(
            decide(false, Some(&old), &copy("new"), Some("new")),
            Step::Remove
        );
        assert_eq!(
            decide(false, Some(&old), &copy("edited"), Some("new")),
            hold(Some("old"), Issue::Edited)
        );
        assert_eq!(
            decide(false, Some(&old), &Disk::Other, None),
            hold(Some("old"), Issue::Edited)
        );
        assert_eq!(decide(false, Some(&old), &Disk::Absent, None), Step::Forget);
        // Once asked about, an edit is never taken back silently, even when it matches the
        // library.
        let asked = Row {
            digest: Some("old".to_owned()),
            issue: Some(Issue::Edited),
        };
        assert_eq!(
            decide(false, Some(&asked), &copy("new"), Some("new")),
            hold(Some("old"), Issue::Edited)
        );
        assert_eq!(
            decide(false, Some(&asked), &copy("old"), None),
            Step::Remove
        );
        let conflict = Row {
            digest: None,
            issue: Some(Issue::Conflict),
        };
        assert_eq!(
            decide(false, Some(&conflict), &copy("theirs"), None),
            Step::Forget
        );
    }
}
