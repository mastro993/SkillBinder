//! Catalog format v1's field rules, held to what the app's engine enforces so it drops no entry
//! (`docs/specs/explore-and-install/design.md`, Catalog format v1).

/// The longest description the catalog carries, in characters.
pub(super) const DESCRIPTION_LIMIT: usize = 1_024;

/// The library's skill-name rule: at most 64 lowercase ASCII letters, digits, and single
/// internal hyphens (`crates/engine/src/payload/frontmatter.rs`). The engine only warns on a
/// break; the catalog refuses one.
pub(super) fn slug(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && !text.starts_with('-')
        && !text.ends_with('-')
        && !text.contains("--")
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// A skill folder: relative, `/`-separated, with no empty, `.`, or `..` segment; empty for a
/// root SKILL.md.
pub(super) fn path(text: &str) -> bool {
    text.is_empty()
        || (!text.contains('\\')
            && !text.chars().any(char::is_control)
            && text
                .split('/')
                .all(|segment| !segment.is_empty() && segment != "." && segment != ".."))
}

/// An HTTPS URL ending in `.git` that `crates/engine/src/git/validation.rs` accepts: a host,
/// and no credentials, query, fragment, or whitespace.
pub(super) fn repository(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split('/').next().unwrap_or_default();
    url.ends_with(".git")
        && !host.is_empty()
        && !host.starts_with('-')
        && !host.contains('@')
        && rest.len() > host.len() + ".git".len()
        && !url.contains(['?', '#'])
        && !url.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// Cuts `text` to the catalog's description limit.
pub(super) fn description(text: &str) -> String {
    text.chars().take(DESCRIPTION_LIMIT).collect()
}

#[cfg(test)]
mod tests {
    use super::{description, path, repository, slug};

    #[test]
    fn slugs_follow_the_library_name_rule() {
        for valid in ["tdd", "a", "grill-me", "v2-notes", &"a".repeat(64)] {
            assert!(slug(valid), "refused {valid}");
        }
        for invalid in ["", "-a", "a-", "a--b", "Tdd", "a_b", "é", &"a".repeat(65)] {
            assert!(!slug(invalid), "accepted {invalid}");
        }
    }

    #[test]
    fn paths_are_relative_folders() {
        for valid in ["", "skills/tdd", ".claude/skills/a", "skills/.curated/x"] {
            assert!(path(valid), "refused {valid}");
        }
        for invalid in ["/skills", "skills/", "skills//a", "../a", "a/./b", "a\\b"] {
            assert!(!path(invalid), "accepted {invalid}");
        }
    }

    #[test]
    fn repositories_are_credential_free_https_git_urls() {
        assert!(repository("https://github.com/acme/skills.git"));
        for invalid in [
            "http://github.com/acme/skills.git",
            "https://github.com/acme/skills",
            "https://u@github.com/acme/skills.git",
            "https://github.com/acme/skills.git?x=1",
            "https://github.com/a b.git",
            "https:///acme.git",
            "https://github.com.git",
            "git@github.com:acme/skills.git",
        ] {
            assert!(!repository(invalid), "accepted {invalid}");
        }
    }

    #[test]
    fn descriptions_are_cut_by_character() {
        assert_eq!(description(&"é".repeat(2_000)).chars().count(), 1_024);
        assert_eq!(description("short"), "short");
    }
}
