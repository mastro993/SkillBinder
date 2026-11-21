//! `update-manifest --version <v> --dist <dir>` (ADR 0018): describes the release's archives in
//! `<dir>/update.json` and signs it with minisign into `<dir>/update.json.minisig`. The Release
//! and Nightly workflows upload both as release assets, and the Catalog job mirrors them to
//! `updates/v1/` on the Pages site (`catalog updates`).

use crate::{catalog::timestamp, version::Version};
use anyhow::{Context as _, anyhow, bail, ensure};
use minisign::{SecretKey, SecretKeyBox};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path, time::SystemTime};

/// The repository's GitHub Releases; archive and release-page URLs hang off it.
pub(crate) const RELEASES: &str = "https://github.com/mastro993/SkillBinder/releases/";
pub(crate) const MANIFEST: &str = "update.json";
pub(crate) const SIGNATURE: &str = "update.json.minisig";
/// What `dist/` holds beside the archives.
const NOT_ARCHIVES: &[&str] = &["SHA256SUMS", MANIFEST, SIGNATURE];
const USAGE: &str = "Usage: cargo run -p xtask -- update-manifest --version <version> --dist <directory>, with UPDATE_SIGNING_KEY and UPDATE_SIGNING_KEY_PASSWORD set";

/// `update.json`. The app ignores unknown fields; a breaking change publishes `updates/v2/`.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Manifest {
    pub(crate) version: String,
    notes_url: String,
    published_at: String,
    /// By target triple, so a platform can change its archive format under the same key.
    assets: BTreeMap<String, Asset>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Asset {
    url: String,
    sha256: String,
    size: u64,
}

pub(crate) fn run(args: &[String]) -> anyhow::Result<()> {
    let [version_flag, version, dist_flag, dist] = args else {
        bail!(USAGE);
    };
    ensure!(version_flag == "--version" && dist_flag == "--dist", USAGE);
    let version: Version = version.parse()?;
    let key = secret_key()?;
    let dist = Path::new(dist);
    write(dist, &version, SystemTime::now(), &key)?;
    println!(
        "Wrote and signed {} for v{version}.",
        dist.join(MANIFEST).display()
    );
    Ok(())
}

/// `UPDATE_SIGNING_KEY` holds the minisign secret key file, encrypted with the password in
/// `UPDATE_SIGNING_KEY_PASSWORD` (docs/release.md).
fn secret_key() -> anyhow::Result<SecretKey> {
    let read =
        |name: &str| std::env::var(name).with_context(|| format!("{name} is not set; {USAGE}"));
    open(
        &read("UPDATE_SIGNING_KEY")?,
        read("UPDATE_SIGNING_KEY_PASSWORD")?,
    )
}

fn open(key_file: &str, password: String) -> anyhow::Result<SecretKey> {
    SecretKeyBox::from_string(key_file)
        .and_then(|key| key.into_secret_key(Some(password)))
        .context("UPDATE_SIGNING_KEY does not open with UPDATE_SIGNING_KEY_PASSWORD")
}

/// Writes the manifest, then its signature over exactly the written bytes.
fn write(dist: &Path, version: &Version, now: SystemTime, key: &SecretKey) -> anyhow::Result<()> {
    let manifest = serde_json::to_string_pretty(&describe(dist, version, now)?)? + "\n";
    let signature = minisign::sign(None, key, manifest.as_bytes(), None, None)
        .context("Could not sign the update manifest")?;
    fs::write(dist.join(MANIFEST), &manifest)?;
    fs::write(dist.join(SIGNATURE), signature.into_string())?;
    Ok(())
}

fn describe(dist: &Path, version: &Version, now: SystemTime) -> anyhow::Result<Manifest> {
    let mut assets = BTreeMap::new();
    for entry in fs::read_dir(dist).with_context(|| format!("Could not read {}", dist.display()))? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|name| anyhow!("{} is not a UTF-8 file name", name.to_string_lossy()))?;
        if NOT_ARCHIVES.contains(&name.as_str()) {
            continue;
        }
        let triple = triple(&name, version)?;
        let bytes = fs::read(entry.path())?;
        let asset = Asset {
            url: format!("{RELEASES}download/v{version}/{name}"),
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            size: u64::try_from(bytes.len())?,
        };
        ensure!(
            assets.insert(triple.clone(), asset).is_none(),
            "{} holds two archives for {triple}",
            dist.display()
        );
    }
    ensure!(!assets.is_empty(), "{} holds no archives", dist.display());
    Ok(Manifest {
        version: version.to_string(),
        notes_url: format!("{RELEASES}tag/v{version}"),
        published_at: timestamp(now)?,
        assets,
    })
}

