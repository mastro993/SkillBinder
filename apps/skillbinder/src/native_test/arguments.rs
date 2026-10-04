use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, bail, ensure};
use skillbinder_engine::EngineConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeMode {
    Interactive,
    Smoke,
    Mcp,
}

pub(crate) fn parse_launch(
    args: impl IntoIterator<Item = OsString>,
) -> anyhow::Result<(EngineConfig, NativeMode)> {
    let mut args = args.into_iter();
    let mut data = None;
    let mut home = None;
    let mut mode = NativeMode::Interactive;
    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some("--data-dir") | Some("--home") => {
                let value = args.next().ok_or_else(|| {
                    anyhow::anyhow!("Missing value for {}", argument.to_string_lossy())
                })?;
                ensure!(
                    !value.to_string_lossy().starts_with("--"),
                    "Missing value for {}",
                    argument.to_string_lossy()
                );
                let path = PathBuf::from(value);
                ensure!(
                    !path.as_os_str().is_empty(),
                    "Empty value for {}",
                    argument.to_string_lossy()
                );
                if argument == "--data-dir" {
                    ensure!(data.replace(path).is_none(), "Duplicate --data-dir");
                } else {
                    ensure!(home.replace(path).is_none(), "Duplicate --home");
                }
            }
            Some("--smoke-test") if mode == NativeMode::Interactive => mode = NativeMode::Smoke,
            Some("--mcp") if mode == NativeMode::Interactive => mode = NativeMode::Mcp,
            Some("--smoke-test" | "--mcp") => {
                bail!("Native test modes cannot be combined or repeated")
            }
            _ => bail!(
                "Unknown native-test argument: {}",
                argument.to_string_lossy()
            ),
        }
    }
    ensure!(
        home.is_none() || data.is_some(),
        "--home requires --data-dir"
    );
    ensure!(
        mode == NativeMode::Interactive || data.is_some(),
        "--smoke-test and --mcp require an isolated --data-dir"
    );
    let Some(data) = data else {
        return Ok((EngineConfig::platform()?, mode));
    };
    let platform = EngineConfig::platform()?;
    let data = resolved_path(&data)?;
    ensure!(
        data != resolved_path(&platform.data_dir)?,
        "--data-dir cannot use the platform data directory"
    );
    let home = resolved_path(&home.unwrap_or_else(|| data.join("fixture-home")))?;
    if mode == NativeMode::Mcp {
        ensure!(
            home != resolved_path(&platform.home)?,
            "--home cannot use the real home in MCP mode"
        );
    }
    Ok((EngineConfig::isolated(data, home), mode))
}

fn resolved_path(path: &Path) -> anyhow::Result<PathBuf> {
    let absolute = std::path::absolute(path).context("Could not resolve native test path")?;
    let mut ancestor = absolute.as_path();
    let mut missing = Vec::new();
    while !ancestor.exists() {
        let name = ancestor
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid native test path"))?;
        missing.push(name.to_owned());
        ancestor = ancestor
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid native test path"))?;
    }
    let mut resolved = fs::canonicalize(ancestor).context("Could not resolve native test path")?;
    for name in missing.into_iter().rev() {
        resolved.push(name);
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> anyhow::Result<(EngineConfig, NativeMode)> {
        parse_launch(args.iter().map(OsString::from))
    }

    #[test]
    fn modes_require_isolation_and_exclude_each_other() -> anyhow::Result<()> {
        let fixture = std::env::temp_dir().join("skillbinder-parser-fixture");
        let fixture = fixture.to_str().context("fixture path is not UTF-8")?;
        assert_eq!(parse(&[])?.1, NativeMode::Interactive);
        assert_eq!(parse(&["--data-dir", fixture, "--mcp"])?.1, NativeMode::Mcp);
        for args in [
            vec!["--mcp"],
            vec!["--smoke-test"],
            vec!["--mcp", "--smoke-test", "--data-dir", fixture],
            vec!["--mcp", "--mcp", "--data-dir", fixture],
            vec!["--home", fixture],
            vec!["--data-dir"],
            vec!["--data-dir", "--mcp"],
            vec!["--data-dir", fixture, "--data-dir", fixture],
        ] {
            assert!(parse(&args).is_err(), "accepted {args:?}");
        }
        Ok(())
    }

    #[test]
    fn mcp_rejects_real_data_and_home_paths() -> anyhow::Result<()> {
        let platform = EngineConfig::platform()?;
        let fixture = std::env::temp_dir().join("skillbinder-parser-fixture");
        let fixture = fixture.to_str().context("fixture path is not UTF-8")?;
        assert!(
            parse(&[
                "--mcp",
                "--data-dir",
                platform
                    .data_dir
                    .to_str()
                    .context("platform data directory is not UTF-8")?
            ])
            .is_err()
        );
        assert!(
            parse(&[
                "--mcp",
                "--data-dir",
                fixture,
                "--home",
                platform
                    .home
                    .to_str()
                    .context("home directory is not UTF-8")?
            ])
            .is_err()
        );
        Ok(())
    }
}
