//! The workspace `clippy.toml`: the I/O APIs that only crates owning I/O may call.
//!
//! This list is the single source of truth. `cargo run -p xtask -- clippy-policy` writes
//! `clippy.toml` from it, and `xtask verify` fails when the file differs.
use anyhow::{Context, ensure};
use std::{fmt::Write as _, fs, path::Path};

const FS: &str = "filesystem work belongs in crates/engine";
const NET: &str = "network work belongs in crates/engine";
const PROCESS: &str = "process work belongs in crates/engine";
const ENV: &str = "environment access belongs in crates/engine or apps/desktop";
const TERMINAL: &str = "terminal I/O belongs in apps/desktop or xtask";
const EXIT: &str =
    "only apps/desktop may end the process; it drains the engine and releases the instance lock";

struct Rule {
    reason: &'static str,
    /// Clippy ignores a missing path instead of failing, for platform- or feature-gated items.
    allow_invalid: bool,
    types: &'static [&'static str],
    methods: &'static [&'static str],
}

const RULES: &[Rule] = &[
    Rule {
        reason: FS,
        allow_invalid: false,
        types: &[
            "std::fs::DirBuilder",
            "std::fs::DirEntry",
            "std::fs::File",
            "std::fs::OpenOptions",
            "std::fs::ReadDir",
        ],
        methods: &[
            "std::fs::canonicalize",
            "std::fs::copy",
            "std::fs::create_dir",
            "std::fs::create_dir_all",
            "std::fs::exists",
            "std::fs::hard_link",
            "std::fs::metadata",
            "std::fs::read",
            "std::fs::read_dir",
            "std::fs::read_link",
            "std::fs::read_to_string",
            "std::fs::remove_dir",
            "std::fs::remove_dir_all",
            "std::fs::remove_file",
            "std::fs::rename",
            "std::fs::set_permissions",
            "std::fs::symlink_metadata",
            "std::fs::write",
            "std::path::Path::canonicalize",
            "std::path::Path::exists",
            "std::path::Path::is_dir",
            "std::path::Path::is_file",
            "std::path::Path::is_symlink",
            "std::path::Path::metadata",
            "std::path::Path::read_dir",
            "std::path::Path::read_link",
            "std::path::Path::symlink_metadata",
            "std::path::Path::try_exists",
        ],
    },
    // Platform-specific or tokio feature-gated paths: absent on some builds.
    Rule {
        reason: FS,
        allow_invalid: true,
        types: &[
            "tokio::fs::DirBuilder",
            "tokio::fs::DirEntry",
            "tokio::fs::File",
            "tokio::fs::OpenOptions",
            "tokio::fs::ReadDir",
        ],
        methods: &[
            "std::os::unix::fs::chown",
            "std::os::unix::fs::chroot",
            "std::os::unix::fs::fchown",
            "std::os::unix::fs::lchown",
            "std::os::unix::fs::symlink",
            "std::os::windows::fs::symlink_dir",
            "std::os::windows::fs::symlink_file",
            "tokio::fs::canonicalize",
            "tokio::fs::copy",
            "tokio::fs::create_dir",
            "tokio::fs::create_dir_all",
            "tokio::fs::hard_link",
            "tokio::fs::metadata",
            "tokio::fs::read",
            "tokio::fs::read_dir",
            "tokio::fs::read_link",
            "tokio::fs::read_to_string",
            "tokio::fs::remove_dir",
            "tokio::fs::remove_dir_all",
            "tokio::fs::remove_file",
            "tokio::fs::rename",
            "tokio::fs::set_permissions",
            "tokio::fs::symlink",
            "tokio::fs::symlink_dir",
            "tokio::fs::symlink_file",
            "tokio::fs::symlink_metadata",
            "tokio::fs::try_exists",
            "tokio::fs::write",
        ],
    },
    Rule {
        reason: NET,
        allow_invalid: false,
        types: &[
            "std::net::TcpListener",
            "std::net::TcpStream",
            "std::net::UdpSocket",
        ],
        methods: &["std::net::ToSocketAddrs::to_socket_addrs"],
    },
    // Platform-specific or tokio feature-gated paths: absent on some builds.
    Rule {
        reason: NET,
        allow_invalid: true,
        types: &[
            "std::os::unix::net::UnixDatagram",
            "std::os::unix::net::UnixListener",
            "std::os::unix::net::UnixStream",
            "tokio::net::TcpListener",
            "tokio::net::TcpSocket",
            "tokio::net::TcpStream",
            "tokio::net::UdpSocket",
            "tokio::net::UnixDatagram",
            "tokio::net::UnixListener",
            "tokio::net::UnixSocket",
            "tokio::net::UnixStream",
            "tokio::net::windows::named_pipe::ClientOptions",
            "tokio::net::windows::named_pipe::ServerOptions",
        ],
        methods: &["tokio::net::lookup_host"],
    },
    Rule {
        reason: PROCESS,
        allow_invalid: false,
        types: &["std::process::Child", "std::process::Command"],
        methods: &[],
    },
    // Platform-specific or tokio feature-gated paths: absent on some builds.
    Rule {
        reason: PROCESS,
        allow_invalid: true,
        types: &["tokio::process::Child", "tokio::process::Command"],
        methods: &[],
    },
    Rule {
        reason: ENV,
        allow_invalid: false,
        types: &[],
        methods: &[
            "std::env::args",
            "std::env::args_os",
            "std::env::current_dir",
            "std::env::current_exe",
            "std::env::home_dir",
            "std::env::remove_var",
            "std::env::set_current_dir",
            "std::env::set_var",
            "std::env::temp_dir",
            "std::env::var",
            "std::env::var_os",
            "std::env::vars",
            "std::env::vars_os",
        ],
    },
    Rule {
        reason: TERMINAL,
        allow_invalid: false,
        types: &[],
        methods: &["std::io::stderr", "std::io::stdin", "std::io::stdout"],
    },
    // Platform-specific or tokio feature-gated paths: absent on some builds.
    Rule {
        reason: TERMINAL,
        allow_invalid: true,
        types: &[],
        methods: &["tokio::io::stderr", "tokio::io::stdin", "tokio::io::stdout"],
    },
    Rule {
        reason: EXIT,
        allow_invalid: false,
        types: &[],
        methods: &["std::process::abort", "std::process::exit"],
    },
];

