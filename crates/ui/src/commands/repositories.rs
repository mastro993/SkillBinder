//! Your repositories from a terminal: `repos`, `repo add`, `repo remove`, and `repo install-all`,
//! as Explore's Your repositories, Add repository, Remove, and Install all run them.

use super::explore::needs_git;
use super::plan::Plan;
use super::resolve;
use super::run::on_runtime;
use crate::native_view::{failures, install_all_message, installable};
use skillbinder_client::Client;
use skillbinder_proto::command_line::{
    Changed, CommandError, ErrorCode, Failure, Output, RepositoryRow,
};
use skillbinder_proto::{
    AppError, AppSnapshot, Repository, RepositoryProblem, normalized_url, owner_repo,
};
/// `repos`: your repositories, as Explore lists them.
pub(super) fn repositories(snapshot: &AppSnapshot) -> Vec<RepositoryRow> {
    snapshot
        .explore
        .repositories
        .iter()
        .map(|repository| RepositoryRow {
            name: owner_repo(&repository.url),
            url: repository.url.clone(),
            skills: repository.skills.len(),
            installable: installable(snapshot, repository),
        })
        .collect()
}

/// The repository `name` names, by its URL, however it is spelled, or its `<owner>/<repo>`.
pub(super) fn repository<'a>(
    snapshot: &'a AppSnapshot,
    name: &str,
) -> Result<&'a Repository, CommandError> {
    let typed = if name.contains("://") || name.contains('@') {
        normalized_url(name)
    } else {
        name.to_owned()
    };
    resolve::one(
        "repository",
        &typed,
        snapshot.explore.repositories.iter(),
        |repository| owner_repo(&repository.url),
        |repository| {
            vec![
                repository.url.clone(),
                normalized_url(&repository.url),
                owner_repo(&repository.url),
            ]
        },
    )
}

/// `repo add`, which lists the repository before adding it.
pub(super) fn plan_add(snapshot: &AppSnapshot, url: &str) -> Result<Plan, CommandError> {
    needs_git(snapshot, "Adding a repository")?;
    Ok(Plan::AddRepository {
        url: url.to_owned(),
    })
}

/// Adds the repository at `url` as the Add repository dialog does.
pub(super) async fn add_repository(url: String, client: &Client) -> Result<Changed, CommandError> {
    let adding = client.clone();
    let added = client
        .runtime()
        .spawn(async move { adding.add_repository(url).await })
        .await
        .unwrap_or_else(|_| Err(RepositoryProblem::Failed(AppError::storage())))
        .map_err(problem)?;
    let skills = match added.skills.len() {
        1 => "1 skill".to_owned(),
        count => format!("{count} skills"),
    };
    Ok(Changed::said(format!(
        "Added {}: {skills}.",
        owner_repo(&added.url)
    )))
}

/// A repository the Add repository dialog refuses, in its words.
pub(super) fn problem(problem: RepositoryProblem) -> CommandError {
    match problem {
        RepositoryProblem::NotGitUrl => CommandError::new(ErrorCode::Usage, "Not a Git URL.")
            .hint("Use an HTTPS, SSH, or scp-style Git URL without credentials."),
        RepositoryProblem::Unreachable => CommandError::new(
            ErrorCode::Failed,
            "Couldn't reach this repository. Check the URL and your Git access.",
        ),
        RepositoryProblem::NoSkills => CommandError::new(ErrorCode::Failed, "No skills found."),
        RepositoryProblem::AlreadyAdded => {
            CommandError::new(ErrorCode::Conflict, "This repository is already added.")
        }
        RepositoryProblem::NeedsGit => {
            CommandError::new(ErrorCode::GitMissing, "Adding a repository needs Git.")
                .hint("Install Git, then try again.")
        }
        RepositoryProblem::Failed(error) => error.into(),
    }
}

/// Forgets `repository`, as its Remove does; skills installed from it stay.
pub(super) async fn remove_repository(
    repository: Repository,
    client: &Client,
) -> Result<Changed, CommandError> {
    let id = repository.id.clone();
    on_runtime(client, move |client| async move {
        client.remove_repository(id).await
    })
    .await?;
    Ok(Changed::said(format!(
        "Removed {}. Installed skills stay in your library.",
        owner_repo(&repository.url)
    )))
}

/// `repo install-all`: nothing to do once the library holds every one of its slugs, as Explore
/// then offers no Install all.
pub(super) fn plan_install_all(snapshot: &AppSnapshot, name: &str) -> Result<Plan, CommandError> {
    let repository = repository(snapshot, name)?;
    needs_git(snapshot, "Installing")?;
    if installable(snapshot, repository) == 0 {
        return Ok(Plan::Save {
            saves: Vec::new(),
            done: format!(
                "Every skill of {} is in your library.",
                owner_repo(&repository.url)
            ),
        });
    }
    Ok(Plan::InstallAll(repository.clone()))
}

/// Installs `repository`'s skills the library lacks, one after another, as Install all does. A
/// skill that fails leaves the others installed, which come back beside the failures.
pub(super) async fn install_all(
    repository: Repository,
    client: &Client,
) -> Result<Changed, Failure> {
    let id = repository.id.clone();
    let outcome = on_runtime(
        client,
        move |client| async move { client.install_all(id).await },
    )
    .await?;
    let changed = Changed {
        message: install_all_message(&outcome),
        installed: outcome.installed.iter().map(ToString::to_string).collect(),
        ..Changed::default()
    };
    if outcome.failed.is_empty() {
        return Ok(changed);
    }
    let failed = match outcome.failed.len() {
        1 => "1 skill couldn't be installed".to_owned(),
        count => format!("{count} skills couldn't be installed"),
    };
    Err(Failure {
        error: CommandError::new(ErrorCode::Failed, failed).hint(failures(&repository, &outcome)),
        result: Some(Box::new(Output::Changed(changed))),
    })
}

#[cfg(test)]
mod tests;
