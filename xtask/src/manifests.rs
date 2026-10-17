//! Manifest and configuration checks that `cargo metadata --no-deps` cannot see.
use anyhow::{Context, ensure};
use std::{fs, path::Path};
use toml::{Table, Value};

pub const GPUI_MCP_GIT: &str = "https://github.com/themixednuts/gpui-mcp";
pub const GPUI_MCP_REV: &str = "9dda8e5cb49990261e3fdaa26abe38112d30dafb";

fn read_table(path: &Path) -> anyhow::Result<Table> {
    let text =
        fs::read_to_string(path).with_context(|| format!("Could not read {}", path.display()))?;
    text.parse()
        .with_context(|| format!("Invalid TOML in {}", path.display()))
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

/// The workspace lints forbid unsafe code and allowing the I/O policy's disallowed APIs back.
pub fn check_workspace_lint_levels(root: &Path) -> anyhow::Result<()> {
    let manifest = read_table(&root.join("Cargo.toml"))?;
    let lints = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("lints"));
    for (group, lint) in [
        ("rust", "unsafe_code"),
        ("clippy", "disallowed_methods"),
        ("clippy", "disallowed_types"),
    ] {
        let level = lints
            .and_then(|lints| lints.get(group))
            .and_then(|group| group.get(lint));
        let level = level
            .and_then(Value::as_str)
            .or_else(|| level?.get("level")?.as_str());
        ensure!(
            level == Some("forbid"),
            "Cargo.toml [workspace.lints.{group}] must set {lint} = \"forbid\", found {level:?}"
        );
    }
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
            check_cargo_config(&path, &read_table(&path)?)?;
        }
    }
    Ok(())
}

/// Repository Cargo configuration may not override dependencies, pass compiler flags that cap or
/// allow the boundary lints, wrap the compiler, alias the `clippy` or `fmt` subcommands, replace
/// the test runner, or point Clippy at another configuration.
fn check_cargo_config(path: &Path, config: &Table) -> anyhow::Result<()> {
    let build = config.get("build").and_then(Value::as_table);
    let targets = config.get("target").and_then(Value::as_table);
    let env = config.get("env").and_then(Value::as_table);
    let rejected = ["alias", "patch", "source", "paths"]
        .into_iter()
        .filter(|key| config.contains_key(*key))
        .map(|key| format!("[{key}]"))
        .chain(build.into_iter().flat_map(|build| {
            ["rustflags", "rustc-wrapper", "rustc-workspace-wrapper"]
                .into_iter()
                .filter(|key| build.contains_key(*key))
                .map(|key| format!("build.{key}"))
        }))
        .chain(targets.into_iter().flatten().flat_map(|(target, table)| {
            ["rustflags", "runner"]
                .into_iter()
                .filter(|key| table.get(*key).is_some())
                .map(move |key| format!("target.{target}.{key}"))
        }))
        .chain(env.into_iter().flat_map(Table::keys).filter_map(|key| {
            let upper = key.to_ascii_uppercase();
            (upper.contains("CLIPPY") || upper.contains("RUSTFLAGS")).then(|| format!("env.{key}"))
        }))
        .collect::<Vec<_>>();
    ensure!(
        rejected.is_empty(),
        "{} must not set {}; it would bypass dependency or lint policy",
        path.display(),
        rejected.join(", ")
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "Test assertions report fixture failures."
    )]
    use super::{
        GPUI_MCP_GIT, GPUI_MCP_REV, check_overrides, check_workspace_lint_levels,
        check_workspace_lints,
    };
    use std::fs;

    #[test]
    fn workspace_lints_must_forbid_the_policy_lints() {
        let dir = tempfile::tempdir().expect("temp dir");
        let manifest = |methods: &str| {
            format!(
                "[workspace.lints.rust]\nunsafe_code = \"forbid\"\n[workspace.lints.clippy]\ndisallowed_methods = {methods}\ndisallowed_types = \"forbid\"\n"
            )
        };
        fs::write(dir.path().join("Cargo.toml"), manifest("\"forbid\"")).expect("write");
        assert!(check_workspace_lint_levels(dir.path()).is_ok());
        fs::write(
            dir.path().join("Cargo.toml"),
            manifest("{ level = \"forbid\" }"),
        )
        .expect("write");
        assert!(check_workspace_lint_levels(dir.path()).is_ok());
        fs::write(dir.path().join("Cargo.toml"), manifest("\"allow\"")).expect("write");
        assert!(check_workspace_lint_levels(dir.path()).is_err());
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
        for config in [
            "[patch.crates-io]\n",
            "[build]\nrustflags = [\"--cap-lints\", \"allow\"]\n",
            "[target.'cfg(unix)']\nrustflags = [\"-A\", \"clippy::disallowed_methods\"]\n",
            "[env]\nCLIPPY_CONF_DIR = \"elsewhere\"\n",
            "[alias]\nclippy = \"check\"\n",
            "[target.x86_64-unknown-linux-gnu]\nrunner = \"true\"\n",
        ] {
            fs::write(dir.path().join(".cargo/config.toml"), config).expect("write");
            assert!(check_overrides(dir.path()).is_err(), "{config}");
        }
        let allowed = "[env]\nMACOSX_DEPLOYMENT_TARGET = \"26.0\"\n";
        fs::write(dir.path().join(".cargo/config.toml"), allowed).expect("write");
        assert!(check_overrides(dir.path()).is_ok());
    }
}
