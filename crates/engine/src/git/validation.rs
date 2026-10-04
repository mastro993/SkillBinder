use super::{AppError, AppResult, RemoteConfig, failure};
use skillbinder_proto::Library;

pub(super) fn version(raw: &str) -> AppResult<String> {
    let value = raw
        .strip_prefix("git version ")
        .and_then(|s| s.split_whitespace().next())
        .ok_or_else(|| failure("Git version is unreadable."))?;
    let mut parts = value.split('.');
    let major: u32 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| failure("Git version is unreadable."))?;
    let minor: u32 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| failure("Git version is unreadable."))?;
    if (major, minor) < (2, 39) {
        return Err(failure("Git 2.39 or newer is required."));
    }
    Ok(value.to_owned())
}
pub(super) fn remote(remote: &RemoteConfig) -> AppResult<()> {
    let invalid = || {
        AppError::validation("Use an HTTPS, SSH, SCP-style SSH, or file remote and a valid branch.")
    };
    if [&remote.url, &remote.branch].iter().any(|s| {
        s.is_empty() || s.starts_with('-') || s.chars().any(|c| c.is_control() || c.is_whitespace())
    }) || remote.branch.starts_with('@')
    {
        return Err(invalid());
    }
    if remote.url.contains("://") {
        let parsed = url::Url::parse(&remote.url).map_err(|_| invalid())?;
        if parsed.host_str().is_some_and(|host| host.starts_with('-'))
            || parsed.username().starts_with('-')
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || parsed.password().is_some()
        {
            return Err(invalid());
        }
        match parsed.scheme() {
            "https" if parsed.host_str().is_some() && parsed.username().is_empty() => {}
            "ssh" if parsed.host_str().is_some() => {}
            "file" if parsed.to_file_path().is_ok() && parsed.username().is_empty() => {}
            _ => return Err(invalid()),
        }
    } else {
        let (host, path) = remote.url.split_once(':').ok_or_else(invalid)?;
        let host = host.rsplit('@').next().ok_or_else(invalid)?;
        if host.is_empty()
            || host.starts_with('-')
            || host.contains('/')
            || path.is_empty()
            || path.starts_with('-')
            || !host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ".-_".contains(c))
        {
            return Err(invalid());
        }
    }
    Ok(())
}
pub(super) fn metadata(bytes: &[u8]) -> AppResult<()> {
    let library: Library = serde_json::from_slice(bytes)
        .map_err(|_| failure("Remote or local library metadata is invalid."))?;
    crate::library::validate_library(&library)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn versions() {
        assert!(version("git version 2.38.9").is_err());
        assert_eq!(
            version("git version 2.39.0.windows.1").unwrap(),
            "2.39.0.windows.1"
        );
        assert!(version("garbage").is_err());
    }
    #[test]
    fn unsafe_remotes_are_rejected() {
        for url in [
            "--upload-pack=x",
            "https://u:p@host/repo",
            "https://u@host/repo",
            "https://host/repo?token=secret",
            "ssh://-oProxyCommand=x/repo",
            "git@-host:repo",
            "file:///tmp/a\nb",
        ] {
            assert!(
                remote(&RemoteConfig {
                    url: url.into(),
                    branch: "main".into()
                })
                .is_err(),
                "{url}"
            );
        }
    }
}
