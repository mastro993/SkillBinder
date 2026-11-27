//! The message of a commit Push or Sync makes (ADR 0038): a subject naming the device and each
//! skill, one body line per skill saying why it changed, and trailers that other devices read.

use skillbinder_proto::{ChangeKind as Kind, Reason, Side};
use std::collections::BTreeMap;

/// The trailer naming the device that made a commit.
pub(super) const DEVICE: &str = "SkillBinder-Device";
/// The trailer saying why a commit changed one skill.
pub(super) const SKILL: &str = "SkillBinder-Skill";
/// The trailer saying whose version of a skill changed on both sides a merge kept.
pub(super) const CHOICE: &str = "SkillBinder-Choice";
/// The longest subject, in characters.
const SUBJECT: usize = 72;
/// Characters Git drops around an author name; one made only of these is refused.
const CRUD: &str = ".,:;<>\"\\'";

/// How a change treats a skill whose directory is there `before` and `after` it, or not.
pub(super) fn kind(before: bool, after: bool) -> Kind {
    match (before, after) {
        (false, true) => Kind::Added,
        (true, true) => Kind::Changed,
        (_, false) => Kind::Removed,
    }
}

fn verb(kind: Kind) -> &'static str {
    match kind {
        Kind::Added => "add",
        Kind::Changed => "update",
        Kind::Removed => "remove",
    }
}

/// The slug and reason a `SkillBinder-Skill` trailer's value records, the inverse of
/// [`Note::trailers`].
pub(super) fn skill_trailer(value: &str) -> Option<(String, Reason)> {
    let (slug, rest) = value.trim().split_once(' ')?;
    let (word, detail) = match rest.split_once(' ') {
        Some((word, detail)) => (word, Some(detail.to_owned())),
        None => (rest, None),
    };
    Some((decoded(slug), Reason::stored(word, detail)))
}

/// The slug and side a `SkillBinder-Choice` trailer's value records.
pub(super) fn choice_trailer(value: &str) -> Option<(String, Side)> {
    let (slug, side) = value.trim().split_once(' ')?;
    Some((decoded(slug), Side::stored(side)?))
}

/// A slug with its spaces and percent signs percent-encoded, so its end in a trailer is the
/// first space.
fn encoded(slug: &str) -> String {
    slug.replace('%', "%25").replace(' ', "%20")
}

fn decoded(slug: &str) -> String {
    slug.replace("%20", " ").replace("%25", "%")
}

/// What a commit says: the device that made it, each skill it changes and why, and for a merge,
/// whose version of each skill changed on both sides it kept.
pub(super) struct Note {
    device: String,
    /// Added skills first, then changed, then removed, each by slug.
    skills: Vec<(String, Kind, Reason)>,
    /// For a merge, the choices, by slug; `None` for any other commit.
    choices: Option<Vec<(String, Side)>>,
}

impl Note {
    /// The note for `kinds`, each skill's reason taken from `reasons`, else `Outside`.
    pub(super) fn new(
        device: String,
        kinds: BTreeMap<String, Kind>,
        reasons: &BTreeMap<String, Reason>,
    ) -> Self {
        let mut skills: Vec<_> = kinds
            .into_iter()
            .map(|(slug, kind)| {
                let reason = reasons.get(&slug).cloned().unwrap_or(Reason::Outside);
                (slug, kind, reason)
            })
            .collect();
        skills.sort_by(|a, b| (a.1, &a.0).cmp(&(b.1, &b.0)));
        Self {
            device,
            skills,
            choices: None,
        }
    }

    /// A merge's note: what it changes against the remote's commit, as [`Note::new`] says, and
    /// each choice.
    pub(super) fn merge(
        device: String,
        kinds: BTreeMap<String, Kind>,
        reasons: &BTreeMap<String, Reason>,
        choices: &BTreeMap<String, Side>,
    ) -> Self {
        let mut note = Self::new(device, kinds, reasons);
        note.choices = Some(choices.iter().map(|(s, side)| (s.clone(), *side)).collect());
        note
    }

