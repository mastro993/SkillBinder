use super::finding;
use skillbinder_proto::{ValidationMessage, ValidationStatus};
use yaml_rust2::{
    Yaml, YamlLoader,
    parser::{Event, Parser},
};

pub(super) fn validate(
    bytes: Option<&[u8]>,
    slug: &str,
    messages: &mut Vec<ValidationMessage>,
) -> String {
    let Some(bytes) = bytes else {
        finding(messages, ValidationStatus::Invalid, "SKILL.md is missing.");
        return String::new();
    };
    let Some((name, description)) = parse(bytes) else {
        finding(
            messages,
            ValidationStatus::Invalid,
            "SKILL.md frontmatter is missing, malformed, or exceeds supported YAML limits.",
        );
        return String::new();
    };
    match name.as_deref() {
        None | Some("") => finding(
            messages,
            ValidationStatus::Invalid,
            "The skill name is missing.",
        ),
        Some(name) => {
            if name != slug {
                finding(
                    messages,
                    ValidationStatus::Invalid,
                    "The skill name does not match its directory.",
                );
            }
            if name.chars().count() > 64
                || name.starts_with('-')
                || name.ends_with('-')
                || name.contains("--")
                || !name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                finding(
                    messages,
                    ValidationStatus::Warning,
                    "The skill name should use at most 64 lowercase letters, digits, and single internal hyphens.",
                );
            }
        }
    }
    let description = description.unwrap_or_default();
    if description.trim().is_empty() {
        finding(
            messages,
            ValidationStatus::Invalid,
            "The skill description is missing.",
        );
    }
    if description.chars().count() > 1024 {
        finding(
            messages,
            ValidationStatus::Warning,
            "The skill description exceeds 1,024 characters.",
        );
    }
    description
}

fn parse(bytes: &[u8]) -> Option<(Option<String>, Option<String>)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut yaml = String::new();
    let mut closed = false;
    for line in lines {
        if line.trim() == "---" {
            closed = true;
            break;
        }
        if yaml.len() + line.len() + 1 > 65_536 {
            return None;
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    if !closed {
        return None;
    }
    let mut parser = Parser::new_from_str(&yaml);
    let mut nodes = 0;
    let mut depth = 0usize;
    loop {
        match parser.next_token().ok()?.0 {
            Event::StreamEnd => break,
            Event::Alias(_) => return None,
            Event::Scalar(_, _, anchor, tag) => {
                if anchor != 0 || tag.is_some() {
                    return None;
                }
                nodes += 1;
            }
            Event::MappingStart(anchor, tag) | Event::SequenceStart(anchor, tag) => {
                if anchor != 0 || tag.is_some() {
                    return None;
                }
                depth += 1;
                nodes += 1;
            }
            Event::MappingEnd | Event::SequenceEnd => depth = depth.checked_sub(1)?,
            _ => {}
        }
        if nodes > 10_000 || depth > 32 {
            return None;
        }
    }
    // The loader rejects duplicate mapping keys, including quoted/flow spellings.
    let documents = YamlLoader::load_from_str(&yaml).ok()?;
    if documents.len() != 1 {
        return None;
    }
    let mapping = documents.first()?.as_hash()?;
    let field = |key: &str| {
        mapping
            .get(&Yaml::String(key.to_owned()))
            .and_then(Yaml::as_str)
            .map(str::to_owned)
    };
    Some((field("name"), field("description")))
}
