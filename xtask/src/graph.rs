//! Workspace crate graph and source-boundary checks.
use crate::imports::{code_text, imports};
use crate::policy::{
    ENGINE, IO, MEMBERS, UI, allowed_dep_features, allowed_dep_source, allowed_dep_target,
    allowed_dev, allowed_features, allowed_normal,
};
use anyhow::{Context, ensure};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    id: String,
    dependencies: Vec<Dependency>,
    features: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    optional: bool,
    uses_default_features: bool,
    features: BTreeSet<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    rename: Option<String>,
    #[serde(default)]
    source: Option<String>,
}

pub fn verify(root: &Path) -> anyhow::Result<()> {
    let metadata = cargo_metadata(root)?;
    let packages = workspace_packages(&metadata)?;
    ensure!(
        packages.keys().cloned().collect::<BTreeSet<_>>()
            == MEMBERS.iter().copied().map(str::to_owned).collect(),
        "Workspace members changed; update xtask graph policy if the crate is intentional"
    );
    for (name, package) in &packages {
        check_dep_kinds(name, package)?;
        check_normal_deps(name, package)?;
        check_dev_deps(name, package)?;
        check_build_deps(name, package)?;
        check_features(name, package)?;
        check_dep_declarations(name, package)?;
        check_dep_features(name, package)?;
    }
    check_sources(root)?;
    Ok(())
}

fn cargo_metadata(root: &Path) -> anyhow::Result<Metadata> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--locked",
            "--manifest-path",
        ])
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

fn workspace_packages(metadata: &Metadata) -> anyhow::Result<BTreeMap<String, &Package>> {
    let ids: BTreeSet<&str> = metadata
        .workspace_members
        .iter()
        .map(String::as_str)
        .collect();
    let mut packages = BTreeMap::new();
    for package in &metadata.packages {
        if ids.contains(package.id.as_str()) {
            packages.insert(package.name.clone(), package);
        }
    }
    Ok(packages)
}

fn check_dep_kinds(name: &str, package: &Package) -> anyhow::Result<()> {
    for dep in &package.dependencies {
        ensure!(
            matches!(dep.kind.as_deref(), None | Some("dev") | Some("build")),
            "{name} has unsupported dependency kind {:?}; update xtask graph policy",
            dep.kind
        );
    }
    Ok(())
}

fn check_build_deps(name: &str, package: &Package) -> anyhow::Result<()> {
    let actual = named_deps(&package.dependencies, Some("build"));
    ensure!(
        actual.is_empty(),
        "{name} build-dependencies changed: expected none, found {actual:?}"
    );
    Ok(())
}

fn check_dep_declarations(name: &str, package: &Package) -> anyhow::Result<()> {
    let mut seen = BTreeSet::new();
    for dep in &package.dependencies {
        let kind = dep.kind.as_deref().unwrap_or("normal");
        ensure!(
            seen.insert((kind, dep.name.as_str())),
            "{name} declares {kind} dependency {} more than once",
            dep.name
        );
        ensure!(
            dep.rename.is_none(),
            "{name} renames dependency {}; import it under its own name",
            dep.name
        );
        let source = allowed_dep_source(&dep.name);
        ensure!(
            dep.source.as_deref() == source,
            "{name} dependency {} source changed: expected {source:?}, found {:?}",
            dep.name,
            dep.source
        );
    }
    Ok(())
}

fn check_dep_features(name: &str, package: &Package) -> anyhow::Result<()> {
    for dep in &package.dependencies {
        let kind = dep.kind.as_deref().unwrap_or("normal");
        let (default, features) = allowed_dep_features(name, kind, &dep.name);
        let features: BTreeSet<String> = features.iter().copied().map(str::to_owned).collect();
        ensure!(
            dep.uses_default_features == default && dep.features == features,
            "{name} {kind} dependency {} features changed: expected default={default} {features:?}, found default={} {:?}",
            dep.name,
            dep.uses_default_features,
            dep.features
        );
        let target = allowed_dep_target(name, &dep.name);
        ensure!(
            dep.target.as_deref() == target,
            "{name} {kind} dependency {} target changed: expected {target:?}, found {:?}",
            dep.name,
            dep.target
        );
    }
    Ok(())
}

fn check_features(name: &str, package: &Package) -> anyhow::Result<()> {
    let allowed = allowed_features(name);
    ensure!(
        package.features == allowed,
        "{name} features changed: expected {allowed:?}, found {:?}; native test controls must stay behind non-default native-test",
        package.features
    );
    Ok(())
}

fn check_normal_deps(name: &str, package: &Package) -> anyhow::Result<()> {
    let allowed = allowed_normal(name).context(format!("Unknown workspace package {name}"))?;
    let actual = named_deps(&package.dependencies, None);
    ensure!(
        actual == allowed,
        "{name} normal dependencies changed: expected {allowed:?}, found {actual:?}"
    );
    Ok(())
}

fn check_dev_deps(name: &str, package: &Package) -> anyhow::Result<()> {
    let allowed = allowed_dev(name);
    let actual = named_deps(&package.dependencies, Some("dev"));
    ensure!(
        actual == allowed,
        "{name} dev-dependencies changed: expected {allowed:?}, found {actual:?}"
    );
    Ok(())
}

fn named_deps(deps: &[Dependency], kind: Option<&str>) -> BTreeSet<(String, bool)> {
    deps.iter()
        .filter(|dep| dep.kind.as_deref() == kind)
        .map(|dep| (dep.name.clone(), dep.optional))
        .collect()
}

fn check_sources(root: &Path) -> anyhow::Result<()> {
    for (dir, forbidden) in [
        ("crates/proto/src", &[IO, ENGINE, UI][..]),
        ("crates/theme/src", &[IO, ENGINE]),
        ("crates/ui/src", &[IO, ENGINE]),
        ("crates/client/src", &[IO]),
        ("crates/engine/src", &[UI]),
    ] {
        for path in rust_files(&root.join(dir))? {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("Could not read {}", path.display()))?;
            deny_imports(&path, &text, forbidden)?;
        }
    }
    Ok(())
}

fn deny_imports(path: &Path, text: &str, forbidden: &[&[&str]]) -> anyhow::Result<()> {
    let source = code_text(text).with_context(|| format!("Could not check {}", path.display()))?;
    for needle in forbidden.iter().copied().flatten() {
        ensure!(
            !imports(&source, needle),
            "{} imports {needle}; keep that work in the owning crate",
            path.display()
        );
    }
    Ok(())
}

fn rust_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(&path).map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => anyhow::anyhow!("Missing {}", path.display()),
            _ => anyhow::Error::from(error),
        })? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "Test assertions report fixture failures."
    )]
    use std::path::Path;

    #[test]
    fn workspace_graph_matches_policy() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        super::verify(root).expect("workspace graph matches the recorded crate boundaries");
    }
}
