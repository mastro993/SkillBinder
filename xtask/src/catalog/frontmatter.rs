//! SKILL.md frontmatter, read under the limits the engine's payload check applies
//! (`crates/engine/src/payload/frontmatter.rs`), so a skill the catalog lists also inspects.

use yaml_rust2::{
    Yaml, YamlLoader,
    parser::{Event, Parser},
};

const MAX_BYTES: usize = 65_536;
const MAX_NODES: usize = 10_000;
const MAX_DEPTH: usize = 32;

pub(super) struct Frontmatter {
    pub(super) name: Option<String>,
    pub(super) description: Option<String>,
    /// `metadata.internal: true`, which the `skills` CLI hides from listings.
    pub(super) internal: bool,
}

/// The frontmatter of a SKILL.md, or `None` when it is missing, malformed, or uses anchors,
/// aliases, tags, or more nodes or depth than the engine accepts.
pub(super) fn parse(bytes: &[u8]) -> Option<Frontmatter> {
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
        if yaml.len() + line.len() + 1 > MAX_BYTES {
            return None;
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    if !closed || !plain(&yaml)? {
        return None;
    }
    let documents = YamlLoader::load_from_str(&yaml).ok()?;
    let [document] = documents.as_slice() else {
        return None;
    };
    let mapping = document.as_hash()?;
    let field = |key: &str| mapping.get(&Yaml::String(key.to_owned()));
    let text = |key: &str| field(key).and_then(Yaml::as_str).map(str::to_owned);
    let internal = field("metadata")
        .and_then(Yaml::as_hash)
        .and_then(|metadata| metadata.get(&Yaml::String("internal".to_owned())))
        .and_then(Yaml::as_bool)
        .unwrap_or(false);
    Some(Frontmatter {
        name: text("name"),
        description: text("description"),
        internal,
    })
}

/// Whether `yaml` has no anchors, aliases, or tags and stays within the node and depth limits;
/// `None` when it does not parse.
fn plain(yaml: &str) -> Option<bool> {
    let mut parser = Parser::new_from_str(yaml);
    let (mut nodes, mut depth) = (0usize, 0usize);
    loop {
        match parser.next_token().ok()?.0 {
            Event::StreamEnd => return Some(true),
            Event::Alias(_) => return Some(false),
            Event::Scalar(_, _, anchor, tag) => {
                if anchor != 0 || tag.is_some() {
                    return Some(false);
                }
                nodes += 1;
            }
            Event::MappingStart(anchor, tag) | Event::SequenceStart(anchor, tag) => {
                if anchor != 0 || tag.is_some() {
                    return Some(false);
                }
                depth += 1;
                nodes += 1;
            }
            Event::MappingEnd | Event::SequenceEnd => depth = depth.checked_sub(1)?,
            _ => {}
        }
        if nodes > MAX_NODES || depth > MAX_DEPTH {
            return Some(false);
        }
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "Test assertions report fixture failures."
    )]
    use super::parse;

    #[test]
    fn name_description_and_internal_are_read() {
        let front = parse(
            b"---\nname: tdd\ndescription: Tests first.\nmetadata:\n  internal: true\n---\nBody",
        )
        .expect("valid frontmatter");
        assert_eq!(front.name.as_deref(), Some("tdd"));
        assert_eq!(front.description.as_deref(), Some("Tests first."));
        assert!(front.internal);
        let front = parse(b"---\nname: tdd\n---\n").expect("valid frontmatter");
        assert_eq!(front.description, None);
        assert!(!front.internal);
    }

    #[test]
    fn frontmatter_the_engine_refuses_is_refused() {
        for text in [
            "name: tdd\n",
            "---\nname: tdd\n",
            "---\n- a\n---\n",
            "---\nname: &a tdd\ndescription: *a\n---\n",
            "---\nname: !!str tdd\n---\n",
            "---\nname: tdd\nname: other\n---\n",
        ] {
            assert!(parse(text.as_bytes()).is_none(), "accepted {text:?}");
        }
    }
}
