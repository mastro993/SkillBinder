//! The app version: `[workspace.package] version` in the root manifest, which every member
//! inherits. The release workflows read, bump, and stamp it here (docs/release.md).
//!
//! - `version` prints the current version.
//! - `version next <major|minor|patch|X.Y.Z>` prints the release after it.
//! - `version set <version>` writes it to `Cargo.toml` and the members' `Cargo.lock` entries.
use anyhow::{Context, bail, ensure};
use std::{cmp::Ordering, fmt, fs, path::Path, str::FromStr};
use toml::{Table, Value};

pub(crate) fn run(root: &Path, args: &[String]) -> anyhow::Result<()> {
    match args {
        [] => println!("{}", read(root)?),
        [command, part] if command == "next" => println!("{}", next(&read(root)?, part)?),
        [command, version] if command == "set" => {
            set(root, &version.parse()?)?;
            println!("{version}");
        }
        _ => bail!(
            "Usage: cargo run -p xtask -- version [next <major|minor|patch|X.Y.Z> | set <version>]"
        ),
    }
    Ok(())
}

/// A SemVer version without build metadata, which Cargo and release tags both accept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Version {
    pub(crate) core: [u64; 3],
    pre: Option<String>,
}

impl FromStr for Version {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> anyhow::Result<Self> {
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };
        let numbers: Vec<&str> = core.split('.').collect();
        let [major, minor, patch] = numbers[..] else {
            bail!("{text} is not a version: expected X.Y.Z");
        };
        let number = |part: &str| -> anyhow::Result<u64> {
            // SemVer cores have no leading zeros; Cargo rejects `01.0.0`.
            ensure!(
                !part.is_empty()
                    && part.bytes().all(|byte| byte.is_ascii_digit())
                    && (part == "0" || !part.starts_with('0')),
                "{text} is not a version: {part:?} is not a number without leading zeros"
            );
            part.parse()
                .with_context(|| format!("{text} is not a version"))
        };
        if let Some(pre) = pre {
            ensure!(
                pre.split('.').all(|identifier| !identifier.is_empty()
                    && identifier
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')),
                "{text} is not a version: {pre:?} is not a prerelease"
            );
        }
        Ok(Self {
            core: [number(major)?, number(minor)?, number(patch)?],
            pre: pre.map(str::to_owned),
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [major, minor, patch] = self.core;
        write!(formatter, "{major}.{minor}.{patch}")?;
        if let Some(pre) = &self.pre {
            write!(formatter, "-{pre}")?;
        }
        Ok(())
    }
}

/// SemVer precedence: a prerelease sorts before its release, so `0.1.1-beta.12 < 0.1.1`, and
/// numeric identifiers compare as numbers, so `beta.9 < beta.12`.
impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.core
            .cmp(&other.core)
            .then_with(|| match (&self.pre, &other.pre) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(pre), Some(other)) => identifiers(pre).cmp(&identifiers(other)),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A prerelease identifier; numbers sort before text.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Identifier<'a> {
    Number(u64),
    Text(&'a str),
}

fn identifiers(pre: &str) -> Vec<Identifier<'_>> {
    pre.split('.')
        .map(|part| match part.parse() {
            // `01` is text, so equal orderings stay equal versions.
            Ok(number) if part == "0" || !part.starts_with('0') => Identifier::Number(number),
            _ => Identifier::Text(part),
        })
        .collect()
}

pub(crate) fn read(root: &Path) -> anyhow::Result<Version> {
    let manifest = read_table(&root.join("Cargo.toml"))?;
    manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(Value::as_str)
        .context("Cargo.toml has no [workspace.package] version")?
        .parse()
}

/// The release after `current`. `main` only ever holds a stable version: a nightly beta stamps
/// its prerelease into the build, never into a commit.
fn next(current: &Version, part: &str) -> anyhow::Result<Version> {
    ensure!(
        current.pre.is_none(),
        "The current version {current} is a prerelease"
    );
    let [major, minor, patch] = current.core;
    let core = match part {
        "major" => [major + 1, 0, 0],
        "minor" => [major, minor + 1, 0],
        "patch" => [major, minor, patch + 1],
        exact => {
            let version: Version = exact.parse()?;
            ensure!(
                version.pre.is_none(),
                "{version} is a prerelease; expected X.Y.Z"
            );
            ensure!(
                version.core > current.core,
                "{version} is not newer than {current}"
            );
            version.core
        }
    };
    Ok(Version { core, pre: None })
}

/// Writes `version` to the root manifest and to the members' entries in `Cargo.lock`, exactly
/// the lines Cargo would change, so a `--locked` build accepts the result.
fn set(root: &Path, version: &Version) -> anyhow::Result<()> {
    let current = read(root)?;
    let manifest_path = root.join("Cargo.toml");
    let lock_path = root.join("Cargo.lock");
    let manifest = fs::read_to_string(&manifest_path)?;
    let mut lock = fs::read_to_string(&lock_path)?;
    for name in members(root)? {
        lock = replace_once(
            &lock,
            &format!("[[package]]\nname = \"{name}\"\nversion = \"{current}\"\n"),
            &format!("[[package]]\nname = \"{name}\"\nversion = \"{version}\"\n"),
        )
        .with_context(|| format!("Cargo.lock has no {name} {current}"))?;
    }
    let table = manifest
        .find("\n[workspace.package]\n")
        .context("Cargo.toml has no [workspace.package] table")?;
    let (head, tail) = manifest.split_at(table);
    let tail = replace_once(
        tail,
        &format!("\nversion = \"{current}\"\n"),
        &format!("\nversion = \"{version}\"\n"),
    )
    .context("Cargo.toml has no [workspace.package] version line")?;
    fs::write(manifest_path, format!("{head}{tail}"))?;
    fs::write(lock_path, lock)?;
    Ok(())
}

fn replace_once(text: &str, from: &str, to: &str) -> Option<String> {
    let start = text.find(from)?;
    Some(format!(
        "{}{to}{}",
        &text[..start],
        &text[start + from.len()..]
    ))
}

/// The package names of the workspace members, each of which must inherit the version.
fn members(root: &Path) -> anyhow::Result<Vec<String>> {
    let manifest = read_table(&root.join("Cargo.toml"))?;
    let members = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(Value::as_array)
        .context("Cargo.toml has no [workspace] members")?;
    members
        .iter()
        .map(|member| {
            let member = member
                .as_str()
                .context("A workspace member is not a path")?;
            let manifest = read_table(&root.join(member).join("Cargo.toml"))?;
            let package = manifest
                .get("package")
                .with_context(|| format!("{member}/Cargo.toml has no [package]"))?;
            let inherits = package
                .get("version")
                .and_then(|version| version.get("workspace"))
                .and_then(Value::as_bool);
            ensure!(
                inherits == Some(true),
                "{member}/Cargo.toml must declare `version.workspace = true`"
            );
            package
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .with_context(|| format!("{member}/Cargo.toml has no package name"))
        })
        .collect()
}

fn read_table(path: &Path) -> anyhow::Result<Table> {
    let text =
        fs::read_to_string(path).with_context(|| format!("Could not read {}", path.display()))?;
    text.parse()
        .with_context(|| format!("Invalid TOML in {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn next_of(current: &str, part: &str) -> anyhow::Result<String> {
        Ok(next(&current.parse()?, part)?.to_string())
    }

    #[test]
    fn bumps_each_part() -> anyhow::Result<()> {
        assert_eq!(next_of("1.2.3", "patch")?, "1.2.4");
        assert_eq!(next_of("1.2.3", "minor")?, "1.3.0");
        assert_eq!(next_of("1.2.3", "major")?, "2.0.0");
        assert_eq!(next_of("1.2.3", "1.10.0")?, "1.10.0");
        Ok(())
    }

    #[test]
    fn refuses_versions_that_do_not_move_forward() {
        for (current, part) in [
            ("1.2.3", "1.2.3"),
            ("1.2.3", "1.1.9"),
            ("1.2.3", "1.2.4-beta.1"),
            ("1.2.3-beta.1", "patch"),
            ("1.2.3", "next"),
        ] {
            assert!(next_of(current, part).is_err(), "{current} then {part}");
        }
    }

    #[test]
    fn parses_only_semver_without_build_metadata() {
        for valid in ["0.0.0", "10.20.30", "0.1.1-beta.12", "1.0.0-rc-1"] {
            assert_eq!(
                valid
                    .parse::<Version>()
                    .map(|version| version.to_string())
                    .ok()
                    .as_deref(),
                Some(valid)
            );
        }
        for invalid in [
            "",
            "1.2",
            "1.2.3.4",
            "01.2.3",
            "1.2.x",
            "1.2.3-",
            "1.2.3-beta..1",
            "1.2.3+build",
            "v1.2.3",
        ] {
            assert!(invalid.parse::<Version>().is_err(), "{invalid}");
        }
    }

    #[test]
    fn orders_prereleases_before_their_release() -> anyhow::Result<()> {
        let ordered = [
            "0.1.0",
            "0.1.1-beta.3",
            "0.1.1-beta.12",
            "0.1.1-beta.12.1",
            "0.1.1-rc.1",
            "0.1.1",
            "0.2.0-beta.1",
            "0.10.0",
        ];
        for pair in ordered.windows(2) {
            let [lower, higher] = pair else { continue };
            assert!(
                lower.parse::<Version>()? < higher.parse::<Version>()?,
                "{lower} < {higher}"
            );
        }
        Ok(())
    }

    #[test]
    fn stamps_the_manifest_and_only_member_lock_entries() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let root = root.path();
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"app\"]\n\n[workspace.package]\nedition = \"2024\"\nversion = \"0.1.0\"\n\n[workspace.dependencies]\nother = { version = \"0.1.0\" }\n",
        )?;
        fs::create_dir(root.join("app"))?;
        fs::write(
            root.join("app/Cargo.toml"),
            "[package]\nname = \"app\"\nversion.workspace = true\n",
        )?;
        let lock = "[[package]]\nname = \"app\"\nversion = \"0.1.0\"\ndependencies = [\"other\"]\n\n[[package]]\nname = \"other\"\nversion = \"0.1.0\"\n";
        fs::write(root.join("Cargo.lock"), lock)?;

        set(root, &"0.2.0-beta.3".parse()?)?;

        assert_eq!(read(root)?.to_string(), "0.2.0-beta.3");
        let manifest = fs::read_to_string(root.join("Cargo.toml"))?;
        assert!(manifest.contains("other = { version = \"0.1.0\" }"));
        assert_eq!(
            fs::read_to_string(root.join("Cargo.lock"))?,
            lock.replacen("version = \"0.1.0\"", "version = \"0.2.0-beta.3\"", 1)
        );
        Ok(())
    }
}
