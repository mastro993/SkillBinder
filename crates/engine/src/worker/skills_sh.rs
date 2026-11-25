//! The engine's skills.sh search and its results' folders: the search runs on a thread of its
//! own, and a result's repository is fetched and searched on the Git pool. Installing a result
//! resolves its folder first, then installs the skill found.

use super::Engine;
use crate::catalog::{Begin, Resolve, Resolved, not_found};
use crate::install::Failed;
use crate::scan::registry::workspace_skill_dirs;
use crate::sources::{CacheUser, cache_dir};
use crate::storage;
use skillbinder_proto::*;
use std::future::Future;

impl Engine {
    /// Asks skills.sh for `query` in place of any earlier query; a query under two characters
    /// clears the search. It shows as searching in [`Self::current_explore`] from this call on;
    /// the future ends once the search settles.
    pub fn search_skills_sh(
        &self,
        query: &str,
    ) -> impl Future<Output = AppResult<()>> + Send + 'static {
        let asked = self.owner.searches.search(query);
        async move {
            if let Some(settled) = asked? {
                settled
                    .await
                    .map_err(storage::error("waiting for the skills.sh search"))?;
            }
            Ok(())
        }
    }

    /// Asks skills.sh for `query` without changing the search Explore shows. A query under two
    /// characters asks nothing and answers [`SkillsShState::Idle`].
    pub fn ask_skills_sh(
        &self,
        query: &str,
    ) -> impl Future<Output = AppResult<SkillsShState>> + Send + 'static {
        let asked = self.owner.searches.ask(query);
        async move {
            match asked? {
                Some(answered) => answered
                    .await
                    .map_err(storage::error("waiting for the skills.sh answer")),
                None => Ok(SkillsShState::Idle),
            }
        }
    }

    /// Finds `result`'s folder in its repository, once per session. It shows as resolving in
    /// [`Self::current_explore`] from this call on; the future answers with the skill found,
    /// `None` when no folder matches, or why the repository could not be read.
    pub fn resolve(
        &self,
        result: SkillsShResult,
    ) -> impl Future<Output = AppResult<Option<CatalogSkill>>> + Send + 'static {
        let begun = self.owner.resolutions.begin(&result);
        let engine = self.clone();
        async move {
            match begun {
                Begin::Known(resolved) => resolved,
                Begin::Wait(found) => found.await.unwrap_or_else(|_| Err(AppError::storage())),
                Begin::Start => {
                    let resolved = engine.find_folder(&result).await;
                    engine.owner.resolutions.finish(&result, &resolved);
                    resolved
                }
            }
        }
    }

    /// Installs `result`: resolves its folder, then installs the skill found, inactive. It shows
    /// as installing from this call on; a failure to read the repository stays shown as the
    /// install's, and a result with no folder shows as not found.
    pub fn install_skills_sh(
        &self,
        result: SkillsShResult,
    ) -> impl Future<Output = AppResult<()>> + Send + 'static {
        let begun = self.owner.installs.begin(result_key(&result));
        let resolve = self.resolve(result.clone());
        let engine = self.clone();
        async move {
            let begun = begun?.ok_or_else(|| {
                AppError::new(
                    ErrorCategory::Busy,
                    "This skill is installing.",
                    "Wait for its install to end.",
                )
            })?;
            match resolve.await {
                Ok(Some(skill)) => {
                    // The skill shows installing under its own source before the result stops.
                    let install = engine.install_skill(skill);
                    drop(begun);
                    install.await
                }
                Ok(None) => {
                    drop(begun);
                    Err(not_found(&result))
                }
                Err(error) => {
                    let failed = Err(Failed::from(error.clone()));
                    begun.end(&failed);
                    Err(error)
                }
            }
        }
    }

    async fn find_folder(&self, result: &SkillsShResult) -> Resolved {
        let source = SkillSource {
            url: result.url.clone(),
            reference: None,
            path: String::new(),
        };
        let previews = self.owner.previews.clone();
        let resolve = self
            .call(move |state| {
                let git = state.git.clone().ok_or_else(|| {
                    AppError::new(
                        ErrorCategory::Validation,
                        "Finding this skill needs Git.",
                        "Install Git, then try again.",
                    )
                })?;
                Ok(Resolve {
                    git,
                    cache: cache_dir(&state.config.data_dir, CacheUser::Catalog, &source),
                    previews,
                    agent_dirs: workspace_skill_dirs()?.into_iter().collect(),
                    log: state.log.clone(),
                })
            })
            .await?;
        let result = result.clone();
        self.owner
            .pool
            .run(move || resolve.run(&result))?
            .await
            .map_err(storage::error("waiting for the folder resolution"))?
    }
}
