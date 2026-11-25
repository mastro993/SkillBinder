//! Repository rows in machine-local SQLite, and the worker's side of each repository operation:
//! what a listing needs before it leaves for the Git pool, and saving what it found.

use super::{Listed, Listing, accepts};
use crate::logging::{Area, Level};
use crate::{
    git::Git,
    library::normalized,
    persistence::now,
    scan::registry::workspace_skill_dirs,
    schema::repositories,
    sources::{CacheUser, cache_dir},
    storage,
    worker::State,
};
use diesel::prelude::*;
use skillbinder_proto::{
    AppError, AppResult, CatalogSkill, Repository, RepositoryId, RepositoryProblem, SkillSource,
    normalized_url,
};

/// A stored time as Unix seconds; a negative one, which no version writes, as the epoch.
fn seconds(stored: i64) -> u64 {
    u64::try_from(stored).unwrap_or_default()
}

fn stored(seconds: u64) -> i64 {
    i64::try_from(seconds).unwrap_or(i64::MAX)
}

impl State {
    /// The saved repositories in the order they were added, each with its last listing.
    fn load_repositories(&self) -> AppResult<Vec<Repository>> {
        let rows = repositories::table
            .select((
                repositories::id,
                repositories::url,
                repositories::added_at,
                repositories::listing,
                repositories::listed_at,
            ))
            .order((repositories::added_at, repositories::id))
            .load::<(String, String, i64, String, i64)>(&mut *self.database.borrow_mut())
            .map_err(storage::error("loading repositories"))?;
        Ok(rows
            .into_iter()
            .map(|(id, url, added_at, listing, listed_at)| {
                let id = RepositoryId(id);
                // A listing this version cannot read is kept on disk and shows as empty until the
                // repository is listed again.
                let listed: Vec<Listed> = serde_json::from_str(&listing).unwrap_or_default();
                Repository {
                    skills: listed.iter().map(|skill| skill.skill(&id, &url)).collect(),
                    id,
                    url,
                    listed_at: seconds(listed_at),
                    added_at: seconds(added_at),
                }
            })
            .collect())
    }

    /// Reads the saved repositories into what snapshots and the client show.
    pub(crate) fn publish_repositories(&self) -> AppResult<()> {
        self.repositories.publish(self.load_repositories()?);
        Ok(())
    }

    /// The listing of `url` as the user entered it, refused before any network use when Git is
    /// missing, the URL is not one a repository can have, or its repository is added already.
    pub(crate) fn prepare_listing(&self, url: &str) -> Result<Listing, RepositoryProblem> {
        let git = self.git.clone().ok_or(RepositoryProblem::NeedsGit)?;
        if !accepts(url, self.config.repository_urls) {
            return Err(RepositoryProblem::NotGitUrl);
        }
        self.refuse_added(url)?;
        Ok(self.listing(&git, url, workspace_skill_dirs()?.into_iter().collect()))
    }

    /// The listing of `url` with `git`, searching `agent_dirs` beside the known containers.
    fn listing(&self, git: &Git, url: &str, agent_dirs: Vec<String>) -> Listing {
        let source = SkillSource {
            url: url.to_owned(),
            reference: None,
            path: String::new(),
        };
        Listing {
            git: git.clone(),
            url: url.to_owned(),
            // Keyed by the normalized URL, so previews of its skills share the cache.
            cache: cache_dir(&self.config.data_dir, CacheUser::Catalog, &source),
            previews: self.previews.clone(),
            agent_dirs,
            log: self.log.clone(),
        }
    }

    fn refuse_added(&self, url: &str) -> Result<(), RepositoryProblem> {
        let added = repositories::table
            .filter(repositories::normalized.eq(normalized_url(url)))
            .count()
            .get_result::<i64>(&mut *self.database.borrow_mut())
            .map_err(storage::error("counting repositories with a URL"))?;
        if added > 0 {
            Err(RepositoryProblem::AlreadyAdded)
        } else {
            Ok(())
        }
    }

