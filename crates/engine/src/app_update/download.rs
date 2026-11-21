//! Downloads an app update's archive (spec rules 5, 6): streamed to a file while it is hashed,
//! and kept only when its size and SHA-256 match the signed manifest's.

use super::manifest::Asset;
use crate::catalog::CatalogSource;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
    time::Duration,
};

/// The largest archive downloaded.
pub(super) const LIMIT: u64 = 512 * 1024 * 1024;
/// How long the connection may wait for the host or for the next bytes; a download as a whole
/// may take as long as it needs.
const STALL: Duration = Duration::from_secs(30);

/// Downloads `asset` from `site` into `file`, reporting each new whole percent to `progress`.
/// A failed, oversized, or mismatched download removes the file and says why.
pub(super) fn download(
    site: &CatalogSource,
    asset: &Asset,
    file: &Path,
    progress: impl FnMut(u8),
) -> Result<(), String> {
    if asset.size > LIMIT {
        return Err(format!("{} bytes is over the limit", asset.size));
    }
    let fetched = open(site, &asset.url).and_then(|body| {
        let mut out = File::create(file).map_err(|error| error.to_string())?;
        let digest = copy(body, &mut out, asset.size, progress)?;
        out.sync_all().map_err(|error| error.to_string())?;
        if digest.eq_ignore_ascii_case(&asset.sha256) {
            Ok(())
        } else {
            Err("its SHA-256 does not match the manifest".to_owned())
        }
    });
    if fetched.is_err() {
        let _ = fs::remove_file(file);
    }
    fetched
}

/// The archive's bytes: from the site's host, or for a fixture site, from its directory.
fn open(site: &CatalogSource, url: &str) -> Result<Box<dyn Read>, String> {
    match site {
        CatalogSource::Http(base) => {
            let url = base.join(url).map_err(|error| error.to_string())?;
            let agent = ureq::Agent::config_builder()
                .timeout_global(None)
                .timeout_connect(Some(STALL))
                .timeout_recv_response(Some(STALL))
                .timeout_per_read(Some(STALL))
                .user_agent(concat!("SkillBinder/", env!("CARGO_PKG_VERSION")))
                .build()
                .new_agent();
            let response = agent
                .get(url.as_str())
                .call()
                .map_err(|error| error.to_string())?;
            Ok(Box::new(response.into_body().into_reader()))
        }
        CatalogSource::Directory(root) => File::open(root.join(url))
            .map(|file| Box::new(file) as Box<dyn Read>)
            .map_err(|error| error.to_string()),
    }
}

/// Copies at most `size` bytes of `body` into `out`, refusing any other length, and returns their
/// SHA-256 in hex.
fn copy(
    body: impl Read,
    out: &mut impl Write,
    size: u64,
    mut progress: impl FnMut(u8),
) -> Result<String, String> {
    let mut body = body.take(size + 1);
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 64 * 1024];
    let (mut read, mut reported) = (0_u64, None);
    loop {
        let count = match body.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.to_string()),
        };
        let chunk = buffer.get(..count).unwrap_or_default();
        hash.update(chunk);
        out.write_all(chunk).map_err(|error| error.to_string())?;
        read += count as u64;
        let percent = u8::try_from(read.min(size) * 100 / size.max(1)).unwrap_or(100);
        if reported != Some(percent) {
            reported = Some(percent);
            progress(percent);
        }
    }
    if read != size {
        return Err(format!("it has {read} bytes, not {size}"));
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "Test fixture failures must abort the test."
    )]
    use super::*;

    fn asset(bytes: &[u8], url: &str) -> Asset {
        Asset {
            url: url.to_owned(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
            size: bytes.len() as u64,
        }
    }

    #[test]
    fn only_the_signed_bytes_are_kept() {
        let temp = tempfile::tempdir().unwrap();
        let site = CatalogSource::Directory(temp.path().to_path_buf());
        fs::write(temp.path().join("app.dmg"), b"the app").unwrap();
        let file = temp.path().join("downloaded.dmg");
        let mut seen = Vec::new();
        download(&site, &asset(b"the app", "app.dmg"), &file, |p| {
            seen.push(p)
        })
        .unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"the app");
        assert_eq!(seen.last(), Some(&100));

        for wrong in [
            asset(b"another app", "app.dmg"),
            Asset {
                size: 3,
                ..asset(b"the app", "app.dmg")
            },
            Asset {
                size: LIMIT + 1,
                ..asset(b"the app", "app.dmg")
            },
            asset(b"the app", "missing.dmg"),
        ] {
            assert!(download(&site, &wrong, &file, |_| {}).is_err(), "{wrong:?}");
            assert!(!file.exists(), "{wrong:?} is discarded");
        }
    }
}