    /// `<device>: add a, update b, remove c`, cut to 72 characters with "and <n> more".
    pub(super) fn subject(&self) -> String {
        let head = format!("{}: ", self.device);
        let parts: Vec<String> = self
            .skills
            .iter()
            .map(|(slug, kind, _)| format!("{} {slug}", verb(*kind)))
            .collect();
        if parts.is_empty() {
            return match self.choices {
                Some(_) => format!("{head}sync with the remote"),
                None => format!("{head}start the library"),
            };
        }
        let with = |shown: usize| {
            let mut subject = head.clone() + &parts[..shown].join(", ");
            if shown < parts.len() {
                subject += &format!(" and {} more", parts.len() - shown);
            }
            subject
        };
        (1..=parts.len())
            .rev()
            .map(with)
            .find(|subject| subject.chars().count() <= SUBJECT)
            // Not even one skill fits beside a long device name.
            .unwrap_or_else(|| {
                let cut: String = with(1).chars().take(SUBJECT - 1).collect();
                cut + "…"
            })
    }

    /// One line per skill: how it changed and why; then one per choice.
    pub(super) fn body(&self) -> String {
        let skills = (self.skills.iter())
            .map(|(slug, kind, reason)| format!("{} {slug}: {}", verb(*kind), why(reason, *kind)));
        let choices = self
            .choices
            .iter()
            .flatten()
            .map(|(slug, side)| match side {
                Side::Local => format!("keep {slug}: this device's version"),
                Side::Remote => format!("keep {slug}: the remote's version"),
            });
        skills.chain(choices).collect::<Vec<_>>().join("\n")
    }

    /// `SkillBinder-Device: <device>`, then `SkillBinder-Skill: <slug> <reason>[ <detail>]` per
    /// skill. A slug's spaces and percent signs are percent-encoded, so its end is the first
    /// space.
    pub(super) fn trailers(&self) -> String {
        let mut lines = vec![format!("{DEVICE}: {}", self.device)];
        for (slug, _, reason) in &self.skills {
            let slug = encoded(slug);
            let mut line = format!("{SKILL}: {slug} {}", reason.word());
            if let Some(detail) = reason.detail() {
                line = format!("{line} {}", one_line(&detail));
            }
            lines.push(line);
        }
        for (slug, side) in self.choices.iter().flatten() {
            lines.push(format!("{CHOICE}: {} {}", encoded(slug), side.word()));
        }
        lines.join("\n")
    }

    /// Git's `-c` options naming the device as the commit's author and committer.
    pub(super) fn identity(&self) -> Vec<String> {
        let author = self.author();
        let mut args = Vec::new();
        for role in ["author", "committer"] {
            args.extend([
                "-c".to_owned(),
                format!("{role}.name={author}"),
                "-c".to_owned(),
                format!("{role}.email=local@skillbinder.invalid"),
            ]);
        }
        args
    }

    /// The author and committer name: the device's, without the characters Git refuses in one.
    fn author(&self) -> String {
        let name: String = self.device.chars().filter(|c| !"<>".contains(*c)).collect();
        if name.chars().all(|c| c.is_whitespace() || CRUD.contains(c)) {
            "SkillBinder".to_owned()
        } else {
            name
        }
    }
}

/// Why a skill changed, as a commit's body says it.
fn why(reason: &Reason, kind: Kind) -> String {
    match reason {
        Reason::Imported { from: Some(from) } => format!("imported from {}", one_line(from)),
        Reason::Imported { from: None } => "imported".to_owned(),
        Reason::Installed => "installed from Explore".to_owned(),
        Reason::Updated => "updated from its source".to_owned(),
        Reason::Reinstalled => "reinstalled".to_owned(),
        Reason::Restored { at } => match i64::try_from(*at)
            .ok()
            .and_then(|at| chrono::DateTime::from_timestamp(at, 0))
        {
            Some(at) => format!("restored to the version of {}", at.format("%Y-%m-%d")),
            None => "restored to an earlier version".to_owned(),
        },
        Reason::Record => "record added".to_owned(),
        Reason::Outside if kind == Kind::Removed => "removed outside SkillBinder".to_owned(),
        Reason::Outside => "changed outside SkillBinder".to_owned(),
    }
}

