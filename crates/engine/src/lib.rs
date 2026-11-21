//! Embedded operations. A single worker owns SQLite and serializes durable mutations.

mod activation;
mod agents;
mod app_update;
mod catalog;
mod database;
mod folders;
mod git;
mod git_pool;
mod grants;
mod imports;
mod install;
mod library;
mod library_store;
mod lifecycle;
mod logging;
mod onboarding;
mod payload;
mod persistence;
mod reconcile;
mod records;
mod scan;
mod schema;
mod shutdown;
mod sources;
mod worker;
mod workspaces;

pub use app_update::{AppPlacement, Relaunch, UpdatePayload, UpdateSource};
pub use catalog::CatalogSource;
pub use lifecycle::{EngineConfig, GitProfile, GitSearch, RepositoryUrls};
pub use worker::Engine;
