//! Detect crate-boundary-violating paths, including grouped and aliased `use` trees.
use anyhow::ensure;
use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};

/// Lexes Rust source and renders only its code tokens, dropping comments and literals.
pub fn code_text(text: &str) -> anyhow::Result<String> {
    let tokens: TokenStream = text
        .parse()
        .map_err(|error| anyhow::anyhow!("Could not lex Rust source: {error}"))?;
    let mut out = String::with_capacity(text.len());
    render(tokens, &mut out)?;
    Ok(out)
}

/// Reports whether code rendered by [`code_text`] names `needle` as a path.
pub fn imports(source: &str, needle: &str) -> bool {
    use_tree_imports(source, needle) || path_appears(source, needle)
}

fn use_tree_imports(source: &str, needle: &str) -> bool {
    let needle = needle_segments(needle);
    use_items(source).into_iter().any(|item| {
        expand_use(&item)
            .iter()
            .any(|path| starts_with(path, &needle))
    })
}

fn path_appears(source: &str, needle: &str) -> bool {
    compact_paths(source).contains(&format!("{needle}::"))
}

fn needle_segments(needle: &str) -> Vec<String> {
    needle.split("::").map(str::to_string).collect()
}

fn starts_with(path: &[String], needle: &[String]) -> bool {
    path.starts_with(needle)
}

fn compact_paths(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ':' && chars.peek() == Some(&':') {
            chars.next();
            while out.chars().next_back().is_some_and(char::is_whitespace) {
                out.pop();
            }
            while chars.peek().is_some_and(|next| next.is_whitespace()) {
                chars.next();
            }
            out.push_str("::");
            continue;
        }
        if ch.is_whitespace() && out.ends_with("::") {
            continue;
        }
        out.push(ch);
    }
    out
}

fn use_items(source: &str) -> Vec<String> {
    let mut items = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if is_use_keyword(source, i) {
            i += 3;
            skip_ws_bytes(bytes, &mut i);
            if let Some(end) = use_item_end(source, i) {
                items.push(source[i..end].to_string());
                i = end;
                continue;
            }
        }
        i += 1;
    }
    items
}

fn is_use_keyword(source: &str, i: usize) -> bool {
    let bytes = source.as_bytes();
    if i + 3 > bytes.len() || &bytes[i..i + 3] != b"use" {
        return false;
    }
    let before = i
        .checked_sub(1)
        .and_then(|idx| source[idx..].chars().next())
        .is_none_or(|ch| !is_ident_char(ch));
    let after = source[i + 3..]
        .chars()
        .next()
        .is_none_or(|ch| !is_ident_char(ch));
    before && after
}

fn use_item_end(source: &str, start: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, ch) in source[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ';' if depth == 0 => return Some(start + offset),
            _ => {}
        }
    }
    None
}

fn expand_use(item: &str) -> Vec<Vec<String>> {
    let mut cursor = Cursor { rest: item.trim() };
    expand(&[], &mut cursor).unwrap_or_default()
}

fn expand(prefix: &[String], cursor: &mut Cursor<'_>) -> Option<Vec<Vec<String>>> {
    cursor.skip_ws();
    if cursor.eat('{') {
        return expand_group(prefix, cursor);
    }
    if cursor.eat('*') {
        return Some(vec![prefix.to_vec()]);
    }
    let ident = cursor.ident()?;
    let path = with_segment(prefix, &ident);
    cursor.skip_ws();
    if cursor.eat_path_sep() {
        return expand(&path, cursor);
    }
    if cursor.eat_as_alias() {
        return Some(vec![path]);
    }
    Some(vec![path])
}

fn expand_group(prefix: &[String], cursor: &mut Cursor<'_>) -> Option<Vec<Vec<String>>> {
    let mut paths = Vec::new();
    loop {
        cursor.skip_ws();
        if cursor.eat('}') {
            break;
        }
        paths.extend(expand(prefix, cursor)?);
        cursor.skip_ws();
        let _ = cursor.eat(',');
    }
    Some(paths)
}

fn with_segment(prefix: &[String], ident: &str) -> Vec<String> {
    if ident == "self" {
        return prefix.to_vec();
    }
    let mut path = prefix.to_vec();
    path.push(ident.to_string());
    path
}

struct Cursor<'a> {
    rest: &'a str,
}

