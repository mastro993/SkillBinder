//! The engine's install from Explore: refused or prepared on the worker, fetched on the Git pool,
//! and committed on the worker again, so commits never interleave; and clearing the New mark it
//! leaves.

use super::Engine;
use crate::install::{Begun, Failed, Fetch};
use skillbinder_proto::*;
use std::future::Future;

impl Engine {
    /// Installs `skill` from its source into the library, inactive. Its state shows as
    /// installing in [`Self::current_explore`] from this call on, before the returned future
    /// first runs, and a failure stays there until the skill is installed again. Several installs
    /// run at once; installing a skill that is installing already is refused as busy.
    pub fn install_skill(
        &self,
        skill: CatalogSkill,
    ) -> impl Future<Output = AppResult<()>> + Send + 'static {
        let begun = self.owner.installs.begin(install_key(&skill.source));
        let engine = self.clone();
        async move {
            let begun = begun?.ok_or_else(|| {
                AppError::new(
                    ErrorCategory::Busy,
                    "This skill is installing.",
                    "Wait for its install to end.",
                )
            })?;
            let slug = skill.slug.clone();
            engine
                .install_begun(skill, begun)
                .await
                .map_err(|failed| failed.error(&slug))
        }
    }

    /// Runs the install `begun` of `skill` to its end, keeping a failure for Explore to show.
    pub(super) async fn install_begun(
        &self,
        skill: CatalogSkill,
        begun: Begun,
    ) -> Result<(), Failed> {
        let installed = self.install(skill, &begun).await;
        begun.end(&installed);
        installed
    }

    /// Clears skill `slug`'s New mark in the database alone: nothing is scanned, activated, or
    /// committed, so Sync has no change to report. A slug the library does not hold, or one that
    /// is not New, is left as it is.
    pub async fn mark_seen(&self, slug: SkillId) -> AppResult<()> {
        self.call(move |state| {
            crate::library_store::mark_seen(&mut state.database.borrow_mut(), slug.as_str())
        })
        .await
    }

    async fn install(&self, skill: CatalogSkill, begun: &Begun) -> Result<(), Failed> {
        let id = begun.id;
        let prepared = self
            .call(move |state| {
                Ok(state.prepare_install(&skill).map(|git| {
                    let data_dir = &state.config.data_dir;
                    Fetch {
                        git,
                        staging: data_dir.join("staging").join(format!("install-{id}")),
                        cache: crate::sources::install_cache(data_dir, id),
                        // A trailing `/` names the same folder, as `same_source` compares them.
                        source: SkillSource {
                            url: skill.source.url,
                            reference: None,
                            path: skill.source.path.trim_matches('/').to_owned(),
                        },
                        slug: skill.slug,
                        shutdown: state.shutdown.clone(),
                        log: state.log.clone(),
                    }
                }))
            })
            .await?;
        let fetch = prepared?;
        let (slug, source, staging) = (
            fetch.slug.clone(),
            fetch.source.clone(),
            fetch.staging.clone(),
        );
        let staged = self
            .owner
            .pool
            .run(move || fetch.run())?
            .await
            .map_err(|_| Failed::from(AppError::storage()))??;
        let committed = self
            .call({
                let staging = staging.clone();
                move |state| Ok(state.commit_install(slug, source, staging, &staged))
            })
            .await;
        if committed.is_err() {
            // The worker ended before the commit ran, as on quit, so nothing else removes it.
            let _ = std::fs::remove_dir_all(&staging);
        }
        committed?
    }
}
