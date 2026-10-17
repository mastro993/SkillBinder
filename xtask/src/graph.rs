//! Workspace crate graph: members, dependencies, features, and Clippy I/O policy scope.
use crate::clippy_policy;
use crate::manifests::{
    GPUI_MCP_GIT, GPUI_MCP_REV, check_overrides, check_workspace_lint_levels, check_workspace_lints,
};
use anyhow::{Context, ensure};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Command,
};

/// Allowed normal and dev dependencies per workspace member. Build dependencies are not allowed.
const POLICY: &[(&str, &[&str], &[&str])] = &[
    (
        "skillbinder",
        &[
            "anyhow",
            "fs2",
            "gpui-kit",
            "gpui-mcp",
            "log",
            "skillbinder-client",
            "skillbinder-engine",
            "skillbinder-proto",
            "skillbinder-theme",
            "skillbinder-ui",
            "tokio",
        ],
        &[],
    ),
    (
        "skillbinder-client",
        &["skillbinder-engine", "skillbinder-proto", "tokio"],
        &["tempfile"],
    ),
    (
        "skillbinder-engine",
        &[
            "diesel",
            "diesel_migrations",
            "directories",
            "fs2",
            "libsqlite3-sys",
            "regex",
            "rustix",
            "serde",
            "serde_json",
            "sha2",
            "skillbinder-proto",
            "tempfile",
            "time",
            "tokio",
            "unicode-casefold",
            "unicode-normalization",
            "url",
            "uuid",
            "yaml-rust2",
        ],
        &[],
    ),
    ("skillbinder-proto", &["serde", "serde_json", "uuid"], &[]),
    ("skillbinder-theme", &["gpui-kit"], &[]),
    (
        "skillbinder-ui",
        &[
            "anyhow",
            "chrono",
            "gpui-kit",
            "skillbinder-client",
            "skillbinder-proto",
            "skillbinder-theme",
        ],
        &["gpui-kit"],
    ),
    (
        "xtask",
        &["anyhow", "serde", "serde_json", "sha2", "tempfile", "toml"],
        &[],
    ),
];

/// The native test bridge is the only optional code, and only `native-test` enables it.
const NATIVE_TEST: &[(&str, &[&str])] = &[
    (
        "skillbinder",
        &["dep:gpui-mcp", "dep:log", "skillbinder-ui/native-test"],
    ),
    ("skillbinder-ui", &[]),
];

/// Members that own I/O and may replace the workspace `clippy.toml` with their own.
const OWNS_IO: &[&str] = &["skillbinder", "skillbinder-engine", "xtask"];

const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
    workspace_root: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    id: String,
    manifest_path: PathBuf,
    dependencies: Vec<Dependency>,
    features: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
    kind: Option<String>,
    optional: bool,
    source: Option<String>,
    path: Option<PathBuf>,
}

pub fn verify(root: &Path) -> anyhow::Result<()> {
    let metadata = cargo_metadata(root)?;
    let members = workspace_packages(&metadata);
    let names: BTreeSet<&str> = members
        .iter()
        .map(|package| package.name.as_str())
        .collect();
    let expected: BTreeSet<&str> = POLICY.iter().map(|(name, _, _)| *name).collect();
    ensure!(
        names == expected,
        "Workspace members changed: expected {expected:?}, found {names:?}; update xtask graph policy if the crate is intentional"
    );
    let dirs: BTreeMap<&str, &Path> = members
        .iter()
        .filter_map(|package| Some((package.name.as_str(), package.manifest_path.parent()?)))
        .collect();
    clippy_policy::verify(root)?;
    check_overrides(root)?;
    check_workspace_lint_levels(root)?;
    for package in &members {
        check_dependencies(package, &dirs)?;
        check_features(package)?;
        check_workspace_lints(&package.manifest_path)?;
        check_clippy_scope(&metadata.workspace_root, package)?;
    }
    Ok(())
}

fn cargo_metadata(root: &Path) -> anyhow::Result<Metadata> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .output()
        .context("Could not run cargo metadata")?;
    ensure!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).context("Invalid cargo metadata")
}

fn workspace_packages(metadata: &Metadata) -> Vec<&Package> {
    metadata
        .packages
        .iter()
        .filter(|package| metadata.workspace_members.contains(&package.id))
        .collect()
}

