//! The synthetic world's app update site: `updates/v1/{stable,beta}.json` and their signatures,
//! laid out like the Pages site, signed with a key made for the world, whose public half is
//! `key.pub`. Each archive is `downloads/<version>-<target>`, a stand-in for the bundle's
//! executable, which a native-test build running as an installed bundle downloads and stages.
//! Native-test builds read it with `--update-dir`.

use minisign::KeyPair;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, io::Cursor, path::Path};

/// The versions each channel offers: newer than any build of the workspace's 0.1 line.
const CHANNELS: [(&str, &str); 2] = [("stable", "0.2.0"), ("beta", "0.2.1-beta.3")];
const TARGETS: [&str; 3] = [
    "universal-apple-darwin",
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
];

pub(super) fn write(site: &Path) -> anyhow::Result<()> {
    let key = KeyPair::generate_unencrypted_keypair()?;
    let manifests = site.join("updates/v1");
    let downloads = site.join("downloads");
    fs::create_dir_all(&manifests)?;
    fs::create_dir_all(&downloads)?;
    for (channel, version) in CHANNELS {
        let mut assets = serde_json::Map::new();
        for target in TARGETS {
            let url = format!("downloads/{version}-{target}");
            let archive = format!("SkillBinder {version} for {target}\n");
            fs::write(site.join(&url), &archive)?;
            let sha256 = format!("{:x}", Sha256::digest(&archive));
            let asset = json!({ "url": url, "sha256": sha256, "size": archive.len() });
            assets.insert(target.to_owned(), asset);
        }
        let manifest = serde_json::to_string_pretty(&json!({
            "version": version,
            "notes_url": format!("https://example.invalid/releases/tag/v{version}"),
            "published_at": "2026-12-01T10:00:00Z",
            "assets": assets,
        }))? + "\n";
        let signature = minisign::sign(None, &key.sk, Cursor::new(&manifest), None, None)?;
        fs::write(manifests.join(format!("{channel}.json")), &manifest)?;
        fs::write(
            manifests.join(format!("{channel}.json.minisig")),
            signature.into_string(),
        )?;
    }
    fs::write(site.join("key.pub"), key.pk.to_base64() + "\n")?;
    Ok(())
}
