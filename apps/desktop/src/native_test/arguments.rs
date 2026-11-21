use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, bail, ensure};
use skillbinder_engine::{CatalogSource, EngineConfig, GitSearch, UpdatePayload, UpdateSource};

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
    let mut git = None;
    let mut catalog = None;
    let mut updates = None;
    let mut version = None;
    let mut bundle = None;
    let mut mode = NativeMode::Interactive;
    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some(
                "--data-dir" | "--home" | "--git-dir" | "--catalog-dir" | "--update-dir"
                | "--app-version" | "--app-bundle",
            ) => {
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
                let slot = match argument.to_str() {
                    Some("--data-dir") => &mut data,
                    Some("--home") => &mut home,
                    Some("--git-dir") => &mut git,
                    Some("--update-dir") => &mut updates,
                    Some("--app-version") => &mut version,
                    Some("--app-bundle") => &mut bundle,
                    _ => &mut catalog,
                };
                ensure!(
                    slot.replace(path).is_none(),
                    "Duplicate {}",
                    argument.to_string_lossy()
                );
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
    let data =
        data.ok_or_else(|| anyhow::anyhow!("Native test builds require an isolated --data-dir"))?;
    let platform = EngineConfig::platform()?;
    let data = resolved_path(&data)?;
    ensure!(
        data != resolved_path(&platform.data_dir)?,
        "--data-dir cannot use the platform data directory"
    );
    let home = resolved_path(&home.unwrap_or_else(|| data.join("fixture-home")))?;
    ensure!(
        home != resolved_path(&platform.home)?,
        "--home cannot use the real home"
    );
    let mut config = EngineConfig::isolated(data, home);
    // Only this directory is searched, so a check can start without Git and add it later.
    if let Some(git) = git {
        config.git_search = GitSearch::Directory(resolved_path(&git)?);
    }
    // A fixture catalog, laid out like the CDN; renaming it takes the catalog offline.
    if let Some(catalog) = catalog {
        config.catalog = CatalogSource::Directory(resolved_path(&catalog)?);
    }
    // A fixture update site laid out like the Pages site, signed with the key in its `key.pub`;
    // renaming it takes checks offline. Its archives are a bundle's executable, unsigned.
    if let Some(updates) = updates {
        let site = resolved_path(&updates)?;
        let public_key = fs::read_to_string(site.join("key.pub"))
            .with_context(|| format!("No key.pub in {}", site.display()))?;
        config.app_update = Some(UpdateSource {
            site: CatalogSource::Directory(site),
            public_key: public_key.trim().to_owned(),
            payload: UpdatePayload::Executable,
        });
    }
    // The version checks compare against, such as a beta to see the Beta→Stable line.
    if let Some(version) = version {
        config.app_version = version
            .to_string_lossy()
            .parse()
            .context("--app-version takes X.Y.Z or X.Y.Z-beta.N")?;
    }
    // The app runs as this bundle, such as `<fixture>/Downloads/SkillBinder.app`, to see Move to
    // Applications; Move copies into the fixture's Applications folders.
    if let Some(bundle) = bundle {
        config.placement.bundle = Some(resolved_path(&bundle)?);
    }
    Ok((config, mode))
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
        assert_eq!(parse(&["--data-dir", fixture])?.1, NativeMode::Interactive);
        assert_eq!(parse(&["--data-dir", fixture, "--mcp"])?.1, NativeMode::Mcp);
        assert_eq!(
            parse(&["--data-dir", fixture, "--git-dir", fixture])?
                .0
                .git_search,
            GitSearch::Directory(resolved_path(Path::new(fixture))?)
        );
        assert_eq!(
            parse(&["--data-dir", fixture])?.0.git_search,
            GitSearch::System
        );
        assert_eq!(
            parse(&["--data-dir", fixture, "--catalog-dir", fixture])?
                .0
                .catalog,
            CatalogSource::Directory(resolved_path(Path::new(fixture))?)
        );
        assert_eq!(
            parse(&["--data-dir", fixture, "--app-version", "0.1.1-beta.12"])?
                .0
                .app_version
                .to_string(),
            "0.1.1-beta.12"
        );
        assert_eq!(parse(&["--data-dir", fixture])?.0.app_update, None);
        assert_eq!(parse(&["--data-dir", fixture])?.0.placement.bundle, None);
        let bundle = std::env::temp_dir().join("Downloads/SkillBinder.app");
        let bundle_arg = bundle.to_str().context("bundle path is not UTF-8")?;
        assert_eq!(
            parse(&["--data-dir", fixture, "--app-bundle", bundle_arg])?
                .0
                .placement
                .bundle,
            Some(resolved_path(&bundle)?)
        );
        for args in [
            vec![],
            vec!["--data-dir", fixture, "--app-version", "0.1"],
            vec!["--data-dir", fixture, "--update-dir", fixture],
            vec!["--mcp"],
            vec!["--smoke-test"],
            vec!["--mcp", "--smoke-test", "--data-dir", fixture],
            vec!["--mcp", "--mcp", "--data-dir", fixture],
            vec!["--home", fixture],
            vec!["--data-dir"],
            vec!["--data-dir", "--mcp"],
            vec!["--data-dir", fixture, "--data-dir", fixture],
            vec![
                "--data-dir",
                fixture,
                "--git-dir",
                fixture,
                "--git-dir",
                fixture,
            ],
        ] {
            assert!(parse(&args).is_err(), "accepted {args:?}");
        }
        Ok(())
    }

    #[test]
    fn rejects_real_data_and_home_paths() -> anyhow::Result<()> {
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
