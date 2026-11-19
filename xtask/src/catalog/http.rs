//! HTTPS GETs: the published site, skills.sh's search, and GitHub avatars. Tests replace the
//! network with saved responses.

use anyhow::{Context as _, bail};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(30);

/// What a GET found.
pub(super) enum Fetched {
    Found(Vec<u8>),
    /// HTTP 404.
    Missing,
}

pub(super) trait Http {
    /// GETs `url`, refusing a body over `limit` bytes. A 404 is `Missing`; any other answer
    /// but 200 is an error.
    fn get(&self, url: &str, limit: u64) -> anyhow::Result<Fetched>;
}

pub(super) struct Web(ureq::Agent);

impl Web {
    pub(super) fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .https_only(true)
            .http_status_as_error(false)
            .timeout_global(Some(TIMEOUT))
            .user_agent("SkillBinder-catalog")
            .build();
        Self(config.into())
    }
}

impl Http for Web {
    fn get(&self, url: &str, limit: u64) -> anyhow::Result<Fetched> {
        let mut response = self
            .0
            .get(url)
            .call()
            .with_context(|| format!("GET {url}"))?;
        match response.status().as_u16() {
            200 => Ok(Fetched::Found(
                response
                    .body_mut()
                    .with_config()
                    .limit(limit)
                    .read_to_vec()
                    .with_context(|| format!("GET {url}"))?,
            )),
            404 => Ok(Fetched::Missing),
            status => bail!("GET {url} answered HTTP {status}"),
        }
    }
}
