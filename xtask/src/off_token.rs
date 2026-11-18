//! The off-token ratchet: raw size, color, and alpha literals in `crates/ui` outside tests.
//!
//! Sizes, colors, and alphas come from the design-system tokens, so every literal is debt. `xtask
//! verify` counts them and fails when the count rises above the checked-in baseline;
//! `cargo run -p xtask -- off-token` lowers the baseline after a change removes some.
use anyhow::{Context, bail, ensure};
use std::{fs, path::Path};

const CRATE_ROOT: &str = "crates/ui/src/lib.rs";
const BASELINE: &str = "xtask/off-token-baseline.txt";
const BASELINE_HEADER: &str = "\
# Raw px(<number>), rgb(0x…), hsla(…), and .opacity(<number>) / .alpha(<number>) calls in
# crates/ui/src outside tests, which `cargo run -p xtask -- verify` counts. The count may only go
# down: take values from crates/design's tokens, then run `cargo run -p xtask -- off-token`.
";

/// What a literal sets.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Kind {
    Size,
    Color,
    Alpha,
}

/// Fails when `crates/ui` has more literals than the baseline records.
pub fn verify(root: &Path) -> anyhow::Result<()> {
    let count = count(root)?.len();
    let baseline = baseline(root)?;
    ensure!(
        count <= baseline,
        "crates/ui has {count} raw size, color, or alpha literals, more than the {baseline} in {BASELINE}; take the new values from crates/design's tokens (`cargo run -p xtask -- off-token` lists every literal)"
    );
    if count < baseline {
        println!(
            "crates/ui has {count} off-token literals, below the baseline of {baseline}; run `cargo run -p xtask -- off-token` to lower it."
        );
    }
    Ok(())
}

/// Lists every literal, and lowers the baseline to the count; it never raises it.
pub fn lower(root: &Path) -> anyhow::Result<()> {
    let found = count(root)?;
    for (file, line, kind) in &found {
        println!("{file}:{line}: {kind:?}");
    }
    let baseline = baseline(root)?;
    let count = found.len();
    if count > baseline {
        bail!("{count} literals is above the baseline of {baseline}; remove the new ones instead");
    }
    let path = root.join(BASELINE);
    fs::write(&path, format!("{BASELINE_HEADER}{count}\n"))
        .with_context(|| format!("Could not write {}", path.display()))?;
    println!("Baseline {baseline} → {count}.");
    Ok(())
}

fn baseline(root: &Path) -> anyhow::Result<usize> {
    let path = root.join(BASELINE);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("Missing baseline {}", path.display()))?;
    text.lines()
        .find(|line| !line.starts_with('#') && !line.trim().is_empty())
        .and_then(|line| line.trim().parse().ok())
        .with_context(|| format!("{} must hold one count", path.display()))
}

/// Every literal as (file, line, kind), following `mod` declarations from the crate root and
/// skipping `#[cfg(test)]` items, so test modules in their own files are skipped too.
fn count(root: &Path) -> anyhow::Result<Vec<(String, usize, Kind)>> {
    let mut found = Vec::new();
    let mut pending = vec![root.join(CRATE_ROOT)];
    while let Some(path) = pending.pop() {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("Could not read {}", path.display()))?;
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        let dir = match path.file_stem().and_then(|stem| stem.to_str()) {
            Some("lib" | "main") => path.parent().map(Path::to_path_buf),
            _ => path
                .parent()
                .zip(path.file_stem())
                .map(|(dir, stem)| dir.join(stem)),
        }
        .context("Module path has no directory")?;
        let lines: Vec<&str> = text.lines().collect();
        let mut index = 0;
        while let Some(line) = lines.get(index) {
            let code = code(line);
            if code.trim() == "#[cfg(test)]" {
                index = item_end(&lines, index + 1);
                continue;
            }
            if let Some(name) = module(code) {
                pending.push(dir.join(format!("{name}.rs")));
            }
            for kind in literals(code) {
                found.push((relative.clone(), index + 1, kind));
            }
            index += 1;
        }
    }
    found.sort();
    Ok(found)
}

