//! `catalog updates --out <site>` (ADR 0018): mirrors the signed update manifests into the site,
//! byte for byte so their signatures still verify. `updates/v1/stable.json` is the latest
//! release's `update.json`, and `beta.json` the newer of it and the newest prerelease's, each
//! beside its `.minisig`. A release published before manifests existed has none and is skipped.
//!
//! A Pages deploy replaces the whole site, so every catalog run rebuilds `updates/`. When the
//! releases cannot be read, the live site's manifests are carried forward and the run fails
//! once it has deployed; when the live site cannot be read either, nothing deploys.

#[cfg(test)]
mod tests;

use super::{
    Outcome, SITE,
    http::{Fetched, Http},
    write,
};
use crate::{
    update_manifest::{MANIFEST, Manifest, RELEASES, SIGNATURE},
    version::Version,
};
use anyhow::{Context as _, bail, ensure};
use serde::Deserialize;
use std::{path::Path, process::Command};

/// Where the manifests live, under the site and on Pages.
const UPDATES: &str = "updates/v1";
const MANIFEST_BYTES: u64 = 1024 * 1024;

/// The repository's published releases.
pub(super) trait Releases {
    fn list(&self) -> anyhow::Result<Vec<Release>>;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Release {
    tag_name: String,
    is_prerelease: bool,
    is_latest: bool,
}

/// The GitHub CLI, authenticated by the workflow's `GH_TOKEN`.
pub(super) struct Gh;

impl Releases for Gh {
    fn list(&self) -> anyhow::Result<Vec<Release>> {
        let output = Command::new("gh")
            .args(["release", "list", "--exclude-drafts", "--limit", "200"])
            .args(["--json", "tagName,isPrerelease,isLatest"])
            .output()
            .context("Could not start gh")?;
        ensure!(
            output.status.success(),
            "gh release list failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        serde_json::from_slice(&output.stdout).context("gh release list printed unexpected JSON")
    }
}

/// A release's manifest and signature, as published.
#[derive(Clone)]
struct Signed {
    version: Version,
    manifest: Vec<u8>,
    signature: Vec<u8>,
}

pub(super) fn mirror(
    releases: &dyn Releases,
    http: &dyn Http,
    out: &Path,
) -> anyhow::Result<Outcome> {
    let out = out.join(UPDATES);
    let error = match channels(releases, http) {
        Ok(channels) => {
            for (channel, signed) in channels {
                write(&out.join(format!("{channel}.json")), &signed.manifest)?;
                write(
                    &out.join(format!("{channel}.json.minisig")),
                    &signed.signature,
                )?;
                println!("Mirrored v{} as {channel}.", signed.version);
            }
            return Ok(Outcome::Fresh);
        }
        Err(error) => error,
    };
    for name in [
        "stable.json",
        "stable.json.minisig",
        "beta.json",
        "beta.json.minisig",
    ] {
        let url = format!("{SITE}{UPDATES}/{name}");
        let fetched = http
            .get(&url, MANIFEST_BYTES)
            .context("Neither the releases nor the published update manifests could be read")?;
        if let Fetched::Found(bytes) = fetched {
            write(&out.join(name), &bytes)?;
        }
    }
    Ok(Outcome::StaleUpdates(error))
}

/// The manifest each channel serves, when a release has one.
fn channels(
    releases: &dyn Releases,
    http: &dyn Http,
) -> anyhow::Result<Vec<(&'static str, Signed)>> {
    let listed = releases.list()?;
    let stable = match listed.iter().find(|release| release.is_latest) {
        Some(release) => signed(http, &release.tag_name)?,
        None => None,
    };
    let mut prereleases = Vec::new();
    for release in listed.iter().filter(|release| release.is_prerelease) {
        prereleases.push((version(&release.tag_name)?, &release.tag_name));
    }
    let beta = match prereleases.into_iter().max() {
        Some((_, tag)) => signed(http, tag)?,
        None => None,
    };
    let beta = match (&stable, beta) {
        (Some(stable), Some(beta)) if stable.version > beta.version => Some(stable.clone()),
        (stable, beta) => beta.or_else(|| stable.clone()),
    };
    Ok([("stable", stable), ("beta", beta)]
        .into_iter()
        .filter_map(|(channel, signed)| Some((channel, signed?)))
        .collect())
}

/// `tag`'s manifest and signature, or `None` when the release has neither.
fn signed(http: &dyn Http, tag: &str) -> anyhow::Result<Option<Signed>> {
    let get = |name: &str| http.get(&format!("{RELEASES}download/{tag}/{name}"), MANIFEST_BYTES);
    match (get(MANIFEST)?, get(SIGNATURE)?) {
        (Fetched::Missing, Fetched::Missing) => Ok(None),
        (Fetched::Found(manifest), Fetched::Found(signature)) => {
            let named: Manifest = serde_json::from_slice(&manifest)
                .with_context(|| format!("{tag}'s {MANIFEST} is malformed"))?;
            let version = named.version.parse()?;
            ensure!(
                version == self::version(tag)?,
                "{tag}'s {MANIFEST} names {version}"
            );
            Ok(Some(Signed {
                version,
                manifest,
                signature,
            }))
        }
        _ => bail!("{tag} has only one of {MANIFEST} and {SIGNATURE}"),
    }
}

fn version(tag: &str) -> anyhow::Result<Version> {
    tag.strip_prefix('v')
        .with_context(|| format!("{tag} is not a vX.Y.Z tag"))?
        .parse()
}
