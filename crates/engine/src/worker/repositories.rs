//! The engine's repositories for Explore: listing runs on the Git pool, never the worker, and
//! only saving its result runs on the worker; Install all installs one skill after another.

use super::{Engine, action::Action};
use crate::{
    catalog::{Listed, Listing},
    install::Failed,
    logging::{Area, Level},
    shutdown::Shutdown,
    storage,
};
use skillbinder_proto::*;
use std::time::{Duration, Instant};

impl Engine {
    /// Lists repository `url`'s skills without saving anything, as the Add repository dialog
    /// does before Add.
    pub async fn list_repository(
        &self,
        url: String,
    ) -> Result<RepositoryListing, RepositoryProblem> {
        let subject = vec![("url", url.trim().to_owned())];
        let listed = self.action(Action::ListRepository, subject, |engine| async move {
            problem(engine.list(url).await.map(|(_, listed)| RepositoryListing {
                slugs: listed.into_iter().map(|skill| skill.slug).collect(),
            }))
        });
        settled(listed.await)
    }

    /// Lists repository `url` and saves it with its skills. An invalid URL, an unreachable
    /// repository, one with no skills, and one added already save nothing.
    pub async fn add_repository(&self, url: String) -> Result<Repository, RepositoryProblem> {
        let subject = vec![("url", url.trim().to_owned())];
        let added = self.action(Action::AddRepository, subject, |engine| async move {
            let (url, listed) = match engine.list(url).await {
                Ok(listing) => listing,
                Err(listing) => return problem(Err(listing)),
            };
            engine
                .call(move |state| Ok(state.save_repository(&url, &listed)))
                .await
                .and_then(problem)
        });
        settled(added.await)
    }

    /// Forgets repository `id`. Skills installed from it stay in the library.
    pub async fn remove_repository(&self, id: RepositoryId) -> AppResult<()> {
        let subject = vec![("repository", id.to_string())];
        self.action(Action::RemoveRepository, subject, |engine| async move {
            engine.call(move |state| state.remove_repository(&id)).await
        })
        .await
    }

    /// Saves a removed repository again with its last listing, as Undo does, with no network.
    pub async fn restore_repository(&self, repository: Repository) -> AppResult<()> {
        let subject = vec![("url", repository.url.clone())];
        self.action(Action::RestoreRepository, subject, |engine| async move {
            engine
                .call(move |state| state.restore_repository(&repository))
                .await
        })
        .await
    }

    /// Removes the source caches nothing uses, as closing a removed repository's Undo does.
    pub async fn prune_source_caches(&self) -> AppResult<()> {
        self.action(Action::PruneSourceCaches, Vec::new(), |engine| async move {
            engine.call(|state| state.prune_source_caches()).await
        })
        .await
    }

    /// Lists again each repository last listed `max_age` ago or longer, on the Git pool, unless
    /// a listing of them runs already. One that cannot be listed keeps its last listing.
    pub async fn refresh_repositories(&self, max_age: Duration) -> AppResult<()> {
        self.action(
            Action::RefreshRepositories,
            Vec::new(),
            |engine| async move { engine.relist(max_age).await },
        )
        .await
    }

    /// Lists again the repositories due, as [`Self::refresh_repositories`] does, and writes one
    /// summary when any was due.
    async fn relist(&self, max_age: Duration) -> AppResult<()> {
        let Some(_refreshing) = self.owner.repositories.begin_refresh() else {
            return Ok(());
        };
        let started = Instant::now();
        let due = self
            .call(move |state| state.due_listings(max_age.as_secs()))
            .await?;
        if due.is_empty() {
            return Ok(());
        }
        let mut running = Vec::new();
        for (id, listing) in due {
            running.push((id, self.owner.pool.run(move || listing.run())?));
        }
        let (count, mut failed) = (running.len(), 0);
        for (id, listed) in running {
            match listed.await {
                Ok(Ok(listed)) => self.call(move |state| state.relisted(&id, &listed)).await?,
                _ => failed += 1,
            }
        }
        self.log
            .event(Level::Info, Area::Explore, "repositories refresh finished")
            .field("count", count)
            .field("failed", failed)
            .took(started);
        Ok(())
    }

    /// Installs, one after another, each skill of repository `id` whose slug the library does not
    /// hold; `installed` runs after each one lands. Its progress shows in
    /// [`Self::current_explore`] while it runs.
    pub async fn install_all(
        &self,
        id: RepositoryId,
        installed: impl Fn() + Send + 'static,
    ) -> AppResult<InstallAllOutcome> {
        let subject = vec![("repository", id.to_string())];
        self.action(Action::InstallAll, subject, |engine| async move {
            engine.install_each(id, installed).await
        })
        .await
    }

    /// Installs each skill of repository `id` the library lacks, as [`Self::install_all`] does.
    async fn install_each(
        &self,
        id: RepositoryId,
        installed: impl Fn() + Send,
    ) -> AppResult<InstallAllOutcome> {
        let skills = self
            .call({
                let id = id.clone();
                move |state| {
                    if state.git.is_none() {
                        return Err(AppError::validation("Installing needs Git."));
                    }
                    state.not_in_library(&id)
                }
            })
            .await?;
        let progress = self
            .owner
            .repositories
            .begin_install_all(&id, skills.len())
            .ok_or_else(|| {
                AppError::new(
                    ErrorCategory::Busy,
                    "This repository's skills are installing.",
                    "Wait for Install all to end.",
                )
            })?;
        let mut outcome = InstallAllOutcome::default();
        for skill in skills {
            let slug = skill.slug.clone();
            // A skill installing on its own already ends that way.
            if let Some(begun) = self.owner.installs.begin(install_key(&skill.source))? {
                match self.install_begun(skill, begun).await {
                    Ok(()) => {
                        outcome.installed.push(SkillId(slug));
                        installed();
                    }
                    // The library came to hold the slug meanwhile, as a Pull can.
                    Err(Failed::Held) => {}
                    Err(Failed::Cancelled) => return Err(Shutdown::error()),
                    Err(Failed::Failure(failure, error)) => {
                        self.log_install_failure(&slug, &error);
                        outcome.failed.push((slug, failure));
                    }
                }
            }
            progress.advance();
        }
        Ok(outcome)
    }

    /// Lists repository `url`, trimmed, on the Git pool: its URL and its skills, at least one.
    async fn list(&self, url: String) -> Result<(String, Vec<Listed>), RepositoryProblem> {
        let url = url.trim().to_owned();
        let listing: Listing = self
            .call({
                let url = url.clone();
                move |state| Ok(state.prepare_listing(&url))
            })
            .await??;
        let listed = self
            .owner
            .pool
            .run(move || listing.run())?
            .await
            .map_err(storage::error("waiting for the repository listing"))??;
        if listed.is_empty() {
            return Err(RepositoryProblem::NoSkills);
        }
        Ok((url, listed))
    }
}

/// A repository's problem as its action's outcome: a failure is the action's error, and the
/// problems the dialog shows are what the action found.
fn problem<T>(result: Result<T, RepositoryProblem>) -> AppResult<Result<T, RepositoryProblem>> {
    match result {
        Err(RepositoryProblem::Failed(error)) => Err(error),
        found => Ok(found),
    }
}

/// An action's outcome as the problem it hands to the dialog.
fn settled<T>(outcome: AppResult<Result<T, RepositoryProblem>>) -> Result<T, RepositoryProblem> {
    outcome.unwrap_or_else(|error| Err(error.into()))
}
