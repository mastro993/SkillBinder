//! Detect crate-boundary-violating paths, including grouped and aliased `use` trees.

pub fn imports(text: &str, needle: &str) -> bool {
    let source = strip_noise(text);
    use_tree_imports(&source, needle) || path_appears(&source, needle)
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
    let compact = compact_paths(source);
    let token = format!("{needle}::");
    compact.contains(&token) || compact.contains(&format!("use {needle};"))
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

fn strip_noise(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '/' if chars.peek() == Some(&'/') => skip_line(&mut chars),
            '/' if chars.peek() == Some(&'*') => skip_block(&mut chars),
            '"' => skip_quoted(&mut chars, '"'),
            '\'' => skip_quoted(&mut chars, '\''),
            'b' | 'c' | 'r' => push_or_skip_prefixed(&mut out, &mut chars, ch),
            _ => out.push(ch),
        }
    }
    out
}

fn skip_line(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    chars.next();
    for ch in chars.by_ref() {
        if ch == '\n' {
            break;
        }
    }
}

fn skip_block(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    chars.next();
    while let Some(ch) = chars.next() {
        if ch == '*' && chars.peek() == Some(&'/') {
            chars.next();
            break;
        }
    }
}

fn skip_quoted(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, quote: char) {
    let mut escaped = false;
    for ch in chars.by_ref() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            break;
        }
    }
}

fn push_or_skip_prefixed(
    out: &mut String,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    first: char,
) {
    if first == 'r' && skip_raw(chars) {
        return;
    }
    if matches!(first, 'b' | 'c') {
        if chars.peek() == Some(&'r') {
            chars.next();
            if skip_raw(chars) {
                return;
            }
            out.push(first);
            out.push('r');
            return;
        }
        if chars.peek() == Some(&'"') {
            chars.next();
            skip_quoted(chars, '"');
            return;
        }
    }
    out.push(first);
}

fn skip_raw(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    let mut hashes = 0usize;
    while chars.peek() == Some(&'#') {
        chars.next();
        hashes += 1;
    }
    if chars.peek() != Some(&'"') {
        return false;
    }
    chars.next();
    while let Some(ch) = chars.next() {
        if ch != '"' {
            continue;
        }
        let mut seen = 0usize;
        while seen < hashes && chars.peek() == Some(&'#') {
            chars.next();
            seen += 1;
        }
        if seen == hashes {
            return true;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::imports;

    #[test]
    fn grouped_and_aliased_uses_count() {
        assert!(imports("use std::{fs, path::Path};", "std::fs"));
        assert!(imports("use std::fs as foo;", "std::fs"));
        assert!(imports(
            "use std::{\n    fs,\n    path::Path,\n};",
            "std::fs"
        ));
        assert!(imports("use std::fs::{self, File};", "std::fs"));
        assert!(imports("let _ = std :: fs::read(path);", "std::fs"));
        assert!(imports("use diesel::prelude::*;", "diesel"));
    }

    #[test]
    fn comments_strings_and_neighbors_do_not_count() {
        assert!(!imports("// use std::fs::read;\nfn f() {}", "std::fs"));
        assert!(!imports("let s = \"use std::fs\";", "std::fs"));
        assert!(!imports("use std::path::Path;", "std::fs"));
        assert!(imports("use std::fs::File;\nfn f() {}", "std::fs"));
    }
}