    /// Saves repository `url` with the skills a listing just found, unless one with its
    /// normalized URL was added while it listed.
    pub(crate) fn save_repository(
        &self,
        url: &str,
        listed: &[Listed],
    ) -> Result<Repository, RepositoryProblem> {
        self.refuse_added(url)?;
        let id = RepositoryId::new();
        let at = now();
        self.insert_repository(&id, url, at, listed, at)?;
        self.log
            .event(Level::Info, Area::Explore, "repository added")
            .field("url", url);
        self.publish_repositories()?;
        Ok(Repository {
            skills: listed.iter().map(|skill| skill.skill(&id, url)).collect(),
            id,
            url: url.to_owned(),
            listed_at: at,
            added_at: at,
        })
    }

    fn insert_repository(
        &self,
        id: &RepositoryId,
        url: &str,
        added_at: u64,
        listed: &[Listed],
        listed_at: u64,
    ) -> AppResult<()> {
        let listing = serde_json::to_string(listed)
            .map_err(storage::error("serializing a repository listing"))?;
        diesel::insert_into(repositories::table)
            .values((
                repositories::id.eq(id.as_str()),
                repositories::url.eq(url),
                repositories::normalized.eq(normalized_url(url)),
                repositories::added_at.eq(stored(added_at)),
                repositories::listing.eq(listing),
                repositories::listed_at.eq(stored(listed_at)),
            ))
            .execute(&mut *self.database.borrow_mut())
            .map(drop)
            .map_err(storage::error("saving a repository"))
    }

    /// Forgets repository `id`. Skills installed from it stay in the library with their records
    /// and sources; its cache stays until it is pruned.
    pub(crate) fn remove_repository(&self, id: &RepositoryId) -> AppResult<()> {
        diesel::delete(repositories::table.filter(repositories::id.eq(id.as_str())))
            .execute(&mut *self.database.borrow_mut())
            .map_err(storage::error("removing a repository"))?;
        self.publish_repositories()
    }

    /// Saves `repository` again, as Undo does, with its last listing and no network. One whose
    /// normalized URL was added again since stays as it is.
    pub(crate) fn restore_repository(&self, repository: &Repository) -> AppResult<()> {
        if !accepts(&repository.url, self.config.repository_urls) {
            return Err(AppError::validation("This repository's URL is not usable."));
        }
        if self.refuse_added(&repository.url).is_ok() {
            let listed: Vec<Listed> = repository.skills.iter().map(Listed::of).collect();
            self.insert_repository(
                &repository.id,
                &repository.url,
                repository.added_at,
                &listed,
                repository.listed_at,
            )?;
        }
        self.publish_repositories()
    }

    /// The listings of the repositories last listed `max_age` seconds ago or longer; none
    /// without Git, so their last listings stay.
    pub(crate) fn due_listings(&self, max_age: u64) -> AppResult<Vec<(RepositoryId, Listing)>> {
        let Some(git) = &self.git else {
            return Ok(Vec::new());
        };
        let agent_dirs: Vec<String> = workspace_skill_dirs()?.into_iter().collect();
        Ok(self
            .repositories
            .current()
            .0
            .iter()
            .filter(|repository| now().saturating_sub(repository.listed_at) >= max_age)
            .map(|repository| {
                let listing = self.listing(git, &repository.url, agent_dirs.clone());
                (repository.id.clone(), listing)
            })
            .collect())
    }

    /// Saves what listing repository `id` again found, if it is still saved.
    pub(crate) fn relisted(&self, id: &RepositoryId, listed: &[Listed]) -> AppResult<()> {
        let listing = serde_json::to_string(listed)
            .map_err(storage::error("serializing a repository listing"))?;
        diesel::update(repositories::table.filter(repositories::id.eq(id.as_str())))
            .set((
                repositories::listing.eq(listing),
                repositories::listed_at.eq(stored(now())),
            ))
            .execute(&mut *self.database.borrow_mut())
            .map_err(storage::error("saving a repository listing"))?;
        self.publish_repositories()
    }

    /// The skills of repository `id` whose slug the library does not hold, which Install all
    /// installs.
    pub(crate) fn not_in_library(&self, id: &RepositoryId) -> AppResult<Vec<CatalogSkill>> {
        let held = self.held_names()?;
        let repositories = self.repositories.current().0;
        let repository = repositories
            .iter()
            .find(|repository| &repository.id == id)
            .ok_or_else(|| AppError::stale("This repository was removed."))?;
        Ok(repository
            .skills
            .iter()
            .filter(|skill| !held.contains(&normalized(&skill.slug)))
            .cloned()
            .collect())
    }
}