fn check_dependencies(package: &Package, members: &BTreeMap<&str, &Path>) -> anyhow::Result<()> {
    let name = package.name.as_str();
    let (_, normal, dev) = POLICY
        .iter()
        .find(|(member, _, _)| *member == name)
        .context(format!("Unknown workspace package {name}"))?;
    for (kind, allowed) in [
        (None, *normal),
        (Some("dev"), *dev),
        (Some("build"), &[][..]),
    ] {
        let allowed: BTreeSet<&str> = allowed.iter().copied().collect();
        let actual: BTreeSet<&str> = package
            .dependencies
            .iter()
            .filter(|dep| dep.kind.as_deref() == kind)
            .map(|dep| dep.name.as_str())
            .collect();
        ensure!(
            actual == allowed,
            "{name} {} dependencies changed: expected {allowed:?}, found {actual:?}",
            kind.unwrap_or("normal")
        );
    }
    for dep in &package.dependencies {
        ensure!(
            matches!(dep.kind.as_deref(), None | Some("dev" | "build")),
            "{name} has unsupported dependency kind {:?}",
            dep.kind
        );
        let enabler = format!("dep:{}", dep.name);
        let optional = NATIVE_TEST
            .iter()
            .any(|(member, enables)| *member == name && enables.contains(&enabler.as_str()));
        ensure!(
            dep.optional == optional,
            "{name} dependency {} must{} be optional",
            dep.name,
            if optional { "" } else { " not" }
        );
        check_origin(name, dep, members)?;
    }
    Ok(())
}

/// Workspace crates must resolve to their member directories, so Clippy lints them; others come
/// from crates.io or the pinned gpui-mcp revision.
fn check_origin(
    name: &str,
    dep: &Dependency,
    members: &BTreeMap<&str, &Path>,
) -> anyhow::Result<()> {
    if let Some(dir) = members.get(dep.name.as_str()) {
        ensure!(
            dep.path.as_deref() == Some(*dir) && dep.source.is_none(),
            "{name} dependency {} must be the workspace member at {}",
            dep.name,
            dir.display()
        );
        return Ok(());
    }
    let expected = if dep.name == "gpui-mcp" {
        format!("git+{GPUI_MCP_GIT}?rev={GPUI_MCP_REV}")
    } else {
        CRATES_IO.to_owned()
    };
    ensure!(
        dep.path.is_none() && dep.source.as_deref() == Some(expected.as_str()),
        "{name} dependency {} must come from {expected}, found path {:?} source {:?}",
        dep.name,
        dep.path,
        dep.source
    );
    Ok(())
}

fn check_features(package: &Package) -> anyhow::Result<()> {
    let expected: BTreeMap<String, BTreeSet<String>> = NATIVE_TEST
        .iter()
        .filter(|(member, _)| *member == package.name)
        .map(|(_, enables)| {
            let enables = enables.iter().copied().map(str::to_owned).collect();
            ("native-test".to_owned(), enables)
        })
        .collect();
    ensure!(
        package.features == expected,
        "{} features changed: expected {expected:?}, found {:?}; native test controls must stay behind non-default native-test",
        package.name,
        package.features
    );
    Ok(())
}

/// Clippy reads the nearest `clippy.toml` at or above a crate, so only crates that own I/O may
/// shadow the workspace policy, and only from their own directory.
fn check_clippy_scope(root: &Path, package: &Package) -> anyhow::Result<()> {
    let manifest_dir = package
        .manifest_path
        .parent()
        .context("Manifest has no directory")?;
    let relative = manifest_dir.strip_prefix(root).with_context(|| {
        format!(
            "{} is outside the workspace root {}",
            manifest_dir.display(),
            root.display()
        )
    })?;
    let owns_io = OWNS_IO.contains(&package.name.as_str());
    // The root's own clippy.toml is the policy itself; clippy_policy checks it and `.clippy.toml`.
    let in_tree = relative
        .ancestors()
        .filter(|dir| !dir.as_os_str().is_empty());
    for dir in in_tree.map(|dir| root.join(dir)) {
        for file in ["clippy.toml", ".clippy.toml"] {
            let config = dir.join(file);
            ensure!(
                !config.is_file() || (owns_io && dir == manifest_dir && file == "clippy.toml"),
                "{} shadows the workspace clippy.toml for {}; only {OWNS_IO:?} may own I/O",
                config.display(),
                package.name
            );
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
    use super::{OWNS_IO, POLICY};
    use std::path::Path;

    fn normal(name: &str) -> &'static [&'static str] {
        POLICY
            .iter()
            .find(|(member, _, _)| *member == name)
            .map(|(_, normal, _)| *normal)
            .expect("member is in policy")
    }

    #[test]
    fn only_the_engine_reaches_sqlite() {
        for (member, deps, _) in POLICY {
            let sqlite = deps
                .iter()
                .any(|dep| dep.starts_with("diesel") || *dep == "libsqlite3-sys");
            assert_eq!(sqlite, *member == "skillbinder-engine", "{member}");
        }
    }

    #[test]
    fn contracts_and_ui_stay_off_the_engine() {
        let proto = normal("skillbinder-proto");
        assert!(
            !proto
                .iter()
                .any(|dep| dep.starts_with("skillbinder") || dep.starts_with("gpui"))
        );
        assert!(!normal("skillbinder-ui").contains(&"skillbinder-engine"));
        assert!(!normal("skillbinder-theme").contains(&"skillbinder-engine"));
        assert!(!OWNS_IO.contains(&"skillbinder-ui"));
    }

    #[test]
    fn workspace_graph_matches_policy() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root");
        super::verify(root).expect("workspace graph matches the recorded crate boundaries");
    }
}
