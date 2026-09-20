use std::collections::HashSet;
use thiserror::Error;
use yaml_rust2::{Yaml, YamlLoader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub raw: String,
}
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FrontmatterError {
    #[error("frontmatter is missing or exceeds 64 KiB")]
    MissingOrTooLarge,
    #[error("frontmatter YAML is invalid: {0}")]
    Invalid(String),
    #[error("frontmatter uses unsupported YAML features")]
    Unsupported,
}

pub fn parse_frontmatter(bytes: &[u8]) -> Result<Frontmatter, FrontmatterError> {
    if bytes.len() > 1_048_576 {
        return Err(FrontmatterError::MissingOrTooLarge);
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| FrontmatterError::Invalid("SKILL.md is not UTF-8".into()))?;
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Err(FrontmatterError::MissingOrTooLarge);
    }
    let mut raw = String::new();
    let mut found = false;
    for line in lines {
        if line.trim() == "---" {
            found = true;
            break;
        }
        raw.push_str(line);
        raw.push('\n');
        if raw.len() > 65_536 {
            return Err(FrontmatterError::MissingOrTooLarge);
        }
    }
    if !found || raw.len() > 65_536 {
        return Err(FrontmatterError::MissingOrTooLarge);
    }
    let mut keys = HashSet::new();
    let mut scanner = yaml_rust2::scanner::Scanner::new(raw.chars());
    while let Some(token) = scanner
        .next_token()
        .map_err(|error| FrontmatterError::Invalid(error.to_string()))?
    {
        use yaml_rust2::scanner::TokenType;
        if matches!(
            token.1,
            TokenType::Tag(..) | TokenType::Alias(..) | TokenType::Anchor(..)
        ) {
            return Err(FrontmatterError::Unsupported);
        }
    }
    for line in raw.lines() {
        let trimmed = line.trim();
        if !line.starts_with([' ', '\t'])
            && let Some((key, _)) = trimmed.split_once(':')
            && !keys.insert(key.trim().to_owned())
        {
            return Err(FrontmatterError::Unsupported);
        }
    }
    let docs =
        YamlLoader::load_from_str(&raw).map_err(|e| FrontmatterError::Invalid(e.to_string()))?;
    let root = docs
        .first()
        .ok_or_else(|| FrontmatterError::Invalid("empty frontmatter".into()))?;
    let mut nodes = 0usize;
    let mut max_depth = 0usize;
    walk(root, 0, &mut nodes, &mut max_depth);
    if nodes > 10_000 || max_depth > 32 {
        return Err(FrontmatterError::Unsupported);
    }
    let Yaml::Hash(map) = root else {
        return Err(FrontmatterError::Invalid(
            "frontmatter must be a mapping".into(),
        ));
    };
    let get = |key: &str| {
        map.get(&Yaml::String(key.into()))
            .and_then(|value| value.as_str().map(str::to_owned))
    };
    Ok(Frontmatter {
        name: get("name"),
        description: get("description"),
        raw,
    })
}
fn walk(node: &Yaml, depth: usize, nodes: &mut usize, max: &mut usize) {
    *nodes += 1;
    *max = (*max).max(depth);
    match node {
        Yaml::Array(values) => values.iter().for_each(|v| walk(v, depth + 1, nodes, max)),
        Yaml::Hash(values) => values.iter().for_each(|(k, v)| {
            walk(k, depth + 1, nodes, max);
            walk(v, depth + 1, nodes, max);
        }),
        _ => {}
    }
}
