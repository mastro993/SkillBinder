//! Runs a development build on a development world: a synthetic world kept in the checkout at
//! `.skillbinder-dev/<name>/`, so it runs beside the installed app and other worlds without
//! touching the user's library or agents (ADR 0029).

use crate::fixture;
use anyhow::{Context as _, bail, ensure};
use std::{
    ffi::OsString,
    fs::{self, File, TryLockError},
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
};

/// Written once a world's build finishes; a world without it is rebuilt.
const COMPLETE: &str = ".complete";
const USAGE: &str = "Usage: cargo run -p xtask -- dev [<name>] [--fresh] [-- <app arguments>]";

#[derive(Debug, PartialEq)]
struct Dev {
    name: String,
    fresh: bool,
    app_args: Vec<OsString>,
}

pub(crate) fn run(root: &Path, args: impl IntoIterator<Item = OsString>) -> anyhow::Result<()> {
    let Dev {
        name,
        fresh,
        app_args,
    } = parse(args)?;
    let world = world(root, &name);
    if reset(&world, &name, fresh)? {
        println!("Building the {name} world…");
        fixture::build_at(root, &world)?;
        fs::write(world.join(COMPLETE), "")?;
    }
    print_summary(&world)?;
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo)
        .current_dir(root)
        .args(app_arguments(&world, app_args))
        .status()
        .context("Could not run cargo")?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

fn parse(args: impl IntoIterator<Item = OsString>) -> anyhow::Result<Dev> {
    let mut args = args.into_iter();
    let (mut name, mut fresh) = (None, false);
    for arg in args.by_ref() {
        let Some(text) = arg.to_str() else {
            bail!("Arguments before -- must be valid UTF-8\n{USAGE}");
        };
        match text {
            "--" => break,
            "--fresh" => fresh = true,
            option if option.starts_with('-') => bail!("Unknown option {option}\n{USAGE}"),
            _ if name.is_some() => bail!("Unexpected argument {text}\n{USAGE}"),
            _ => name = Some(text.to_owned()),
        }
    }
    let name = name.unwrap_or_else(|| "default".to_owned());
    ensure!(
        !matches!(name.as_str(), "" | "." | "..") && !name.contains(['/', '\\']),
        "Choose a world name without path separators, not {name:?}"
    );
    Ok(Dev {
        name,
        fresh,
        app_args: args.collect(),
    })
}

fn world(root: &Path, name: &str) -> PathBuf {
    root.join(".skillbinder-dev").join(name)
}

/// Deletes a world that must be rebuilt, an incomplete one or any with `fresh`, and says whether
/// to build it. Refuses to delete one whose app runs.
fn reset(world: &Path, name: &str, fresh: bool) -> anyhow::Result<bool> {
    if !world.exists() {
        return Ok(true);
    }
    if !fresh && world.join(COMPLETE).exists() {
        return Ok(false);
    }
    ensure!(
        !running(world)?,
        "The {name} world is running; quit it first."
    );
    fs::remove_dir_all(world).with_context(|| format!("Could not delete {}", world.display()))?;
    Ok(true)
}

/// Whether an app holds the world's instance lock.
fn running(world: &Path) -> anyhow::Result<bool> {
    let path = world.join("data/application.lock");
    let lock = match File::open(&path) {
        Ok(lock) => lock,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(error).with_context(|| format!("Could not open {}", path.display()));
        }
    };
    match lock.try_lock() {
        Ok(()) => Ok(false),
        Err(TryLockError::WouldBlock) => Ok(true),
        Err(TryLockError::Error(error)) => {
            Err(error).with_context(|| format!("Could not check {}", path.display()))
        }
    }
}

/// The cargo arguments that launch the `native-test` app on `world`, then `app_args`.
fn app_arguments(world: &Path, app_args: Vec<OsString>) -> Vec<OsString> {
    let mut args: Vec<OsString> = ["run", "-p", "skillbinder", "--features", "native-test"]
        .into_iter()
        .chain(["--locked", "--"])
        .map(OsString::from)
        .collect();
    for (flag, folder) in [
        ("--data-dir", "data"),
        ("--home", "home"),
        ("--catalog-dir", "catalog"),
        ("--update-dir", "app-updates"),
    ] {
        args.push(flag.into());
        args.push(world.join(folder).into_os_string());
    }
    args.extend(app_args);
    args
}

