//! The engine's previews for Explore's detail dialog: Git is looked up on the worker, and the
//! source is fetched and read on the Git pool, so a slow fetch never holds up other commands.

use super::{Engine, action::Action};
use crate::catalog::{Read, Request};
use crate::sources::{CacheUser, cache_dir};
use skillbinder_proto::*;
use std::future::Future;

impl Engine {
    /// Opens `skill`'s preview in place of any other. It shows as loading in
    /// [`Self::current_explore`] from this call on, before the returned future first runs; the
    /// future ends once the preview is loaded or failed.
    pub fn open_preview(&self, skill: &CatalogSkill) -> impl Future<Output = ()> + Send + 'static {
        self.load_preview(Some(self.owner.previews.open(skill)))
    }

    /// Loads the open preview again, as Retry does; nothing happens when no dialog is open.
    pub fn retry_preview(&self) -> impl Future<Output = ()> + Send + 'static {
        self.load_preview(self.owner.previews.reopen())
    }

    /// Closes the preview, as closing the dialog does; its cache may be pruned from then on.
    pub fn close_preview(&self) {
        self.owner.previews.close();
    }

    fn load_preview(&self, request: Option<Request>) -> impl Future<Output = ()> + Send + 'static {
        let subject = request
            .iter()
            .map(|request| ("url", request.source.url.clone()))
            .collect();
        let loaded = self.action(Action::Preview, subject, |engine| async move {
            if let Some(request) = request {
                let state = engine.read_preview(&request).await;
                engine.owner.previews.finish(&request, state);
            }
            Ok(())
        });
        async move {
            let _ = loaded.await;
        }
    }

    async fn read_preview(&self, request: &Request) -> PreviewState {
        let source = request.source.clone();
        let previews = self.owner.previews.clone();
        let read = self
            .call(move |state| {
                Ok(state.git.clone().map(|git| Read {
                    git,
                    cache: cache_dir(&state.config.data_dir, CacheUser::Catalog, &source),
                    previews,
                    log: state.log.clone(),
                }))
            })
            .await;
        let read = match read {
            Ok(Some(read)) => read,
            Ok(None) => return PreviewState::NeedsGit,
            Err(_) => return PreviewState::Failed,
        };
        let request = Request::clone(request);
        match self.owner.pool.run(move || read.run(&request)) {
            Ok(result) => result.await.unwrap_or(PreviewState::Failed),
            Err(_) => PreviewState::Failed,
        }
    }
}
