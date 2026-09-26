use crate::{
    git::{GitEnvironment, GitError, VerifiedGit},
    paths::AppPaths,
};
use serde::{Deserialize, Serialize};
use skillbinder_core::git_sync::{SyncFacts, SyncState, classify};
use std::{ffi::OsString, fs, sync::Mutex};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteConfig {
    remote: String,
    branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSyncStatus {
    pub state: SyncState,
    pub remote: Option<String>,
    pub branch: Option<String>,
    pub local_revision: Option<String>,
    pub remote_revision: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub has_local_changes: bool,
}

#[derive(Debug, Error)]
pub enum GitSyncError {
    #[error("remote must be an HTTPS, SSH, or local Git URL")]
    InvalidRemote,
    #[error("branch name is invalid")]
    InvalidBranch,
    #[error("Git sync is not configured")]
    NotConfigured,
    #[error("configured remote is unavailable: {0}")]
    RemoteUnavailable(String),
    #[error(
        "local and remote histories diverged; pull or push cannot choose which changes to keep"
    )]
    Diverged,
    #[error("Git operation failed: {0}")]
    Git(#[from] GitError),
    #[error("Git sync settings could not be read or written: {0}")]
    Storage(String),
}

pub struct GitSyncService {
    paths: AppPaths,
    git: GitEnvironment,
    operation: Mutex<()>,
}

impl GitSyncService {
    pub fn new(paths: AppPaths) -> Self {
        Self {
            paths,
            git: GitEnvironment::default(),
            operation: Mutex::new(()),
        }
    }

    pub fn status(&self) -> Result<GitSyncStatus, GitSyncError> {
        let _guard = self
            .operation
            .lock()
            .map_err(|_| GitSyncError::Storage("Git sync lock is poisoned".into()))?;
        self.status_locked()
    }

    pub fn connect(&self, remote: &str, branch: &str) -> Result<GitSyncStatus, GitSyncError> {
        validate_remote(remote)?;
        validate_branch(branch)?;
        let _guard = self
            .operation
            .lock()
            .map_err(|_| GitSyncError::Storage("Git sync lock is poisoned".into()))?;
        let git = self.git.verify()?;
        self.ensure_origin(&git, remote)?;
        self.write_config(&RemoteConfig {
            remote: remote.trim().to_owned(),
            branch: branch.trim().to_owned(),
        })?;
        self.status_locked()
    }

    pub fn refresh(&self) -> Result<GitSyncStatus, GitSyncError> {
        self.status()
    }

    pub fn pull(&self) -> Result<GitSyncStatus, GitSyncError> {
        let _guard = self
            .operation
            .lock()
            .map_err(|_| GitSyncError::Storage("Git sync lock is poisoned".into()))?;
        let config = self.config()?.ok_or(GitSyncError::NotConfigured)?;
        let git = self.git.verify()?;
        let status = self.status_locked_with(&git, &config)?;
        if status.has_local_changes {
            return Err(GitSyncError::Diverged);
        }
        if status.state != SyncState::NeedsPull {
            return Ok(status);
        }
        let reference = remote_reference(&config.branch);
        self.check_managed_diff(&git, &reference)?;
        self.run(&git, &["merge", "--ff-only", &reference])?;
        self.status_locked()
    }

    pub fn push(&self) -> Result<GitSyncStatus, GitSyncError> {
        let _guard = self
            .operation
            .lock()
            .map_err(|_| GitSyncError::Storage("Git sync lock is poisoned".into()))?;
        let config = self.config()?.ok_or(GitSyncError::NotConfigured)?;
        let git = self.git.verify()?;
        let status = self.status_locked_with(&git, &config)?;
        if status.state == SyncState::NeedsPull || status.state == SyncState::NeedsSync {
            return Err(if status.state == SyncState::NeedsSync {
                GitSyncError::Diverged
            } else {
                GitSyncError::RemoteUnavailable(
                    "remote has changes that must be pulled first".into(),
                )
            });
        }
        self.commit_managed_changes(&git)?;
        self.run(
            &git,
            &[
                "push",
                "origin",
                &format!("HEAD:refs/heads/{}", config.branch),
            ],
        )?;
        self.status_locked()
    }