fn print_summary(world: &Path) -> anyhow::Result<()> {
    println!("Development world at {}", world.display());
    println!(
        "  Projects to add as workspaces: {}",
        world.join("projects").display()
    );
    println!(
        "  Sync remote: {}",
        fixture::file_url(&world.join("remotes/library.git"))?
    );
    println!(
        "  Command line: cargo run -p skillbinder-cli -- --data-dir {} <command>",
        world.join("data").display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(args: &[&str]) -> anyhow::Result<Dev> {
        parse(args.iter().map(OsString::from))
    }

    /// A world folder, finished when `complete`.
    fn world_in(scratch: &Path, complete: bool) -> anyhow::Result<PathBuf> {
        let world = world(scratch, "default");
        fs::create_dir_all(world.join("data"))?;
        if complete {
            fs::write(world.join(COMPLETE), "")?;
        }
        Ok(world)
    }

    /// Holds the world's instance lock, as its running app would.
    fn hold_lock(world: &Path) -> anyhow::Result<File> {
        let lock = File::create(world.join("data/application.lock"))?;
        lock.try_lock()?;
        Ok(lock)
    }

    #[test]
    fn names_default_and_pass_app_arguments() -> anyhow::Result<()> {
        let none = Vec::<OsString>::new();
        assert_eq!(
            parsed(&[])?,
            Dev {
                name: "default".into(),
                fresh: false,
                app_args: none
            }
        );
        assert_eq!(
            parsed(&[
                "second",
                "--fresh",
                "--",
                "--mcp",
                "--app-version",
                "0.1.1-beta.12"
            ])?,
            Dev {
                name: "second".into(),
                fresh: true,
                app_args: ["--mcp", "--app-version", "0.1.1-beta.12"]
                    .map(OsString::from)
                    .to_vec(),
            }
        );
        Ok(())
    }

    #[test]
    fn refuses_options_and_names_that_are_not_worlds() {
        for args in [
            &["--mcp"][..],
            &["a", "b"],
            &[""],
            &[".."],
            &["a/b"],
            &["a\\b"],
        ] {
            assert!(parsed(args).is_err(), "{args:?}");
        }
    }

    #[test]
    fn places_worlds_in_the_checkout() {
        assert_eq!(
            world(Path::new("/checkout"), "second"),
            Path::new("/checkout/.skillbinder-dev/second")
        );
    }

    #[test]
    fn assembles_the_native_test_launch() {
        let world = Path::new("/w");
        let args = app_arguments(world, vec!["--mcp".into()]);
        let expected = [
            "run".into(),
            "-p".into(),
            "skillbinder".into(),
            "--features".into(),
            "native-test".into(),
            "--locked".into(),
            "--".into(),
            "--data-dir".into(),
            world.join("data").into_os_string(),
            "--home".into(),
            world.join("home").into_os_string(),
            "--catalog-dir".into(),
            world.join("catalog").into_os_string(),
            "--update-dir".into(),
            world.join("app-updates").into_os_string(),
            "--mcp".into(),
        ];
        assert_eq!(args, expected);
    }

    #[test]
    fn builds_missing_and_incomplete_worlds_and_keeps_complete_ones() -> anyhow::Result<()> {
        let scratch = tempfile::tempdir()?;
        let missing = world(scratch.path(), "default");
        assert!(reset(&missing, "default", false)?);

        let complete = world_in(scratch.path(), true)?;
        assert!(!reset(&complete, "default", false)?);
        assert!(complete.join(COMPLETE).exists());

        fs::remove_file(complete.join(COMPLETE))?;
        assert!(reset(&complete, "default", false)?);
        assert!(!complete.exists());
        Ok(())
    }

    #[test]
    fn fresh_rebuilds_a_complete_world() -> anyhow::Result<()> {
        let scratch = tempfile::tempdir()?;
        let world = world_in(scratch.path(), true)?;
        assert!(reset(&world, "default", true)?);
        assert!(!world.exists());
        Ok(())
    }

    #[test]
    fn launches_a_running_complete_world_without_rebuilding() -> anyhow::Result<()> {
        let scratch = tempfile::tempdir()?;
        let world = world_in(scratch.path(), true)?;
        let _lock = hold_lock(&world)?;
        assert!(!reset(&world, "default", false)?);
        Ok(())
    }

    #[test]
    fn refuses_to_rebuild_a_running_world() -> anyhow::Result<()> {
        for (complete, fresh) in [(true, true), (false, false)] {
            let scratch = tempfile::tempdir()?;
            let world = world_in(scratch.path(), complete)?;
            let _lock = hold_lock(&world)?;
            let error = reset(&world, "default", fresh).err().context("rebuilt")?;
            assert_eq!(
                error.to_string(),
                "The default world is running; quit it first."
            );
            assert!(world.join("data/application.lock").exists());
        }
        Ok(())
    }
}
