#![expect(
    clippy::expect_used,
    reason = "Test assertions report fixture failures."
)]

use super::{Release, Releases, UPDATES, mirror};
use crate::{
    catalog::{
        Outcome, SITE,
        http::{Fetched, Http},
    },
    update_manifest::RELEASES,
};
use anyhow::{anyhow, bail};
use std::{collections::BTreeMap, fs, path::Path};

struct Listed(Option<Vec<(&'static str, bool, bool)>>);

impl Releases for Listed {
    fn list(&self) -> anyhow::Result<Vec<Release>> {
        let listed = self
            .0
            .as_ref()
            .ok_or_else(|| anyhow!("gh release list failed"))?;
        Ok(listed
            .iter()
            .map(|&(tag, is_prerelease, is_latest)| Release {
                tag_name: tag.to_owned(),
                is_prerelease,
                is_latest,
            })
            .collect())
    }
}

/// Published files by URL; any other URL answers 404, and `down` fails every request.
#[derive(Default)]
struct Web {
    files: BTreeMap<String, Vec<u8>>,
    down: bool,
}

impl Web {
    /// A release's manifest naming `version`, and a signature naming its tag.
    fn release(mut self, version: &str) -> Self {
        let manifest = format!(
            r#"{{"version":"{version}","notes_url":"n","published_at":"t","assets":{{}}}}"#
        );
        let base = format!("{RELEASES}download/v{version}/");
        self.files
            .insert(format!("{base}update.json"), manifest.into_bytes());
        self.files.insert(
            format!("{base}update.json.minisig"),
            format!("sig v{version}").into_bytes(),
        );
        self
    }

    fn published(mut self, name: &str, bytes: &str) -> Self {
        self.files
            .insert(format!("{SITE}{UPDATES}/{name}"), bytes.as_bytes().to_vec());
        self
    }
}

impl Http for Web {
    fn get(&self, url: &str, _limit: u64) -> anyhow::Result<Fetched> {
        if self.down {
            bail!("GET {url} could not connect");
        }
        Ok(self
            .files
            .get(url)
            .map_or(Fetched::Missing, |bytes| Fetched::Found(bytes.clone())))
    }
}

/// The mirrored files by name, and whether the run carried the live site forward.
fn run(listed: Listed, web: &Web) -> anyhow::Result<(BTreeMap<String, String>, bool)> {
    let site = tempfile::tempdir().expect("temporary directory");
    let stale = matches!(mirror(&listed, web, site.path())?, Outcome::StaleUpdates(_));
    Ok((files(&site.path().join(UPDATES)), stale))
}

fn files(dir: &Path) -> BTreeMap<String, String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return BTreeMap::new();
    };
    entries
        .map(|entry| {
            let entry = entry.expect("readable entry");
            let name = entry.file_name().into_string().expect("UTF-8 name");
            (
                name,
                fs::read_to_string(entry.path()).expect("readable file"),
            )
        })
        .collect()
}

fn version_of(files: &BTreeMap<String, String>, name: &str) -> Option<String> {
    let manifest: serde_json::Value = serde_json::from_str(files.get(name)?).ok()?;
    manifest["version"].as_str().map(str::to_owned)
}

#[test]
fn beta_serves_the_newer_of_the_release_and_the_newest_prerelease() -> anyhow::Result<()> {
    let web = Web::default()
        .release("0.2.0")
        .release("0.2.1-beta.9")
        .release("0.2.1-beta.12");
    let releases = || {
        Listed(Some(vec![
            ("v0.2.1-beta.9", true, false),
            ("v0.2.1-beta.12", true, false),
            ("v0.2.0", false, true),
        ]))
    };
    let (files, stale) = run(releases(), &web)?;
    assert!(!stale);
    assert_eq!(version_of(&files, "stable.json").as_deref(), Some("0.2.0"));
    assert_eq!(
        version_of(&files, "beta.json").as_deref(),
        Some("0.2.1-beta.12")
    );
    assert_eq!(files["beta.json.minisig"], "sig v0.2.1-beta.12");
    assert_eq!(files["stable.json.minisig"], "sig v0.2.0");

    // A release after the betas is served on both channels, byte for byte.
    let web = web.release("0.2.1");
    let mut listed = releases().0.expect("listed");
    listed[2].2 = false;
    listed.push(("v0.2.1", false, true));
    let (files, _) = run(Listed(Some(listed)), &web)?;
    assert_eq!(files["beta.json"], files["stable.json"]);
    assert_eq!(files["beta.json.minisig"], "sig v0.2.1");
    Ok(())
}

#[test]
fn releases_without_manifests_publish_nothing_for_their_channel() -> anyhow::Result<()> {
    let (files, stale) = run(Listed(Some(vec![("v0.1.0", false, true)])), &Web::default())?;
    assert!(files.is_empty() && !stale);

    let web = Web::default().release("0.1.1-beta.2");
    let listed = Listed(Some(vec![
        ("v0.1.1-beta.2", true, false),
        ("v0.1.0", false, true),
    ]));
    let (files, _) = run(listed, &web)?;
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        ["beta.json", "beta.json.minisig"]
    );
    Ok(())
}

#[test]
fn a_failed_copy_carries_the_live_manifests_forward() -> anyhow::Result<()> {
    let web = Web::default()
        .published("stable.json", "live stable")
        .published("stable.json.minisig", "live stable sig")
        .published("beta.json", "live beta")
        .published("beta.json.minisig", "live beta sig");
    let (files, stale) = run(Listed(None), &web)?;
    assert!(stale);
    assert_eq!(files["beta.json"], "live beta");
    assert_eq!(files.len(), 4);

    // A manifest that names another version is a failed copy too.
    let mut web = web.release("0.3.0");
    let moved = web
        .files
        .remove(&format!("{RELEASES}download/v0.3.0/update.json"))
        .expect("fixture manifest");
    web.files
        .insert(format!("{RELEASES}download/v0.2.0/update.json"), moved);
    web.files.insert(
        format!("{RELEASES}download/v0.2.0/update.json.minisig"),
        b"sig".to_vec(),
    );
    let (files, stale) = run(Listed(Some(vec![("v0.2.0", false, true)])), &web)?;
    assert!(stale);
    assert_eq!(files["stable.json"], "live stable");
    Ok(())
}

#[test]
fn nothing_deploys_when_the_live_site_is_unreachable_too() {
    let web = Web {
        down: true,
        ..Web::default()
    };
    assert!(run(Listed(None), &web).is_err());
}