/// The line before any `//` comment that is not inside a string.
fn code(line: &str) -> &str {
    let mut in_string = false;
    let mut previous = ' ';
    for (at, char) in line.char_indices() {
        match char {
            '"' if previous != '\\' => in_string = !in_string,
            '/' if previous == '/' && !in_string => return &line[..at - 1],
            _ => {}
        }
        previous = char;
    }
    line
}

/// The index after the item starting at `start`: through its braces, or its `;`.
fn item_end(lines: &[&str], start: usize) -> usize {
    let (mut depth, mut opened) = (0usize, false);
    for (index, line) in lines.iter().enumerate().skip(start) {
        let code = code(line);
        if code.trim_start().starts_with("#[") {
            continue;
        }
        for char in code.chars() {
            match char {
                '{' => (depth, opened) = (depth + 1, true),
                '}' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        if depth == 0 && (opened || code.trim_end().ends_with(';')) {
            return index + 1;
        }
    }
    lines.len()
}

/// The name in `mod name;`, with any visibility.
fn module(code: &str) -> Option<&str> {
    let code = code.trim();
    let code = match code.strip_prefix("pub") {
        Some(rest) if rest.starts_with('(') => rest.split_once(')')?.1.trim_start(),
        Some(rest) => rest.trim_start(),
        None => code,
    };
    code.strip_prefix("mod ")?.strip_suffix(';').map(str::trim)
}

fn literals(code: &str) -> Vec<Kind> {
    let mut kinds = Vec::new();
    let calls = [
        ("px(", Kind::Size, true),
        ("rgb(0x", Kind::Color, false),
        ("hsla(", Kind::Color, false),
        (".opacity(", Kind::Alpha, true),
        (".alpha(", Kind::Alpha, true),
    ];
    for (call, kind, numeric) in calls {
        for (at, _) in code.match_indices(call) {
            let starts_word = call.starts_with('.')
                || !code[..at]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_');
            if starts_word && (!numeric || number_then_paren(&code[at + call.len()..])) {
                kinds.push(kind);
            }
        }
    }
    kinds
}

/// Whether `rest` starts with a numeric literal, such as `-3.`, `0.5`, or `12f32`, then `)`.
fn number_then_paren(rest: &str) -> bool {
    let rest = rest
        .trim_start()
        .strip_prefix('-')
        .unwrap_or(rest.trim_start());
    if !rest.starts_with(|c: char| c.is_ascii_digit()) {
        return false;
    }
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '_'))
        .unwrap_or(rest.len());
    let rest = &rest[end..];
    let rest = rest
        .strip_prefix("f32")
        .or_else(|| rest.strip_prefix("f64"))
        .unwrap_or(rest);
    rest.trim_start().starts_with(')')
}

#[cfg(test)]
mod tests {
    use super::{Kind, code, item_end, literals, module};

    #[test]
    fn literal_calls_count_and_token_calls_do_not() {
        let kinds = literals(
            ".size(px(16.)).mt(px(-3.)).w(px(ICON)).bg(rgb(0xffffff)).c(hsla(0., 0., 0., 0.1)).opacity(0.5).alpha(INK)",
        );
        assert_eq!(
            kinds,
            [
                Kind::Size,
                Kind::Size,
                Kind::Color,
                Kind::Color,
                Kind::Alpha
            ]
        );
        assert!(literals("let x = size_px(3.);").is_empty());
        assert!(literals("px(HEADER + 2.)").is_empty());
    }

    #[test]
    fn comments_are_not_code_but_strings_keep_their_slashes() {
        assert_eq!(code("a(px(1.)) // px(2.)"), "a(px(1.)) ");
        assert_eq!(
            code(r#"let url = "https://x"; // note"#),
            r#"let url = "https://x"; "#
        );
    }

    #[test]
    fn test_items_end_at_their_closing_brace_or_semicolon() {
        let lines = ["mod tests {", "    fn a() {}", "}", "fn after() {}"];
        assert_eq!(item_end(&lines, 0), 3);
        assert_eq!(item_end(&["#[expect(x)]", "mod frames;", "x"], 0), 2);
    }

    #[test]
    fn module_declarations_name_their_file() {
        assert_eq!(module("pub(super) mod menu;"), Some("menu"));
        assert_eq!(module("mod frames;"), Some("frames"));
        assert_eq!(module("mod inline {"), None);
    }
}
