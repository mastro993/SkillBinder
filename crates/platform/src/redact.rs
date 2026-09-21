use std::path::Path;

const CREDENTIAL_KEYS: [&str; 11] = [
    "access_token",
    "api_key",
    "apikey",
    "client_secret",
    "id_token",
    "key",
    "password",
    "passwd",
    "refresh_token",
    "secret",
    "token",
];

pub struct Redactor {
    home: String,
    home_json: String,
}

impl Redactor {
    pub fn new(home: &Path) -> Self {
        let home = home.to_string_lossy().into_owned();
        let home_json = home.replace('\\', "\\\\");
        Self { home, home_json }
    }

    pub fn rewrite_line(&self, line: &str) -> String {
        let stripped = self.strip_home(line);
        let userinfo = redact_userinfo(&stripped);
        let parameters = redact_credential_parameters(&userinfo);
        redact_authorization(&parameters)
    }

    fn strip_home(&self, line: &str) -> String {
        if self.home.is_empty() {
            return line.to_string();
        }
        line.replace(&self.home_json, "~").replace(&self.home, "~")
    }
}

fn redact_userinfo(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(position) = rest.find("://") {
        result.push_str(&rest[..position + 3]);
        let authority = &rest[position + 3..];
        let (authority, remainder) = authority.split_at(authority_end(authority));
        match authority.find('@') {
            Some(at) => {
                result.push_str("***");
                result.push_str(&authority[at..]);
            }
            None => result.push_str(authority),
        }
        rest = remainder;
    }
    result.push_str(rest);
    result
}

fn redact_credential_parameters(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut rest = line;
    loop {
        let Some(position) = rest.find(['?', '&']) else {
            result.push_str(rest);
            return result;
        };
        result.push_str(&rest[..=position]);
        let candidate = &rest[position + 1..];
        let name_end = candidate
            .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
            .unwrap_or(candidate.len());
        let name = &candidate[..name_end];
        let Some(value) = candidate[name_end..].strip_prefix('=') else {
            rest = candidate;
            continue;
        };
        if !CREDENTIAL_KEYS
            .iter()
            .any(|key| key.eq_ignore_ascii_case(name))
        {
            rest = candidate;
            continue;
        }
        result.push_str(name);
        result.push('=');
        let end = secret_value_end(value);
        result.push_str("***");
        rest = &value[end..];
    }
}

fn redact_authorization(line: &str) -> String {
    let lowered = line.to_ascii_lowercase();
    let mut result = String::with_capacity(line.len());
    let mut index = 0;
    while index < line.len() {
        let remaining = &lowered[index..];
        if remaining.starts_with("bearer") {
            let word_end = index + "bearer".len();
            let value_start = skip_spaces(line, word_end);
            if value_start > word_end {
                let end = value_start + secret_value_end(&line[value_start..]);
                result.push_str(&line[index..value_start]);
                result.push_str("***");
                index = end;
                continue;
            }
        } else if remaining.starts_with("authorization") {
            let word_end = index + "authorization".len();
            let mut separator = skip_spaces(line, word_end);
            if matches!(line.as_bytes().get(separator).copied(), Some(b'"' | b'\'')) {
                separator = skip_spaces(line, separator + 1);
            }
            if matches!(line.as_bytes().get(separator).copied(), Some(b':' | b'=')) {
                let mut value_start = skip_spaces(line, separator + 1);
                if matches!(
                    line.as_bytes().get(value_start).copied(),
                    Some(b'"' | b'\'')
                ) {
                    value_start += 1;
                }
                let end = value_start + header_value_end(&line[value_start..]);
                result.push_str(&line[index..value_start]);
                result.push_str("***");
                index = end;
                continue;
            }
        }
        let Some(character) = line[index..].chars().next() else {
            break;
        };
        result.push(character);
        index += character.len_utf8();
    }
    result
}

fn skip_spaces(line: &str, mut index: usize) -> usize {
    while matches!(line.as_bytes().get(index).copied(), Some(b' ' | b'\t')) {
        index += 1;
    }
    index
}

fn authority_end(authority: &str) -> usize {
    authority
        .find(|ch: char| {
            ch.is_whitespace() || matches!(ch, '/' | '?' | '#' | '"' | '\'' | '\\' | '<' | '>')
        })
        .unwrap_or(authority.len())
}