    pub fn sync(&self) -> Result<GitSyncStatus, GitSyncError> {
        let _guard = self
            .operation
            .lock()
            .map_err(|_| GitSyncError::Storage("Git sync lock is poisoned".into()))?;
        let config = self.config()?.ok_or(GitSyncError::NotConfigured)?;
        let git = self.git.verify()?;
        let mut status = self.status_locked_with(&git, &config)?;
        if status.state == SyncState::NeedsSync {
            return Err(GitSyncError::Diverged);
        }
        if status.state == SyncState::NeedsPull {
            let reference = remote_reference(&config.branch);
            self.check_managed_diff(&git, &reference)?;
            self.run(&git, &["merge", "--ff-only", &reference])?;
            status = self.status_locked_with(&git, &config)?;
        }
        if status.state == SyncState::NeedsPush {
            self.commit_managed_changes(&git)?;
            self.run(
                &git,
                &[
                    "push",
                    "origin",
                    &format!("HEAD:refs/heads/{}", config.branch),
                ],
            )?;
        }
        self.status_locked()
    }

    pub fn disconnect(&self) -> Result<GitSyncStatus, GitSyncError> {
        let _guard = self
            .operation
            .lock()
            .map_err(|_| GitSyncError::Storage("Git sync lock is poisoned".into()))?;
        let _ = fs::remove_file(self.paths.git_sync_config());
        if self.paths.library().is_dir()
            && let Ok(git) = self.git.verify()
        {
            let _ = self.run(&git, &["remote", "remove", "origin"]);
        }
        self.status_locked()
    }

    fn status_locked(&self) -> Result<GitSyncStatus, GitSyncError> {
        let Some(config) = self.config()? else {
            return Ok(GitSyncStatus {
                state: SyncState::NotConfigured,
                remote: None,
                branch: None,
                local_revision: None,
                remote_revision: None,
                ahead: 0,
                behind: 0,
                has_local_changes: false,
            });
        };
        let git = self.git.verify()?;
        self.status_locked_with(&git, &config)
    }

    fn status_locked_with(
        &self,
        git: &VerifiedGit,
        config: &RemoteConfig,
    ) -> Result<GitSyncStatus, GitSyncError> {
        self.ensure_origin(git, &config.remote)?;
        let local_revision = self.output(git, &["rev-parse", "HEAD"])?;
        let has_local_changes = !self
            .output(
                git,
                &[
                    "status",
                    "--porcelain",
                    "--untracked-files=all",
                    "--",
                    "skills",
                    ".skillbinder.json",
                ],
            )?
            .is_empty();
        let remote_revision = self
            .output(git, &["ls-remote", "--heads", "origin", &config.branch])?
            .lines()
            .find_map(|line| line.split_whitespace().next().map(str::to_owned));
        if remote_revision.is_none() {
            let facts = SyncFacts {
                local_revision: Some(local_revision),
                remote_revision: None,
                has_local_changes,
                ahead: 0,
                behind: 0,
            };
            return Ok(self.status_from(config, facts));
        }
        self.run(git, &["fetch", "--no-tags", "origin", &config.branch])?;
        let tracking = remote_reference(&config.branch);
        let remote_revision = self.output(git, &["rev-parse", &tracking])?;
        let counts = self.output(
            git,
            &[
                "rev-list",
                "--left-right",
                "--count",
                &format!("HEAD...{tracking}"),
            ],
        )?;
        let mut values = counts.split_whitespace();
        let ahead = parse_count(values.next())?;
        let behind = parse_count(values.next())?;
        Ok(self.status_from(
            config,
            SyncFacts {
                local_revision: Some(local_revision),
                remote_revision: Some(remote_revision),
                has_local_changes,
                ahead,
                behind,
            },
        ))
    }

    fn status_from(&self, config: &RemoteConfig, facts: SyncFacts) -> GitSyncStatus {
        GitSyncStatus {
            state: classify(&facts, true),
            remote: Some(config.remote.clone()),
            branch: Some(config.branch.clone()),
            local_revision: facts.local_revision,
            remote_revision: facts.remote_revision,
            ahead: facts.ahead,
            behind: facts.behind,
            has_local_changes: facts.has_local_changes,
        }
    }

