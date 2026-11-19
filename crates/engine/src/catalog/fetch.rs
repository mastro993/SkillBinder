//! Conditional GETs of the catalog's files, over HTTPS or from a fixture directory that answers
//! the same way.

use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf, time::Duration};

/// How long one request may take, from resolving the host to the body's last byte.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Where the catalog is fetched from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CatalogSource {
    /// The CDN's base URL, ending in `/`.
    Http(url::Url),
    /// A directory laid out like the CDN, for tests and native fixtures. A missing directory
    /// answers as an unreachable host, a missing file as a 404, and each file's ETag is its
    /// digest.
    Directory(PathBuf),
}

/// What a source answered for one file.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum Answer {
    /// The file, with its ETag when the source sent one.
    Body {
        bytes: Vec<u8>,
        etag: Option<String>,
    },
    /// The ETag sent still names the file.
    NotModified,
    /// The source has no such file.
    NotFound,
    /// The host could not be reached: no connection, no name, or a timeout.
    Offline,
    /// Any other status, an unreadable answer, or a body over the limit.
    Failed,
}

/// One refresh's connection to a source; HTTP reuses one agent for every file.
pub(super) enum Fetcher {
    Http { base: url::Url, agent: ureq::Agent },
    Directory(PathBuf),
}

impl Fetcher {
    pub(super) fn new(source: &CatalogSource) -> Self {
        match source {
            CatalogSource::Http(base) => {
                let agent = ureq::Agent::config_builder()
                    .timeout_global(Some(TIMEOUT))
                    .http_status_as_error(false)
                    .user_agent(concat!("SkillBinder/", env!("CARGO_PKG_VERSION")))
                    .build()
                    .new_agent();
                Self::Http {
                    base: base.clone(),
                    agent,
                }
            }
            CatalogSource::Directory(root) => Self::Directory(root.clone()),
        }
    }

    /// GETs `path`, relative to the source's base, sending `etag` as `If-None-Match`, and reads at
    /// most `limit` bytes of the body.
    pub(super) fn get(&self, path: &str, etag: Option<&str>, limit: u64) -> Answer {
        match self {
            Self::Http { base, agent } => http(agent, base, path, etag, limit),
            Self::Directory(root) => directory(root, path, etag, limit),
        }
    }
}

fn http(
    agent: &ureq::Agent,
    base: &url::Url,
    path: &str,
    etag: Option<&str>,
    limit: u64,
) -> Answer {
    let Ok(url) = base.join(path) else {
        return Answer::Failed;
    };
    let mut request = agent.get(url.as_str());
    if let Some(etag) = etag {
        request = request.header("If-None-Match", etag);
    }
    let mut response = match request.call() {
        Ok(response) => response,
        Err(error) => return failure(&error),
    };
    match response.status().as_u16() {
        200 => {}
        304 => return Answer::NotModified,
        404 => return Answer::NotFound,
        _ => return Answer::Failed,
    }
    let etag = response
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    match response.body_mut().with_config().limit(limit).read_to_vec() {
        Ok(bytes) => Answer::Body { bytes, etag },
        Err(error) => failure(&error),
    }
}

/// An unreachable host is offline; anything else that went wrong is a failure.
fn failure(error: &ureq::Error) -> Answer {
    match error {
        ureq::Error::HostNotFound
        | ureq::Error::ConnectionFailed
        | ureq::Error::Timeout(_)
        | ureq::Error::Io(_) => Answer::Offline,
        _ => Answer::Failed,
    }
}

fn directory(root: &std::path::Path, path: &str, etag: Option<&str>, limit: u64) -> Answer {
    if !root.is_dir() {
        return Answer::Offline;
    }
    let Ok(file) = fs::File::open(root.join(path)) else {
        return Answer::NotFound;
    };
    let mut bytes = Vec::new();
    if file.take(limit + 1).read_to_end(&mut bytes).is_err() || bytes.len() as u64 > limit {
        return Answer::Failed;
    }
    let digest = format!("\"{:x}\"", Sha256::digest(&bytes));
    if etag == Some(digest.as_str()) {
        return Answer::NotModified;
    }
    Answer::Body {
        bytes,
        etag: Some(digest),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn a_directory_answers_like_the_cdn() {
        let temp = tempfile::tempdir().unwrap();
        let missing = Fetcher::new(&CatalogSource::Directory(temp.path().join("gone")));
        assert_eq!(missing.get("v1/catalog.json", None, 10), Answer::Offline);
        let fetcher = Fetcher::new(&CatalogSource::Directory(temp.path().to_path_buf()));
        assert_eq!(fetcher.get("v1/catalog.json", None, 10), Answer::NotFound);
        fs::create_dir(temp.path().join("v1")).unwrap();
        fs::write(temp.path().join("v1/catalog.json"), b"{}").unwrap();
        let Answer::Body { bytes, etag } = fetcher.get("v1/catalog.json", None, 10) else {
            panic!("expected the file");
        };
        assert_eq!(bytes, b"{}");
        assert_eq!(
            fetcher.get("v1/catalog.json", etag.as_deref(), 10),
            Answer::NotModified
        );
        assert_eq!(fetcher.get("v1/catalog.json", None, 1), Answer::Failed);
    }
}
