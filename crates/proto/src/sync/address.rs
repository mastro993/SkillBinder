//! How a remote reads on the Sync page, and the web page of one whose host has one (Y3, Y4).

use super::RemoteConfig;

/// Hosts that show a repository at `https://host/path`, whatever URL the remote uses.
const WEB_HOSTS: [&str; 3] = ["github.com", "gitlab.com", "bitbucket.org"];

impl RemoteConfig {
    /// The remote as `host/path` without `.git`; a remote without a host, such as a file URL,
    /// reads as its path.
    pub fn address(&self) -> String {
        match parts(&self.url) {
            Parts {
                host: Some(host),
                path,
                ..
            } => format!("{host}/{}", repository(path)),
            Parts { path, .. } => path.to_owned(),
        }
    }

    /// `https://host/path` for a remote on a host with a web view: github.com, gitlab.com,
    /// bitbucket.org, or any HTTPS remote.
    pub fn web_url(&self) -> Option<String> {
        let Parts { https, host, path } = parts(&self.url);
        let host = host?;
        let known = WEB_HOSTS.iter().any(|web| host.eq_ignore_ascii_case(web));
        (https || known).then(|| format!("https://{host}/{}", repository(path)))
    }
}

/// A remote URL's scheme, host, and path.
struct Parts<'a> {
    https: bool,
    host: Option<&'a str>,
    path: &'a str,
}

/// Splits an HTTPS, SSH, or file URL, or an SCP-style `user@host:path`, the forms a remote takes.
fn parts(url: &str) -> Parts<'_> {
    if let Some((scheme, rest)) = url.split_once("://") {
        let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
        let host = authority.rsplit('@').next().unwrap_or(authority);
        let host = host.split(':').next().unwrap_or(host);
        return if host.is_empty() {
            Parts {
                https: false,
                host: None,
                path: &rest[authority.len()..],
            }
        } else {
            Parts {
                https: scheme.eq_ignore_ascii_case("https"),
                host: Some(host),
                path,
            }
        };
    }
    match url.split_once(':') {
        Some((host, path)) if !host.contains('/') => Parts {
            https: false,
            host: Some(host.rsplit('@').next().unwrap_or(host)),
            path,
        },
        _ => Parts {
            https: false,
            host: None,
            path: url,
        },
    }
}

/// A host's repository path without its slashes at either end or `.git`.
fn repository(path: &str) -> &str {
    let path = path.trim_matches('/');
    path.strip_suffix(".git").unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::RemoteConfig;

    fn remote(url: &str) -> RemoteConfig {
        RemoteConfig {
            url: url.to_owned(),
            branch: "main".to_owned(),
        }
    }

    #[test]
    fn a_remote_reads_as_host_and_path_without_git() {
        for (url, address) in [
            ("git@github.com:ada/skills.git", "github.com/ada/skills"),
            ("https://gitlab.com/ada/skills.git", "gitlab.com/ada/skills"),
            (
                "ssh://git@example.com:2222/ada/skills",
                "example.com/ada/skills",
            ),
            (
                "https://git.example.com/team/skills/",
                "git.example.com/team/skills",
            ),
            ("file:///srv/git/skills.git", "/srv/git/skills.git"),
        ] {
            assert_eq!(remote(url).address(), address, "{url}");
        }
    }

    #[test]
    fn hosts_with_a_web_view_open_in_the_browser() {
        for (url, web) in [
            (
                "git@github.com:ada/skills.git",
                Some("https://github.com/ada/skills"),
            ),
            (
                "ssh://git@gitlab.com/ada/skills.git",
                Some("https://gitlab.com/ada/skills"),
            ),
            (
                "git@bitbucket.org:ada/skills.git",
                Some("https://bitbucket.org/ada/skills"),
            ),
            (
                "https://git.example.com/team/skills.git",
                Some("https://git.example.com/team/skills"),
            ),
            ("git@git.example.com:team/skills.git", None),
            ("ssh://git@example.com:2222/ada/skills", None),
            ("file:///srv/git/skills.git", None),
        ] {
            assert_eq!(remote(url).web_url().as_deref(), web, "{url}");
        }
    }
}
