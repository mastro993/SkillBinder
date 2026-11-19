use super::finding;
use skillbinder_proto::{FrontMatterField, FrontMatterValue, ValidationMessage, ValidationStatus};
use yaml_rust2::{
    Yaml, YamlLoader,
    parser::{Event, Parser},
    yaml::Hash,
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
    let Some((name, description)) = name_and_description(bytes) else {
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

/// SKILL.md's front matter `name` and `description`, each when it is a string; `None` when the
/// front matter is missing, malformed, or exceeds the supported YAML limits.
pub(crate) fn name_and_description(skill_md: &[u8]) -> Option<(Option<String>, Option<String>)> {
    let mapping = parse(skill_md)?;
    let field = |key: &str| {
        mapping
            .get(&Yaml::String(key.to_owned()))
            .and_then(Yaml::as_str)
            .map(str::to_owned)
    };
    Some((field("name"), field("description")))
}

/// SKILL.md's front matter fields other than `name` and `description`, in their order: a scalar
/// as text, a list of scalars as a list, `allowed-tools` split into its tools, and each entry of
/// a `metadata` mapping as its own field. Other nested values are left out, and front matter
/// that does not parse gives none.
pub(crate) fn fields(skill_md: &[u8]) -> Vec<FrontMatterField> {
    let Some(mapping) = parse(skill_md) else {
        return Vec::new();
    };
    let mut fields = Vec::new();
    for (key, value) in &mapping {
        let Some(key) = key.as_str() else { continue };
        match (key, value) {
            ("name" | "description", _) => {}
            ("metadata", Yaml::Hash(entries)) => fields.extend(
                entries
                    .iter()
                    .filter_map(|(key, value)| field(key.as_str()?, value)),
            ),
            ("allowed-tools", Yaml::String(tools)) => fields.push(FrontMatterField {
                key: key.to_owned(),
                value: FrontMatterValue::List(split_tools(tools)),
            }),
            _ => fields.extend(field(key, value)),
        }
    }
    fields
}

fn field(key: &str, value: &Yaml) -> Option<FrontMatterField> {
    let value = match value {
        Yaml::Array(items) => FrontMatterValue::List(items.iter().filter_map(text).collect()),
        scalar => FrontMatterValue::Text(text(scalar)?),
    };
    Some(FrontMatterField {
        key: key.to_owned(),
        value,
    })
}

/// A scalar's text: strings as written, numbers and booleans in their YAML spelling.
fn text(value: &Yaml) -> Option<String> {
    match value {
        Yaml::String(text) | Yaml::Real(text) => Some(text.clone()),
        Yaml::Integer(number) => Some(number.to_string()),
        Yaml::Boolean(flag) => Some(flag.to_string()),
        _ => None,
    }
}

/// `allowed-tools` as written, `Read Grep` or `Read, Grep`, split into its tools; a space or
/// comma inside a tool's parentheses, as in `Bash(git log:*)`, stays in it.
fn split_tools(tools: &str) -> Vec<String> {
    let mut split = Vec::new();
    let (mut tool, mut depth) = (String::new(), 0usize);
    for c in tools.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ' ' | ',' | '\t' | '\n' if depth == 0 => {
                if !tool.is_empty() {
                    split.push(std::mem::take(&mut tool));
                }
                continue;
            }
            _ => {}
        }
        tool.push(c);
    }
    if !tool.is_empty() {
        split.push(tool);
    }
    split
}

/// The front matter's mapping, within the YAML limits: no aliases, anchors, or tags, at most
/// 10,000 nodes nested 32 deep, and 64 KB.
fn parse(bytes: &[u8]) -> Option<Hash> {
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
    documents.into_iter().next()?.into_hash()
}