impl Cursor<'_> {
    fn skip_ws(&mut self) {
        self.rest = self.rest.trim_start();
    }

    fn eat(&mut self, ch: char) -> bool {
        if self.rest.starts_with(ch) {
            self.rest = &self.rest[ch.len_utf8()..];
            true
        } else {
            false
        }
    }

    fn eat_path_sep(&mut self) -> bool {
        self.skip_ws();
        if self.rest.starts_with("::") {
            self.rest = &self.rest[2..];
            true
        } else {
            false
        }
    }

    fn eat_as_alias(&mut self) -> bool {
        self.skip_ws();
        if !self.rest.starts_with("as") {
            return false;
        }
        let after = &self.rest[2..];
        if after.chars().next().is_some_and(is_ident_char) {
            return false;
        }
        self.rest = after;
        self.ident().is_some()
    }

    fn ident(&mut self) -> Option<String> {
        self.skip_ws();
        let rest = self.rest.strip_prefix("r#").unwrap_or(self.rest);
        let len = rest.chars().take_while(|ch| is_ident_char(*ch)).count();
        if len == 0 {
            return None;
        }
        let ident: String = rest.chars().take(len).collect();
        self.rest = &rest[ident.len()..];
        Some(ident)
    }
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn skip_ws_bytes(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
        *i += 1;
    }
}

fn render(tokens: TokenStream, out: &mut String) -> anyhow::Result<()> {
    let mut after_include = false;
    for token in tokens {
        match token {
            TokenTree::Ident(ident) => {
                let name = ident.to_string();
                out.push_str(name.trim_start_matches("r#"));
                out.push(' ');
                after_include = name == "include";
                continue;
            }
            TokenTree::Punct(punct) => {
                ensure!(
                    !(after_include && punct.as_char() == '!'),
                    "include! hides source from boundary checks"
                );
                out.push(punct.as_char());
                if punct.spacing() == Spacing::Alone {
                    out.push(' ');
                }
            }
            TokenTree::Group(group) => {
                let (open, close) = delimiters(group.delimiter());
                out.push_str(open);
                render(group.stream(), out)?;
                out.push_str(close);
            }
            TokenTree::Literal(_) => {}
        }
        after_include = false;
    }
    Ok(())
}

fn delimiters(delimiter: Delimiter) -> (&'static str, &'static str) {
    match delimiter {
        Delimiter::Parenthesis => ("(", ") "),
        Delimiter::Brace => ("{", "} "),
        Delimiter::Bracket => ("[", "] "),
        Delimiter::None => ("", ""),
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "Test fixtures are valid Rust.")]
    use super::{code_text, imports};

    fn hit(text: &str, needle: &str) -> bool {
        imports(&code_text(text).expect("fixture lexes"), needle)
    }

    #[test]
    fn grouped_and_aliased_uses_count() {
        assert!(hit("use std::{fs, path::Path};", "std::fs"));
        assert!(hit("use std::fs as foo;", "std::fs"));
        assert!(hit("use std::{\n    fs,\n    path::Path,\n};", "std::fs"));
        assert!(hit("use std::fs::{self, File};", "std::fs"));
        assert!(hit("use std::r#fs;", "std::fs"));
        assert!(hit("let _ = std :: fs::read(path);", "std::fs"));
        assert!(hit("use diesel::prelude::*;", "diesel"));
        assert!(hit("use diesel;", "diesel"));
    }

    #[test]
    fn comments_strings_and_neighbors_do_not_count() {
        assert!(!hit("// use std::fs::read;\nfn f() {}", "std::fs"));
        assert!(!hit("/* a /* b */ use std::fs::File; */", "std::fs"));
        assert!(!hit("/// std::fs::read\nfn f() {}", "std::fs"));
        assert!(!hit("let s = \"use std::fs\";", "std::fs"));
        assert!(!hit("let s = r#\"std::fs::read\"#;", "std::fs"));
        assert!(!hit("use std::path::Path;", "std::fs"));
    }

    #[test]
    fn labels_and_char_literals_do_not_hide_code() {
        assert!(hit(
            "fn f() { 'outer: loop { std::fs::read(p); break 'outer; } }",
            "std::fs"
        ));
        assert!(hit("fn f() { let q = '\"'; std::fs::read(p); }", "std::fs"));
    }

    #[test]
    fn include_is_rejected() {
        assert!(code_text("include!(\"io.in\");").is_err());
        assert!(code_text("const S: &str = include_str!(\"a.txt\");").is_ok());
    }
}
