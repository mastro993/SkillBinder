//! Recorded crate-graph policy: members, dependencies, features, and forbidden source paths.
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MEMBERS: [&str; 7] = [
    "skillbinder",
    "skillbinder-client",
    "skillbinder-engine",
    "skillbinder-proto",
    "skillbinder-theme",
    "skillbinder-ui",
    "xtask",
];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Dep {
    pub(crate) name: &'static str,
    pub(crate) optional: bool,
}

/// Dependencies whose feature selection differs from plain default features.
pub(crate) const DEP_FEATURES: &[(&str, &str, &str, bool, &[&str])] = &[
    ("skillbinder", "normal", "gpui-mcp", false, &["gpui-pre"]),
    ("skillbinder", "normal", "tokio", true, TOKIO),
    ("skillbinder-client", "normal", "tokio", true, TOKIO),
    ("skillbinder-engine", "normal", "diesel", false, &["sqlite"]),
    (
        "skillbinder-engine",
        "normal",
        "diesel_migrations",
        true,
        &["sqlite"],
    ),
    (
        "skillbinder-engine",
        "normal",
        "libsqlite3-sys",
        true,
        &["bundled"],
    ),
    ("skillbinder-engine", "normal", "serde", true, &["derive"]),
    (
        "skillbinder-engine",
        "normal",
        "time",
        true,
        &["formatting"],
    ),
    ("skillbinder-engine", "normal", "tokio", true, TOKIO),
    (
        "skillbinder-engine",
        "normal",
        "uuid",
        true,
        &["v4", "serde"],
    ),
    (
        "skillbinder-engine",
        "normal",
        "rustix",
        true,
        &["fs", "process"],
    ),
    ("skillbinder-proto", "normal", "serde", true, &["derive"]),
    (
        "skillbinder-proto",
        "normal",
        "uuid",
        true,
        &["v4", "serde"],
    ),
    ("skillbinder-ui", "dev", "gpui-kit", true, &["test-support"]),
    ("xtask", "normal", "serde", true, &["derive"]),
];
pub(crate) const TOKIO: &[&str] = &["rt-multi-thread", "sync", "time", "macros"];

/// Platform predicates of target-specific dependencies; all others are unconditional.
pub(crate) fn allowed_dep_target(package: &str, dep: &str) -> Option<&'static str> {
    match (package, dep) {
        ("skillbinder-engine", "rustix") => Some("cfg(unix)"),
        _ => None,
    }
}

pub(crate) fn allowed_dep_features(
    package: &str,
    kind: &str,
    dep: &str,
) -> (bool, &'static [&'static str]) {
    DEP_FEATURES
        .iter()
        .find(|(p, k, d, _, _)| *p == package && *k == kind && *d == dep)
        .map_or((true, &[]), |(_, _, _, default, features)| {
            (*default, *features)
        })
}

pub(crate) fn allowed_features(name: &str) -> BTreeMap<String, BTreeSet<String>> {
    let enabled: &[&str] = match name {
        "skillbinder" => &["skillbinder-ui/native-test", "dep:log", "dep:gpui-mcp"],
        "skillbinder-ui" => &[],
        _ => return BTreeMap::new(),
    };
    BTreeMap::from([(
        "native-test".to_owned(),
        enabled.iter().copied().map(str::to_owned).collect(),
    )])
}

pub(crate) fn allowed_normal(name: &str) -> Option<BTreeSet<(String, bool)>> {
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
                    name: "proc-macro2",
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

pub(crate) fn allowed_dev(name: &str) -> BTreeSet<(String, bool)> {
    match name {
        "skillbinder-client" => BTreeSet::from([("tempfile".into(), false)]),
        "skillbinder-ui" => BTreeSet::from([("gpui-kit".into(), false)]),
        _ => BTreeSet::new(),
    }
}

pub(crate) const IO: &[&str] = &[
    "std::env",
    "std::fs",
    "std::io",
    "std::net",
    "std::os",
    "std::process",
    "rusqlite",
    "diesel",
    "diesel_migrations",
    "libsqlite3_sys",
];
pub(crate) const ENGINE: &[&str] = &["skillbinder_engine"];
pub(crate) const UI: &[&str] = &["gpui", "gpui_kit", "skillbinder_ui", "skillbinder_client"];

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "Test assertions report fixture failures."
    )]
    use super::{allowed_dep_features, allowed_features, allowed_normal};

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
    fn gpui_mcp_is_optional_on_the_executable() {
        let deps = allowed_normal("skillbinder").unwrap();
        assert!(deps.contains(&("gpui-mcp".into(), true)));
        assert!(deps.contains(&("log".into(), true)));
    }

    #[test]
    fn only_native_test_enables_the_test_bridge() {
        let features = allowed_features("skillbinder");
        assert!(!features.contains_key("default"));
        assert!(features["native-test"].contains("dep:gpui-mcp"));
        assert!(!allowed_features("skillbinder-ui").contains_key("default"));
    }

    #[test]
    fn client_tokio_has_no_io_features() {
        let (default, features) = allowed_dep_features("skillbinder-client", "normal", "tokio");
        assert!(default);
        for io in ["fs", "net", "process", "full"] {
            assert!(!features.contains(&io), "{io}");
        }
    }
}
