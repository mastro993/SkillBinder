//! Manifest and configuration checks that `cargo metadata --no-deps` cannot see.
use anyhow::{Context, ensure};
use std::{fs, path::Path};
use toml::{Table, Value};

/// Entries proving the workspace `clippy.toml` still carries the I/O policy.
const CLIPPY_POLICY: &[&str] = &[
    "std::env::var",
    "std::fs::File",
    "std::fs::write",
    "std::net::TcpStream",
    "std::path::Path::exists",
    "std::process::Command",
];

pub const GPUI_MCP_GIT: &str = "https://github.com/themixednuts/gpui-mcp";
pub const GPUI_MCP_REV: &str = "9dda8e5cb49990261e3fdaa26abe38112d30dafb";

fn read_table(path: &Path) -> anyhow::Result<Table> {
    let text =
        fs::read_to_string(path).with_context(|| format!("Could not read {}", path.display()))?;
    text.parse()
        .with_context(|| format!("Invalid TOML in {}", path.display()))
}

/// The workspace `clippy.toml` must still disallow the sentinel I/O items.
pub fn check_clippy_policy(root: &Path) -> anyhow::Result<()> {
    let path = root.join("clippy.toml");
    let config = read_table(&path)?;
    let active: Vec<&str> = ["disallowed-methods", "disallowed-types"]
        .into_iter()
        .filter_map(|key| config.get(key).and_then(Value::as_array))
        .flatten()
        .filter_map(|entry| match entry {
            Value::String(path) => Some(path.as_str()),
            Value::Table(entry) => entry.get("path").and_then(Value::as_str),
            _ => None,
        })
        .collect();
    for item in CLIPPY_POLICY {
        ensure!(
            active.contains(item),
            "{} no longer disallows {item}",
            path.display()
        );
    }
    Ok(())
}

/// Every member inherits `[workspace.lints]`, which forbids unsafe code and suppressing the I/O
/// policy.
pub fn check_workspace_lints(manifest_path: &Path) -> anyhow::Result<()> {
    let manifest = read_table(manifest_path)?;
    let inherits = manifest
        .get("lints")
        .and_then(|lints| lints.get("workspace"))
        .and_then(Value::as_bool);
    ensure!(
        inherits == Some(true),
        "{} must declare `[lints] workspace = true`",
        manifest_path.display()
    );
    Ok(())
}

/// The only dependency override is the `gpui-pre` backend from the pinned gpui-mcp revision, and
/// Cargo configuration does not add patches or replace sources.
pub fn check_overrides(root: &Path) -> anyhow::Result<()> {
    let manifest = read_table(&root.join("Cargo.toml"))?;
    let expected: Table = toml::toml! {
        [crates-io.gpui-pre]
        git = GPUI_MCP_GIT
        rev = GPUI_MCP_REV
    };
    ensure!(
        manifest.get("patch").and_then(Value::as_table) == Some(&expected),
        "Cargo.toml [patch] changed; only gpui-pre from gpui-mcp {GPUI_MCP_REV} is allowed"
    );
    ensure!(
        !manifest.contains_key("replace"),
        "Cargo.toml must not use [replace]"
    );
    for name in ["config.toml", "config"] {
        let path = root.join(".cargo").join(name);
        if path.is_file() {
            let config = read_table(&path)?;
            for key in ["patch", "source", "paths"] {
                ensure!(
                    !config.contains_key(key),
                    "{} must not override dependencies with [{key}]",
                    path.display()
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "Test assertions report fixture failures."
    )]
    use super::{
        GPUI_MCP_GIT, GPUI_MCP_REV, check_clippy_policy, check_overrides, check_workspace_lints,
    };
    use std::fs;

    #[test]
    fn commented_policy_entries_do_not_count() {
        let dir = tempfile::tempdir().expect("temp dir");
        let entries = super::CLIPPY_POLICY
            .iter()
            .map(|item| format!("    {{ path = \"{item}\" }},\n"))
            .collect::<String>();
        let active = format!("disallowed-methods = [\n{entries}]\n");
        fs::write(dir.path().join("clippy.toml"), &active).expect("write");
        assert!(check_clippy_policy(dir.path()).is_ok());
        let commented = active.replacen(
            "    { path = \"std::fs::write\" }",
            "    # { path = \"std::fs::write\" }",
            1,
        );
        fs::write(dir.path().join("clippy.toml"), commented).expect("write");
        assert!(check_clippy_policy(dir.path()).is_err());
    }

    #[test]
    fn lint_inheritance_ignores_formatting() {
        let dir = tempfile::tempdir().expect("temp dir");
        let manifest = dir.path().join("Cargo.toml");
        fs::write(&manifest, "[lints]\n# inherited\nworkspace=true\n").expect("write");
        assert!(check_workspace_lints(&manifest).is_ok());
        fs::write(&manifest, "[lints.rust]\nunsafe_code = \"allow\"\n").expect("write");
        assert!(check_workspace_lints(&manifest).is_err());
    }

    #[test]
    fn only_the_pinned_backend_patch_is_allowed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let manifest = |rev: &str| {
            format!(
                "[patch.crates-io]\ngpui-pre = {{ git = \"{GPUI_MCP_GIT}\", rev = \"{rev}\" }}\n"
            )
        };
        fs::write(dir.path().join("Cargo.toml"), manifest(GPUI_MCP_REV)).expect("write");
        assert!(check_overrides(dir.path()).is_ok());
        fs::write(dir.path().join("Cargo.toml"), manifest("0000000")).expect("write");
        assert!(check_overrides(dir.path()).is_err());
        fs::write(dir.path().join("Cargo.toml"), manifest(GPUI_MCP_REV)).expect("write");
        fs::create_dir(dir.path().join(".cargo")).expect("mkdir");
        fs::write(dir.path().join(".cargo/config.toml"), "[patch.crates-io]\n").expect("write");
        assert!(check_overrides(dir.path()).is_err());
    }
}
