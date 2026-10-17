//! Workspace crate graph and source-boundary checks.
use crate::imports::imports;
use anyhow::{Context, ensure};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

const MEMBERS: [&str; 7] = [
    "skillbinder",
    "skillbinder-client",
    "skillbinder-engine",
    "skillbinder-proto",
    "skillbinder-theme",
    "skillbinder-ui",
    "xtask",
];

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
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    optional: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Dep {
    name: &'static str,
    optional: bool,
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
    }
    let skillbinder = packages
        .get("skillbinder")
        .context("Missing skillbinder package")?;
    ensure!(
        skillbinder
            .dependencies
            .iter()
            .any(|dep| dep.name == "gpui-mcp" && dep.optional),
        "gpui-mcp must remain an optional skillbinder dependency"
    );
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

fn allowed_normal(name: &str) -> Option<BTreeSet<(String, bool)>> {
    const TABLE: &[(&str, &[Dep])] = &[
        (
            "skillbinder-proto",
            &[
                Dep {
                    name: "serde",
                    optional: false,
                },
                Dep {
                    name: "serde_json",
                    optional: false,
                },
                Dep {
                    name: "uuid",
                    optional: false,
                },
            ],
        ),
        (
            "skillbinder-theme",
            &[Dep {
                name: "gpui-kit",
                optional: false,
            }],
        ),
        (
            "skillbinder-engine",
            &[
                Dep {
                    name: "directories",
                    optional: false,
                },
                Dep {
                    name: "fs2",
                    optional: false,
                },
                Dep {
                    name: "diesel",
                    optional: false,
                },
                Dep {
                    name: "diesel_migrations",
                    optional: false,
                },
                Dep {
                    name: "libsqlite3-sys",
                    optional: false,
                },
                Dep {
                    name: "regex",
                    optional: false,
                },
                Dep {
                    name: "rustix",
                    optional: false,
                },
                Dep {
                    name: "serde",
                    optional: false,
                },
                Dep {
                    name: "serde_json",
                    optional: false,
                },
                Dep {
                    name: "sha2",
                    optional: false,
                },
                Dep {
                    name: "skillbinder-proto",
                    optional: false,
                },
                Dep {
                    name: "tempfile",
                    optional: false,
                },
                Dep {
                    name: "time",
                    optional: false,
                },
                Dep {
                    name: "tokio",
                    optional: false,
                },
                Dep {
                    name: "unicode-casefold",
                    optional: false,
                },
                Dep {
                    name: "unicode-normalization",
                    optional: false,
                },
                Dep {
                    name: "url",
                    optional: false,
                },
                Dep {
                    name: "uuid",
                    optional: false,
                },
                Dep {
                    name: "yaml-rust2",
                    optional: false,
                },
            ],
        ),
        (
            "skillbinder-client",
            &[
                Dep {
                    name: "skillbinder-engine",
                    optional: false,
                },
                Dep {
                    name: "skillbinder-proto",
                    optional: false,
                },
                Dep {
                    name: "tokio",
                    optional: false,
                },
            ],
        ),
        (
            "skillbinder-ui",
            &[
                Dep {
                    name: "anyhow",
                    optional: false,
                },
                Dep {
                    name: "chrono",
                    optional: false,
                },
                Dep {
                    name: "gpui-kit",
                    optional: false,
                },
                Dep {
                    name: "skillbinder-client",
                    optional: false,
                },
                Dep {
                    name: "skillbinder-proto",
                    optional: false,
                },
                Dep {
                    name: "skillbinder-theme",
                    optional: false,
                },
            ],
        ),
        (
            "skillbinder",
            &[
                Dep {
                    name: "anyhow",
                    optional: false,
                },
                Dep {
                    name: "fs2",
                    optional: false,
                },
                Dep {
                    name: "gpui-kit",
                    optional: false,
                },
                Dep {
                    name: "gpui-mcp",
                    optional: true,
                },
                Dep {
                    name: "log",
                    optional: true,
                },
                Dep {
                    name: "skillbinder-client",
                    optional: false,
                },
                Dep {
                    name: "skillbinder-engine",
                    optional: false,
                },
                Dep {
                    name: "skillbinder-proto",
                    optional: false,
                },
                Dep {
                    name: "skillbinder-theme",
                    optional: false,
                },
                Dep {
                    name: "skillbinder-ui",
                    optional: false,
                },
                Dep {
                    name: "tokio",
                    optional: false,
                },
            ],
        ),
        (
            "xtask",
            &[
                Dep {
                    name: "anyhow",
                    optional: false,
                },
                Dep {
                    name: "serde",
                    optional: false,
                },
                Dep {
                    name: "serde_json",
                    optional: false,
                },
                Dep {
                    name: "sha2",
                    optional: false,
                },
                Dep {
                    name: "tempfile",
                    optional: false,
                },
            ],
        ),
    ];
    TABLE
        .iter()
        .find(|(package, _)| *package == name)
        .map(|(_, deps)| {
            deps.iter()
                .map(|dep| (dep.name.to_string(), dep.optional))
                .collect()
        })
}