const HEADER: &str = "\
# Generated by `cargo run -p xtask -- clippy-policy` from xtask/src/clippy_policy.rs; edit the
# list there. Blocking filesystem, process, network, environment, and terminal work belongs in
# crates/engine. crates/engine, apps/desktop, and xtask own I/O and replace this file with their
# own clippy.toml; everything else fails `cargo clippy -D warnings` when it reaches for these
# items. Clippy resolves names after expansion, so aliases, globs, `extern crate`, `include!`,
# and `#[path]` modules are all covered.
";

fn render() -> String {
    let mut out = HEADER.to_owned();
    section(&mut out, "disallowed-types", |rule| rule.types);
    section(&mut out, "disallowed-methods", |rule| rule.methods);
    out
}

fn section(out: &mut String, key: &str, items: fn(&Rule) -> &'static [&'static str]) {
    let _ = write!(out, "\n{key} = [\n");
    for rule in RULES {
        let invalid = if rule.allow_invalid {
            ", allow-invalid = true"
        } else {
            ""
        };
        for path in items(rule) {
            let reason = rule.reason;
            let _ = writeln!(
                out,
                "    {{ path = \"{path}\", reason = \"{reason}\"{invalid} }},"
            );
        }
    }
    out.push_str("]\n");
}

/// Writes the workspace `clippy.toml` from [`RULES`].
pub fn write(root: &Path) -> anyhow::Result<()> {
    let path = root.join("clippy.toml");
    fs::write(&path, render()).with_context(|| format!("Could not write {}", path.display()))
}

/// The workspace `clippy.toml` must match [`RULES`], and no `.clippy.toml` may shadow it.
pub fn verify(root: &Path) -> anyhow::Result<()> {
    let hidden = root.join(".clippy.toml");
    ensure!(
        !hidden.exists(),
        "{} would replace clippy.toml for every crate; remove it",
        hidden.display()
    );
    let path = root.join("clippy.toml");
    let actual = fs::read_to_string(&path)
        .with_context(|| format!("Missing workspace I/O policy {}", path.display()))?;
    ensure!(
        actual == render(),
        "{} differs from xtask/src/clippy_policy.rs; edit the list there and run `cargo run -p xtask -- clippy-policy`",
        path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "Test assertions report fixture failures."
    )]
    use super::{RULES, render};
    use std::collections::BTreeSet;

    #[test]
    fn rendered_policy_is_valid_toml_without_duplicates() {
        let table: toml::Table = render().parse().expect("valid TOML");
        let entries = ["disallowed-types", "disallowed-methods"]
            .iter()
            .flat_map(|key| table[*key].as_array().expect("array"))
            .count();
        let unique: BTreeSet<_> = RULES
            .iter()
            .flat_map(|rule| rule.types.iter().chain(rule.methods))
            .collect();
        assert_eq!(entries, unique.len());
    }
}