/// The target triple of `skillbinder-<version>-<triple>.<extension>`, of the Windows
/// `skillbinder-<version>-<triple>-setup.exe`, or of the Linux
/// `skillbinder-<version>-<arch>.AppImage`, which is `<arch>-unknown-linux-gnu`. A triple has at
/// least three parts and an architecture none, so another version's prerelease suffix is never
/// mistaken for one.
fn triple(name: &str, version: &Version) -> anyhow::Result<String> {
    name.strip_prefix(&format!("skillbinder-{version}-"))
        .and_then(|rest| match rest.strip_suffix(".AppImage") {
            Some(arch) => (!arch.contains(['-', '.'])).then(|| format!("{arch}-unknown-linux-gnu")),
            None => rest
                .split_once('.')
                .map(|(triple, _)| triple.strip_suffix("-setup").unwrap_or(triple))
                .filter(|triple| triple.split('-').count() >= 3)
                .map(str::to_owned),
        })
        .with_context(|| format!("{name} is not a skillbinder-{version}-<target> archive"))
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "Test assertions report fixture failures."
    )]
    use super::{MANIFEST, SIGNATURE, open, write};
    use crate::version::Version;
    use minisign::KeyPair;
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    use std::{
        fs,
        path::Path,
        time::{Duration, SystemTime},
    };

    const VERSION: &str = "0.2.0-beta.7";
    /// 2026-11-19T03:12:44Z.
    const NOW: u64 = 1_795_057_964;

    fn dist(archives: &[(&str, &[u8])]) -> tempfile::TempDir {
        let dist = tempfile::tempdir().expect("temporary directory");
        for (name, bytes) in archives {
            fs::write(dist.path().join(name), bytes).expect("fixture archive");
        }
        fs::write(dist.path().join("SHA256SUMS"), "sums\n").expect("fixture sums");
        dist
    }

    fn sign(dist: &Path, key: &KeyPair) -> anyhow::Result<()> {
        let version: Version = VERSION.parse()?;
        write(
            dist,
            &version,
            SystemTime::UNIX_EPOCH + Duration::from_secs(NOW),
            &key.sk,
        )
    }

    #[test]
    fn describes_every_archive_and_signs_the_written_bytes() -> anyhow::Result<()> {
        let archives: [(&str, &[u8]); 3] = [
            (
                "skillbinder-0.2.0-beta.7-universal-apple-darwin.dmg",
                b"mac",
            ),
            (
                "skillbinder-0.2.0-beta.7-x86_64-pc-windows-msvc-setup.exe",
                b"windows!",
            ),
            (
                "skillbinder-0.2.0-beta.7-x86_64.AppImage",
                b"linux appimage",
            ),
        ];
        let dist = dist(&archives);
        let password = "correct horse".to_owned();
        let pair = KeyPair::generate_encrypted_keypair(Some(password.clone()))?;
        // The workflow passes the key file's text and its password, as `minisign -G` writes them.
        let key_file = pair.sk.to_box(None)?.into_string();
        let key = KeyPair {
            pk: pair.pk.clone(),
            sk: open(&key_file, password)?,
        };
        sign(dist.path(), &key)?;

        let bytes = fs::read(dist.path().join(MANIFEST))?;
        let releases = "https://github.com/mastro993/SkillBinder/releases";
        let asset = |name: &str, content: &[u8]| {
            json!({
                "url": format!("{releases}/download/v{VERSION}/{name}"),
                "sha256": format!("{:x}", Sha256::digest(content)),
                "size": content.len(),
            })
        };
        let expected = json!({
            "version": VERSION,
            "notes_url": format!("{releases}/tag/v{VERSION}"),
            "published_at": "2026-11-19T03:12:44Z",
            "assets": {
                "universal-apple-darwin": asset(archives[0].0, archives[0].1),
                "x86_64-pc-windows-msvc": asset(archives[1].0, archives[1].1),
                "x86_64-unknown-linux-gnu": asset(archives[2].0, archives[2].1),
            },
        });
        assert_eq!(serde_json::from_slice::<Value>(&bytes)?, expected);

        let public = minisign_verify::PublicKey::from_base64(&pair.pk.to_base64())?;
        let signature =
            minisign_verify::Signature::decode(&fs::read_to_string(dist.path().join(SIGNATURE))?)?;
        public.verify(&bytes, &signature, false)?;
        let mut tampered = bytes.clone();
        tampered[0] = b' ';
        assert!(public.verify(&tampered, &signature, false).is_err());
        Ok(())
    }

    #[test]
    fn refuses_a_wrong_password() -> anyhow::Result<()> {
        let pair = KeyPair::generate_encrypted_keypair(Some("right".to_owned()))?;
        let key_file = pair.sk.to_box(None)?.into_string();
        assert!(open(&key_file, "wrong".to_owned()).is_err());
        Ok(())
    }

    #[test]
    fn refuses_foreign_duplicate_or_missing_archives() -> anyhow::Result<()> {
        let key = KeyPair::generate_unencrypted_keypair()?;
        for archives in [
            &[][..],
            &[("skillbinder-0.2.0-x86_64.AppImage", &b"stable"[..])],
            &[("skillbinder-0.2.0-beta.6-x86_64.AppImage", &b"older"[..])],
            &[(
                "skillbinder-0.2.0-beta.6-x86_64-pc-windows-msvc-setup.exe",
                &b"older"[..],
            )],
            &[("notes.txt", &b"stray"[..])],
            &[
                (
                    "skillbinder-0.2.0-beta.7-x86_64-unknown-linux-gnu.tar.gz",
                    &b"tar"[..],
                ),
                ("skillbinder-0.2.0-beta.7-x86_64.AppImage", &b"image"[..]),
            ],
        ] {
            let dist = dist(archives);
            assert!(sign(dist.path(), &key).is_err(), "{archives:?}");
            assert!(!dist.path().join(MANIFEST).exists(), "{archives:?}");
        }
        Ok(())
    }
}
