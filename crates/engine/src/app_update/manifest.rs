//! A channel's `update.json` (ADR 0018): verified with minisign against the compiled-in public
//! key, then read for the version it offers this build and the archive that installs it.

use serde::{Deserialize, Serialize};
use skillbinder_proto::AppVersion;
use std::collections::BTreeMap;

/// The asset key of the archive this build installs from; `None` where no archive is published.
pub(super) const TARGET: Option<&str> = if cfg!(target_os = "macos") {
    Some("universal-apple-darwin")
} else if cfg!(all(windows, target_arch = "x86_64")) {
    Some("x86_64-pc-windows-msvc")
} else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
    Some("x86_64-unknown-linux-gnu")
} else {
    None
};

/// The fields of `update.json` a check reads; unknown fields are ignored.
#[derive(Deserialize)]
struct Manifest {
    version: AppVersion,
    notes_url: String,
    assets: BTreeMap<String, Asset>,
}

/// One platform's archive: where it downloads from, its SHA-256 in hex, and its size in bytes.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub(super) struct Asset {
    /// Absolute, or relative to the update site.
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

/// A newer version and its release page.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct Offer {
    pub version: AppVersion,
    pub notes_url: String,
}

/// Why a manifest was refused.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum Refusal {
    /// The signature does not verify under the public key, or the key or signature is malformed.
    Signature,
    /// It verified but does not read as an `update.json`.
    Unreadable,
}

/// A newer version, with the archive for this platform.
#[derive(Clone, Debug)]
pub(super) struct Found {
    pub offer: Offer,
    pub asset: Asset,
}

/// The version `manifest` offers a build of `current` for `target`, once its `signature` verifies
/// under `public_key`: only a newer version with an archive for `target`.
pub(super) fn offer(
    manifest: &[u8],
    signature: &[u8],
    public_key: &str,
    current: AppVersion,
    target: Option<&str>,
) -> Result<Option<Found>, Refusal> {
    let key = minisign_verify::PublicKey::from_base64(public_key.trim())
        .map_err(|_| Refusal::Signature)?;
    let signature = std::str::from_utf8(signature)
        .ok()
        .and_then(|text| minisign_verify::Signature::decode(text).ok())
        .ok_or(Refusal::Signature)?;
    key.verify(manifest, &signature, false)
        .map_err(|_| Refusal::Signature)?;
    let Manifest {
        version,
        notes_url,
        mut assets,
    } = serde_json::from_slice(manifest).map_err(|_| Refusal::Unreadable)?;
    let asset = target.and_then(|target| assets.remove(target));
    Ok(asset.filter(|_| version > current).map(|asset| Found {
        offer: Offer { version, notes_url },
        asset,
    }))
}