fn secret_value_end(value: &str) -> usize {
    value
        .find(|ch: char| {
            ch.is_whitespace()
                || matches!(
                    ch,
                    '&' | '#' | '"' | '\'' | '\\' | ',' | '}' | ')' | ';' | '<' | '>'
                )
        })
        .unwrap_or(value.len())
}

fn header_value_end(value: &str) -> usize {
    value
        .find(['&', '#', '"', '\'', '\\', ',', '}', '<', '>'])
        .unwrap_or(value.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn home() -> PathBuf {
        std::env::temp_dir().join("skillbinder-redact-home")
    }

    fn redactor() -> Redactor {
        Redactor::new(&home())
    }

    fn home_display() -> String {
        home().to_string_lossy().into_owned()
    }

    #[test]
    fn a_home_prefix_becomes_a_tilde() {
        let line = format!("opened {}", home().join("library").display());
        let rewritten = redactor().rewrite_line(&line);

        assert!(!rewritten.contains(&home_display()));
        assert_eq!(rewritten, "opened ~/library");
    }

    #[test]
    fn a_home_prefix_inside_a_json_string_becomes_a_tilde() {
        let line = format!("{{\"cwd\":\"{}\"}}", home().join("staging").display());
        let rewritten = redactor().rewrite_line(&line);

        assert!(!rewritten.contains(&home_display()));
        assert_eq!(rewritten, "{\"cwd\":\"~/staging\"}");
    }

    #[test]
    fn url_userinfo_becomes_stars() {
        assert_eq!(
            redactor().rewrite_line("fetching https://user:token@example.test/skill?page=2"),
            "fetching https://***@example.test/skill?page=2"
        );
    }

    #[test]
    fn credential_query_values_become_stars() {
        let cases = [
            (
                "https://example.test/x?token=abc",
                "https://example.test/x?token=***",
            ),
            (
                "https://example.test/x?page=2&access_token=abc&sort=name",
                "https://example.test/x?page=2&access_token=***&sort=name",
            ),
            (
                "https://example.test/x?secret=abc",
                "https://example.test/x?secret=***",
            ),
            (
                "https://example.test/x?password=abc",
                "https://example.test/x?password=***",
            ),
            (
                "https://example.test/x?key=abc",
                "https://example.test/x?key=***",
            ),
            (
                "request {\"url\":\"https://example.test/x?api_key=abc\"}",
                "request {\"url\":\"https://example.test/x?api_key=***\"}",
            ),
            (
                "https://example.test/x?page=2",
                "https://example.test/x?page=2",
            ),
        ];
        for (input, expected) in cases {
            assert_eq!(redactor().rewrite_line(input), expected, "input {input}");
        }
    }

    #[test]
    fn bearer_and_authorization_values_become_stars() {
        let cases = [
            ("header Bearer abc123", "header Bearer ***"),
            ("header bearer abc123", "header bearer ***"),
            (
                "header Authorization: Basic abc123",
                "header Authorization: ***",
            ),
            ("header authorization=abc123", "header authorization=***"),
            (
                "{\"Authorization\": \"token abc\"}",
                "{\"Authorization\": \"***\"}",
            ),
            ("authorization is required", "authorization is required"),
        ];
        for (input, expected) in cases {
            assert_eq!(redactor().rewrite_line(input), expected, "input {input}");
        }
    }

    #[test]
    fn a_clean_line_is_unchanged() {
        let line = "indexed 12 skills from ~/library in 340ms";
        assert_eq!(redactor().rewrite_line(line), line);
    }

    #[test]
    fn the_rewrite_is_idempotent() {
        let inputs = [
            format!("opened {}", home().join("library").display()),
            format!("{{\"cwd\":\"{}\"}}", home().join("staging").display()),
            "fetching https://user:token@example.test/skill?page=2".to_string(),
            "https://example.test/x?page=2&access_token=abc".to_string(),
            "header Bearer abc123".to_string(),
            "header Authorization: Basic abc123".to_string(),
            "indexed 12 skills from ~/library in 340ms".to_string(),
        ];
        for input in inputs {
            let once = redactor().rewrite_line(&input);
            let twice = redactor().rewrite_line(&once);
            assert_eq!(once, twice, "input {input}");
        }
    }
}
