//! Library skills whose validation reports warnings or fails, such as one pulled from another
//! device with invalid `SKILL.md` frontmatter: the glyph on their row or card. A found skill's
//! row on a page marks its version the same way.

use gpui_kit::{AnyElement, IntoElement};
use skillbinder_design::{Status, StatusGlyph};
use skillbinder_proto::{Candidate, SkillDetails, SkillId, ValidationMessage, ValidationStatus};

/// The color of a validation glyph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Tone {
    /// Advisory findings only.
    Warning,
    /// The payload fails validation or cannot safely be copied.
    Danger,
}

/// How a row or card marks a skill's validation: the glyph's color, and a tooltip of its heading
/// followed by each finding on its own line.
#[derive(Debug, Eq, PartialEq)]
struct Mark {
    tone: Tone,
    tooltip: String,
}

/// The mark of a skill validated as `validation` with `messages`, or `None` while it is valid.
fn mark(validation: ValidationStatus, messages: &[ValidationMessage]) -> Option<Mark> {
    let (tone, heading) = match validation {
        ValidationStatus::Valid => return None,
        ValidationStatus::Warning => (Tone::Warning, "Passes validation with warnings"),
        ValidationStatus::Invalid => (Tone::Danger, "Fails validation"),
        ValidationStatus::Blocked => (Tone::Danger, "Blocked: this skill can't safely be copied"),
    };
    let tooltip = std::iter::once(heading)
        .chain(messages.iter().map(|message| message.message.as_str()))
        .collect::<Vec<_>>()
        .join("\n");
    Some(Mark { tone, tooltip })
}

/// The alert glyph of a skill whose validation reports warnings or fails: warning-colored for
/// warnings, danger-colored while it is invalid or blocked.
pub(super) fn validation_glyph(id: &SkillId, details: Option<&SkillDetails>) -> Option<AnyElement> {
    let details = details?;
    let mark = mark(details.validation, &details.messages)?;
    Some(glyph(format!("validation-{id}"), mark))
}

/// The glyph of a found skill's `version`, as [`validation_glyph`] marks a library skill.
pub(super) fn found_glyph(version: &Candidate) -> Option<AnyElement> {
    let mark = mark(version.validation, &version.messages)?;
    Some(glyph(format!("found-validation-{}", version.key), mark))
}

fn glyph(id: String, Mark { tone, tooltip }: Mark) -> AnyElement {
    let status = match tone {
        Tone::Warning => Status::Warning,
        Tone::Danger => Status::Danger,
    };
    StatusGlyph::new(id, status, "alert-circle", tooltip).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details(
        validation: ValidationStatus,
        messages: &[(ValidationStatus, &str)],
    ) -> SkillDetails {
        SkillDetails {
            description: String::new(),
            validation,
            messages: messages
                .iter()
                .map(|&(status, message)| ValidationMessage {
                    status,
                    message: message.to_owned(),
                })
                .collect(),
        }
    }

    fn marked(details: &SkillDetails) -> Option<Mark> {
        mark(details.validation, &details.messages)
    }

    #[test]
    fn a_valid_skill_or_one_without_details_has_no_mark() {
        assert!(validation_glyph(&SkillId("a".to_owned()), None).is_none());
        assert_eq!(marked(&details(ValidationStatus::Valid, &[])), None);
    }

    #[test]
    fn warnings_mark_in_the_warning_color_with_their_findings() {
        let warned = details(
            ValidationStatus::Warning,
            &[(ValidationStatus::Warning, ".git metadata is excluded.")],
        );
        assert_eq!(
            marked(&warned),
            Some(Mark {
                tone: Tone::Warning,
                tooltip: "Passes validation with warnings\n.git metadata is excluded.".to_owned(),
            })
        );
    }

    #[test]
    fn invalid_and_blocked_skills_mark_in_the_danger_color_with_every_finding() {
        let invalid = details(
            ValidationStatus::Invalid,
            &[
                (
                    ValidationStatus::Invalid,
                    "The skill description is missing.",
                ),
                (ValidationStatus::Warning, ".git metadata is excluded."),
            ],
        );
        assert_eq!(
            marked(&invalid),
            Some(Mark {
                tone: Tone::Danger,
                tooltip: "Fails validation\nThe skill description is missing.\n\
                          .git metadata is excluded."
                    .to_owned(),
            })
        );
        let blocked = details(
            ValidationStatus::Blocked,
            &[(
                ValidationStatus::Blocked,
                "Plugin manifests are unsupported.",
            )],
        );
        assert_eq!(
            marked(&blocked),
            Some(Mark {
                tone: Tone::Danger,
                tooltip:
                    "Blocked: this skill can't safely be copied\nPlugin manifests are unsupported."
                        .to_owned(),
            })
        );
    }
}