fn allowed_dev(name: &str) -> BTreeSet<(String, bool)> {
    match name {
        "skillbinder-client" => BTreeSet::from([("tempfile".into(), false)]),
        "skillbinder-ui" => BTreeSet::from([("gpui-kit".into(), false)]),
        _ => BTreeSet::new(),
    }
}

fn check_sources(root: &Path) -> anyhow::Result<()> {
    for (dir, forbidden) in [
        ("crates/proto/src", IO_AND_UI),
        ("crates/theme/src", IO_AND_ENGINE),
        ("crates/ui/src", IO_AND_ENGINE),
        ("crates/client/src", IO_ONLY),
        ("crates/engine/src", UI_ONLY),
    ] {
        for path in rust_files(&root.join(dir))? {
            let text = fs::read_to_string(&path)
                .with_context(|| format!("Could not read {}", path.display()))?;
            deny_imports(&path, &text, forbidden)?;
        }
    }
    Ok(())
}

const IO_ONLY: &[&str] = &[
    "std::fs",
    "std::process",
    "rusqlite",
    "diesel",
    "diesel_migrations",
    "libsqlite3_sys",
];
const IO_AND_ENGINE: &[&str] = &[
    "std::fs",
    "std::process",
    "rusqlite",
    "diesel",
    "diesel_migrations",
    "libsqlite3_sys",
    "skillbinder_engine",
];
const IO_AND_UI: &[&str] = &[
    "std::fs",
    "std::process",
    "rusqlite",
    "diesel",
    "diesel_migrations",
    "libsqlite3_sys",
    "gpui",
    "gpui_kit",
    "skillbinder_engine",
    "skillbinder_client",
    "skillbinder_ui",
];
const UI_ONLY: &[&str] = &["gpui", "gpui_kit", "skillbinder_ui", "skillbinder_client"];

fn deny_imports(path: &Path, text: &str, forbidden: &[&str]) -> anyhow::Result<()> {
    for needle in forbidden {
        ensure!(
            !imports(text, needle),
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
    use super::allowed_normal;
    use crate::imports::imports;
    use std::path::Path;

    #[test]
    fn ui_may_not_depend_on_the_engine() {
        let deps = allowed_normal("skillbinder-ui").unwrap();
        assert!(!deps.contains(&("skillbinder-engine".into(), false)));
        assert!(!deps.contains(&("rusqlite".into(), false)));
        assert!(!deps.contains(&("diesel".into(), false)));
    }

    #[test]
    fn engine_owns_sqlite_through_diesel() {
        let deps = allowed_normal("skillbinder-engine").unwrap();
        assert!(deps.contains(&("diesel".into(), false)));
        assert!(deps.contains(&("diesel_migrations".into(), false)));
        assert!(!deps.contains(&("rusqlite".into(), false)));
    }

    #[test]
    fn proto_stays_free_of_io_and_gpui() {
        let deps = allowed_normal("skillbinder-proto").unwrap();
        for banned in [
            "rusqlite",
            "diesel",
            "gpui-kit",
            "skillbinder-engine",
            "std",
        ] {
            assert!(!deps.iter().any(|(name, _)| name == banned), "{banned}");
        }
    }

    #[test]
    fn comments_do_not_count_as_imports() {
        assert!(!imports("// use std::fs::read;\nfn f() {}", "std::fs"));
        assert!(imports("use std::fs::File;\nfn f() {}", "std::fs"));
        assert!(imports("let _ = std::fs::read(path);", "std::fs"));
        assert!(imports("use std::{fs, path::Path};", "std::fs"));
    }

    #[test]
    fn gpui_mcp_is_optional_on_the_executable() {
        let deps = allowed_normal("skillbinder").unwrap();
        assert!(deps.contains(&("gpui-mcp".into(), true)));
        assert!(deps.contains(&("log".into(), true)));
    }

    #[test]
    fn workspace_graph_matches_policy() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        super::verify(root).expect("workspace graph matches the recorded crate boundaries");
    }
}