    fn config(&self) -> Result<Option<RemoteConfig>, GitSyncError> {
        let path = self.paths.git_sync_config();
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|error| GitSyncError::Storage(error.to_string())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(GitSyncError::Storage(error.to_string())),
        }
    }

    fn write_config(&self, config: &RemoteConfig) -> Result<(), GitSyncError> {
        let path = self.paths.git_sync_config();
        let bytes = format!(
            "{}\n",
            serde_json::to_string_pretty(config)
                .map_err(|error| GitSyncError::Storage(error.to_string()))?
        );
        fs::write(path, bytes).map_err(|error| GitSyncError::Storage(error.to_string()))
    }

    fn ensure_origin(&self, git: &VerifiedGit, remote: &str) -> Result<(), GitSyncError> {
        match self.output(git, &["remote", "get-url", "origin"]) {
            Ok(current) if current == remote => Ok(()),
            Ok(_) => {
                self.run(git, &["remote", "set-url", "origin", remote])?;
                Ok(())
            }
            Err(GitSyncError::Git(GitError::ProcessFailed(_))) => {
                self.run(git, &["remote", "add", "origin", remote])?;
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    fn commit_managed_changes(&self, git: &VerifiedGit) -> Result<(), GitSyncError> {
        let staged = self.output(git, &["diff", "--cached", "--name-only"])?;
        if staged.lines().any(|path| !is_managed_path(path)) {
            return Err(GitSyncError::Storage(
                "unrelated staged changes block a SkillBinder sync".into(),
            ));
        }
        self.run(git, &["add", "--", "skills", ".skillbinder.json"])?;
        let staged = self.output(git, &["diff", "--cached", "--name-only"])?;
        if staged.trim().is_empty() {
            return Ok(());
        }
        self.run(
            git,
            &[
                "-c",
                "user.name=SkillBinder",
                "-c",
                "user.email=local@skillbinder.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--no-verify",
                "-m",
                "Sync SkillBinder library",
            ],
        )?;
        Ok(())
    }

    fn check_managed_diff(&self, git: &VerifiedGit, reference: &str) -> Result<(), GitSyncError> {
        let changed = self.output(git, &["diff", "--name-only", &format!("HEAD..{reference}")])?;
        if changed.lines().any(|path| !is_managed_path(path)) {
            return Err(GitSyncError::RemoteUnavailable(
                "remote contains files outside the SkillBinder library".into(),
            ));
        }
        let metadata = self.output(git, &["show", &format!("{reference}:.skillbinder.json")])?;
        serde_json::from_str::<crate::portable_metadata::PortableMetadata>(&metadata).map_err(
            |error| GitSyncError::RemoteUnavailable(format!("remote metadata is invalid: {error}")),
        )?;
        Ok(())
    }

    fn run(
        &self,
        git: &VerifiedGit,
        operation: &[&str],
    ) -> Result<std::process::Output, GitSyncError> {
        let hooks = format!(
            "core.hooksPath={}",
            self.paths.config.join("git-hooks").display()
        );
        let mut args = vec![
            OsString::from("-c"),
            OsString::from(hooks),
            OsString::from("-c"),
            OsString::from("core.autocrlf=false"),
            OsString::from("-C"),
            self.paths.library().into_os_string(),
        ];
        args.extend(operation.iter().map(OsString::from));
        self.git.run(git, args).map_err(Into::into)
    }

    fn output(&self, git: &VerifiedGit, operation: &[&str]) -> Result<String, GitSyncError> {
        let output = self.run(git, operation)?;
        String::from_utf8(output.stdout)
            .map(|value| value.trim().to_owned())
            .map_err(|_| GitSyncError::Storage("Git returned non-UTF-8 output".into()))
    }
}

fn validate_remote(remote: &str) -> Result<(), GitSyncError> {
    let remote = remote.trim();
    let allowed = remote.starts_with("https://")
        || remote.starts_with("ssh://")
        || remote.starts_with("git@")
        || remote.starts_with("file://");
    if remote.is_empty()
        || !allowed
        || remote.starts_with('-')
        || remote
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
        || (remote.starts_with("https://") && remote[8..].contains('@'))
    {
        return Err(GitSyncError::InvalidRemote);
    }
    Ok(())
}

fn validate_branch(branch: &str) -> Result<(), GitSyncError> {
    let branch = branch.trim();
    if branch.is_empty()
        || branch.starts_with('-')
        || branch.starts_with('/')
        || branch.ends_with('/')
        || branch == "."
        || branch == ".."
        || branch.ends_with('.')
        || branch.contains("..")
        || branch.contains("//")
        || branch.contains("/.")
        || branch.contains("./")
        || branch.contains("@{")
        || branch.contains('@')
        || branch.chars().any(|character| {
            character.is_control() || character.is_whitespace() || ":?*[\\^~".contains(character)
        })
    {
        return Err(GitSyncError::InvalidBranch);
    }
    Ok(())
}

fn remote_reference(branch: &str) -> String {
    format!("refs/remotes/origin/{branch}")
}

fn parse_count(value: Option<&str>) -> Result<u32, GitSyncError> {
    value
        .ok_or_else(|| GitSyncError::Storage("Git returned invalid ahead/behind counts".into()))?
        .parse()
        .map_err(|_| GitSyncError::Storage("Git returned invalid ahead/behind counts".into()))
}

fn is_managed_path(path: &str) -> bool {
    path == ".skillbinder.json" || path == "skills" || path.starts_with("skills/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_validation_rejects_credentials_and_options() {
        assert!(validate_remote("https://user:secret@example.com/skills.git").is_err());
        assert!(validate_remote("--upload-pack=evil").is_err());
        assert!(validate_remote("git@example.com:team/skills.git").is_ok());
        assert!(validate_remote("ssh://git@example.com/team/skills.git").is_ok());
    }

    #[test]
    fn managed_path_check_is_narrow() {
        assert!(is_managed_path("skills/review/SKILL.md"));
        assert!(is_managed_path(".skillbinder.json"));
        assert!(!is_managed_path("README.md"));
        assert!(!is_managed_path(".git/config"));
    }

    #[test]
    fn pushes_then_fast_forward_pulls_managed_library_changes() {
        let root = std::env::temp_dir().join(format!(
            "skillbinder-git-sync-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let library = root.join("data/library");
        let remote = root.join("remote.git");
        let peer = root.join("peer");
        std::fs::create_dir_all(library.join("skills/review")).unwrap();
        std::fs::create_dir_all(&remote).unwrap();
        std::fs::write(
            library.join(".skillbinder.json"),
            "{\"schemaVersion\":1,\"libraryId\":\"library\",\"createdAt\":\"now\",\"contentPolicyVersion\":1,\"skills\":{}}\n",
        )
        .unwrap();
        std::fs::write(library.join("skills/review/SKILL.md"), "local").unwrap();
        run_git(&library, &["init", "--initial-branch=main"]);
        run_git(&library, &["config", "user.name", "Test"]);
        run_git(&library, &["config", "user.email", "test@example.invalid"]);
        run_git(&library, &["add", "."]);
        run_git(&library, &["commit", "-m", "initial"]);
        run_git(&remote, &["init", "--bare", "--initial-branch=main"]);

        let paths = AppPaths::new(root.join("data"), root.join("config"), root.join("cache"));
        paths.create_base_directories().unwrap();
        let service = GitSyncService::new(paths);
        let remote_url = format!("file://{}", remote.display());
        assert_eq!(
            service.connect(&remote_url, "main").unwrap().state,
            SyncState::NeedsPush
        );
        assert_eq!(service.push().unwrap().state, SyncState::Synced);

        run_git_at(&root, &["clone", &remote_url, "peer"]);
        run_git(&peer, &["config", "user.name", "Peer"]);
        run_git(&peer, &["config", "user.email", "peer@example.invalid"]);
        std::fs::write(peer.join("skills/review/SKILL.md"), "remote").unwrap();
        run_git(&peer, &["add", "skills/review/SKILL.md"]);
        run_git(&peer, &["commit", "-m", "remote change"]);
        run_git(&peer, &["push", "origin", "main"]);

        assert_eq!(service.status().unwrap().state, SyncState::NeedsPull);
        assert_eq!(service.pull().unwrap().state, SyncState::Synced);
        assert_eq!(
            std::fs::read_to_string(library.join("skills/review/SKILL.md")).unwrap(),
            "remote"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    fn run_git(directory: &std::path::Path, args: &[&str]) {
        run_git_at(directory, args);
    }

    fn run_git_at(directory: &std::path::Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .current_dir(directory)
            .args(args)
            .status()
            .unwrap();
        assert!(
            status.success(),
            "git command failed: git {}",
            args.join(" ")
        );
    }
}