/// `text` with each control character, such as a line break, made a space.
fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Kind, Note, choice_trailer, skill_trailer};
    use skillbinder_proto::{Reason, Side};
    use std::collections::BTreeMap;

    fn note(device: &str, skills: &[(&str, Kind)], reasons: &[(&str, Reason)]) -> Note {
        Note::new(
            device.to_owned(),
            skills.iter().map(|(s, k)| ((*s).to_owned(), *k)).collect(),
            &reasons
                .iter()
                .map(|(s, r)| ((*s).to_owned(), r.clone()))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    #[test]
    fn the_subject_names_the_device_and_each_skill_by_kind() {
        let note = note(
            "Studio Mac",
            &[
                ("notes", Kind::Changed),
                ("old", Kind::Removed),
                ("review", Kind::Added),
                ("deploy", Kind::Changed),
            ],
            &[],
        );
        assert_eq!(
            note.subject(),
            "Studio Mac: add review, update deploy, update notes, remove old"
        );
    }

    #[test]
    fn a_long_subject_is_cut_at_72_characters_with_how_many_more() {
        let slugs: Vec<String> = (0..12).map(|n| format!("skill-{n:02}")).collect();
        let skills: Vec<_> = slugs.iter().map(|s| (s.as_str(), Kind::Added)).collect();
        let subject = note("Studio Mac", &skills, &[]).subject();
        assert_eq!(
            subject,
            "Studio Mac: add skill-00, add skill-01, add skill-02 and 9 more"
        );
        let long = note(&"é".repeat(64), &[("review", Kind::Added)], &[]).subject();
        assert_eq!(long.chars().count(), 72);
        assert!(long.ends_with('…'));
        assert_eq!(
            note("Studio Mac", &[], &[]).subject(),
            "Studio Mac: start the library"
        );
    }

    #[test]
    fn the_body_and_trailers_say_why_each_skill_changed() {
        let note = note(
            "Studio Mac",
            &[
                ("review", Kind::Added),
                ("deploy", Kind::Changed),
                ("my notes", Kind::Changed),
                ("old", Kind::Removed),
            ],
            &[
                (
                    "review",
                    Reason::Imported {
                        from: Some("Claude\nCode".into()),
                    },
                ),
                ("deploy", Reason::Updated),
            ],
        );
        assert_eq!(
            note.body(),
            "add review: imported from Claude Code\n\
             update deploy: updated from its source\n\
             update my notes: changed outside SkillBinder\n\
             remove old: removed outside SkillBinder"
        );
        assert_eq!(
            note.trailers(),
            "SkillBinder-Device: Studio Mac\n\
             SkillBinder-Skill: review imported Claude Code\n\
             SkillBinder-Skill: deploy updated\n\
             SkillBinder-Skill: my%20notes outside\n\
             SkillBinder-Skill: old outside"
        );
    }

    #[test]
    fn each_skill_trailer_reads_back_as_written() {
        let reasons = [
            (
                "review",
                Reason::Imported {
                    from: Some("Claude Code".into()),
                },
            ),
            ("my notes", Reason::Updated),
            ("100%", Reason::Outside),
        ];
        let note = note(
            "Studio Mac",
            &reasons
                .iter()
                .map(|(s, _)| (*s, Kind::Changed))
                .collect::<Vec<_>>(),
            &reasons,
        );
        let read: BTreeMap<_, _> = note
            .trailers()
            .lines()
            .filter_map(|line| line.strip_prefix("SkillBinder-Skill: "))
            .filter_map(skill_trailer)
            .collect();
        let written: BTreeMap<_, _> = reasons
            .iter()
            .map(|(slug, reason)| ((*slug).to_owned(), reason.clone()))
            .collect();
        assert_eq!(read, written);
        assert_eq!(skill_trailer("lonely"), None);
    }

    #[test]
    fn a_merge_says_each_choice_and_reads_it_back() {
        let choices = BTreeMap::from([
            ("my notes".to_owned(), Side::Remote),
            ("review".to_owned(), Side::Local),
        ]);
        let kinds = BTreeMap::from([("review".to_owned(), Kind::Changed)]);
        let merge = Note::merge("Laptop".into(), kinds, &BTreeMap::new(), &choices);
        assert_eq!(merge.subject(), "Laptop: update review");
        assert_eq!(
            merge.body(),
            "update review: changed outside SkillBinder\n\
             keep my notes: the remote's version\n\
             keep review: this device's version"
        );
        let trailers = merge.trailers();
        let read: BTreeMap<_, _> = (trailers.lines())
            .filter_map(|line| line.strip_prefix("SkillBinder-Choice: "))
            .filter_map(choice_trailer)
            .collect();
        assert_eq!(read, choices);
        let nothing = Note::merge("Laptop".into(), BTreeMap::new(), &BTreeMap::new(), &choices);
        assert_eq!(nothing.subject(), "Laptop: sync with the remote");
        assert_eq!(choice_trailer("review mine"), None);
    }

    #[test]
    fn the_author_drops_what_git_refuses_in_a_name() {
        assert_eq!(note("<Studio> Mac", &[], &[]).author(), "Studio Mac");
        assert_eq!(note("<.>", &[], &[]).author(), "SkillBinder");
    }
}
