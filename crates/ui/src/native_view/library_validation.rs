//! Library skills whose validation reports warnings or fails, such as one pulled from another
//! device with invalid `SKILL.md` frontmatter: the glyph on their row or card.

use super::controls::app_icon;
use gpui_kit::component::{ActiveTheme, Sizable, tooltip::Tooltip};
use gpui_kit::{
    AnyElement, App, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, div,
};
use skillbinder_proto::{SkillDetails, SkillId, ValidationStatus};

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

/// A skill's mark, or `None` while it is valid or has no details.
fn mark(details: Option<&SkillDetails>) -> Option<Mark> {
    let details = details?;
    let (tone, heading) = match details.validation {
        ValidationStatus::Valid => return None,
        ValidationStatus::Warning => (Tone::Warning, "Passes validation with warnings"),
        ValidationStatus::Invalid => (Tone::Danger, "Fails validation"),
        ValidationStatus::Blocked => (Tone::Danger, "Blocked: this skill can't safely be copied"),
    };
    let tooltip = std::iter::once(heading)
        .chain(
            details
                .messages
                .iter()
                .map(|message| message.message.as_str()),
        )
        .collect::<Vec<_>>()
        .join("\n");
    Some(Mark { tone, tooltip })
}

/// The alert glyph of a skill whose validation reports warnings or fails: warning-colored for
/// warnings, danger-colored while it is invalid or blocked.
pub(super) fn validation_glyph(
    id: &SkillId,
    details: Option<&SkillDetails>,
    cx: &App,
) -> Option<AnyElement> {
    let Mark { tone, tooltip } = mark(details)?;
    let color = match tone {
        Tone::Warning => cx.theme().warning,
        Tone::Danger => cx.theme().danger,
    };
    Some(
        div()
            .id(format!("validation-{id}"))
            .flex_shrink_0()
            .child(app_icon("alert-circle").small().text_color(color))
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_proto::ValidationMessage;

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

    #[test]
    fn a_valid_skill_or_one_without_details_has_no_mark() {
        assert_eq!(mark(None), None);
        assert_eq!(mark(Some(&details(ValidationStatus::Valid, &[]))), None);
    }

    #[test]
    fn warnings_mark_in_the_warning_color_with_their_findings() {
        let warned = details(
            ValidationStatus::Warning,
            &[(ValidationStatus::Warning, ".git metadata is excluded.")],
        );
        assert_eq!(
            mark(Some(&warned)),
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
            mark(Some(&invalid)),
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
            mark(Some(&blocked)),
            Some(Mark {
                tone: Tone::Danger,
                tooltip:
                    "Blocked: this skill can't safely be copied\nPlugin manifests are unsupported."
                        .to_owned(),
            })
        );
    }
}
